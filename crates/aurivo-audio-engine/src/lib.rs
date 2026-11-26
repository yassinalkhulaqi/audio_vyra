pub mod audio_path;
pub mod chain;
pub mod channels;
pub mod control;
pub mod latency;
pub mod recorder;
pub mod resampler;
pub mod rt_stats;
pub mod state;

use aurivo_core::{AurivoError, Result, LevelMeter};
use audio_path::{adapt_playback, mono_ring_capacity, prepare_output_frames, StreamFormat};
use chain::EffectChain;
use channels::downmix_into;
use recorder::{RecorderState, read_wav, write_wav};
use resampler::LinearResampler;
use rt_stats::{RtSnapshot, RtStats};
use latency::LatencyTracker;
use control::{ControlBinding, ControlMap, ControlMessage};
use aurivo_dsp::{Gain, LevelMeterProcessor, NoiseGate, Compressor, Biquad, Delay, DelayParams, Reverb, ReverbParams, Distortion, DistortionParams, Modulation, ModulationParams, ParametricEq, EqBand, EqFilterKind, EqParams, PitchShifter, PeakLimiter, SpectrumAnalyzer, Vocal, VocalParams};
use aurivo_dsp::eq as eq_ops;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{HeapRb, traits::{Producer, Consumer, Split}};
use serde::{Deserialize, Serialize};
pub use state::AppState;
use std::sync::{Arc, Mutex, atomic::{AtomicU32, AtomicU64, AtomicBool, Ordering}};
use std::collections::BTreeMap;
use aurivo_dsp::{AutomationBank, AutomationRoute, LfoShape};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBandInfo {
    pub kind: i32,
    pub frequency: f32,
    pub q: f32,
    pub gain_db: f32,
    pub enabled: bool,
}

pub struct AudioEngine {
    host: cpal::Host,
    input_stream: Option<cpal::Stream>,
    output_stream: Option<cpal::Stream>,
    chain: Arc<Mutex<EffectChain>>,
    meter: Arc<LevelMeterProcessor>,
    gain: Arc<AtomicU32>,
    delay: Arc<Mutex<DelayParams>>,
    reverb: Arc<Mutex<ReverbParams>>,
    distortion: Arc<Mutex<DistortionParams>>,
    modulation: Arc<Mutex<ModulationParams>>,
    eq: Arc<Mutex<EqParams>>,
    pitch: Arc<AtomicU32>,
    analyzer: Arc<Mutex<SpectrumAnalyzer>>,
    spectrum: Arc<Mutex<Vec<f32>>>,
    output_gain: Arc<AtomicU32>,
    output_muted: Arc<AtomicBool>,
    recorder: Arc<Mutex<RecorderState>>,
    overruns: Arc<AtomicU64>,
    underruns: Arc<AtomicU64>,
    resampler: Arc<Mutex<LinearResampler>>,
    rt_stats: Arc<RtStats>,
    latency: Arc<LatencyTracker>,
    buffered_samples: Arc<AtomicU32>,
    automation: Arc<Mutex<AutomationBank>>,
    automation_base: Arc<Mutex<BTreeMap<String, f32>>>,
    control_map: Arc<Mutex<ControlMap>>,
    playback_stream: Option<cpal::Stream>,
    vocal: Arc<Mutex<VocalParams>>,
    running: bool,
    sample_rate: u32,
    output_sample_rate: u32,
    channels: u16,
    input_channels: u16,
}

impl AudioEngine {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let sr = 48000u32;
        let (gain_proc, gain_shared) = Gain::shared();
        let mut chain = EffectChain::new(sr);
        chain.add("gain", Box::new(gain_proc));
        chain.add("gate", Box::new(NoiseGate::new(-40.0, 5.0, 50.0, sr as f32)));
        chain.add("compressor", Box::new(Compressor::new(-18.0, 3.0, 10.0, 100.0, 2.0, sr as f32)));
        chain.add("highpass", Box::new(Biquad::new_highpass(80.0, 0.707, sr as f32)));
        let (delay_proc, delay_shared) = Delay::shared(sr);
        chain.add("delay", Box::new(delay_proc));
        let (reverb_proc, reverb_shared) = Reverb::shared(sr);
        chain.add("reverb", Box::new(reverb_proc));
        let (distortion_proc, distortion_shared) = Distortion::shared(sr);
        chain.add("distortion", Box::new(distortion_proc));
        let (modulation_proc, modulation_shared) = Modulation::shared(sr);
        chain.add("modulation", Box::new(modulation_proc));
        let (eq_proc, eq_shared) = ParametricEq::shared(sr);
        chain.add("eq", Box::new(eq_proc));
        let (pitch_proc, pitch_shared) = PitchShifter::shared(sr);
        chain.add("pitch", Box::new(pitch_proc));
        let (vocal_proc, vocal_shared) = Vocal::shared(sr);
        chain.add("vocal", Box::new(vocal_proc));
        chain.add("limiter", Box::new(PeakLimiter::new(sr)));
        let meter = Arc::new(LevelMeterProcessor::new());
        let (analyzer, spectrum_handle) = SpectrumAnalyzer::shared(1024);
        let analyzer = Arc::new(Mutex::new(analyzer));
        let output_gain = Arc::new(AtomicU32::new(1.0f32.to_bits()));
        let output_muted = Arc::new(AtomicBool::new(false));
        let recorder = Arc::new(Mutex::new(RecorderState::new(sr, 1)));
        let resampler = Arc::new(Mutex::new(LinearResampler::new(sr, sr)));
        let overruns = Arc::new(AtomicU64::new(0));
        let underruns = Arc::new(AtomicU64::new(0));
        Self {
            host,
            input_stream: None,
            output_stream: None,
            chain: Arc::new(Mutex::new(chain)),
            meter,
            gain: gain_shared,
            delay: delay_shared,
            reverb: reverb_shared,
            distortion: distortion_shared,
            modulation: modulation_shared,
            eq: eq_shared,
            pitch: pitch_shared,
            analyzer,
            spectrum: spectrum_handle,
            output_gain,
            output_muted,
            recorder,
            overruns,
            underruns,
            resampler,
            rt_stats: RtStats::new(),
            latency: LatencyTracker::new(),
            buffered_samples: Arc::new(AtomicU32::new(0)),
            automation: Arc::new(Mutex::new(AutomationBank::new())),
            automation_base: Arc::new(Mutex::new(BTreeMap::new())),
            control_map: Arc::new(Mutex::new(ControlMap::new())),
            playback_stream: None,
            vocal: vocal_shared,
            running: false,
            sample_rate: sr,
            output_sample_rate: sr,
            channels: 1,
            input_channels: 1,
        }
    }

    pub fn list_input_devices(&self) -> Vec<String> {
        self.host.input_devices().map(|d| d.map(|dev| dev.description().map(|d| d.name().to_string()).unwrap_or_default()).collect()).unwrap_or_default()
    }
    pub fn list_output_devices(&self) -> Vec<String> {
        self.host.output_devices().map(|d| d.map(|dev| dev.description().map(|d| d.name().to_string()).unwrap_or_default()).collect()).unwrap_or_default()
    }

    pub fn start(&mut self, input_name: Option<String>, output_name: Option<String>) -> Result<()> {
        if self.running { self.stop(); }
        let input_dev = match input_name {
            Some(n) => self.host.input_devices().ok().and_then(|mut it| it.find(|d| d.description().ok().map(|x| x.name().to_string()) == Some(n.clone()))),
            None => self.host.default_input_device(),
        }.ok_or_else(|| AurivoError::DeviceNotFound("input".into()))?;
        let output_dev = match output_name {
            Some(n) => self.host.output_devices().ok().and_then(|mut it| it.find(|d| d.description().ok().map(|x| x.name().to_string()) == Some(n.clone()))),
            None => self.host.default_output_device(),
        }.ok_or_else(|| AurivoError::DeviceNotFound("output".into()))?;

        let in_cfg = input_dev.default_input_config().map_err(|e| AurivoError::Stream(e.to_string()))?;
        if in_cfg.sample_format() != cpal::SampleFormat::F32 {
            return Err(AurivoError::Stream("input device does not support f32".into()));
        }
        let sr = in_cfg.sample_rate();
        let ch = in_cfg.channels();

        let out_cfg = output_dev.default_output_config().map_err(|e| AurivoError::Stream(e.to_string()))?;
        if out_cfg.sample_format() != cpal::SampleFormat::F32 {
            return Err(AurivoError::Stream("output device does not support f32".into()));
        }
        let out_sr = out_cfg.sample_rate();
        let out_ch = out_cfg.channels();
        let resample = sr != out_sr;
        if let Ok(mut r) = self.resampler.lock() { r.set_rates(sr, out_sr); }

        let ring = HeapRb::<f32>::new(mono_ring_capacity(sr, 4));
        let (mut prod, mut cons) = ring.split();

        let chain = self.chain.clone();
        let meter = self.meter.clone();
        let overruns = self.overruns.clone();
        let underruns = self.underruns.clone();
        let analyzer = self.analyzer.clone();
        let recorder = self.recorder.clone();
        let rt_stats = self.rt_stats.clone();
        let buffered_samples = self.buffered_samples.clone();
        let mut mono_scratch = vec![0.0f32; 256];
        let in_stream = input_dev.build_input_stream(
            in_cfg.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if mono_scratch.len() < data.len() / ch.max(1) as usize + 1 {
                    mono_scratch.resize(data.len() / ch.max(1) as usize + 1, 0.0);
                }
                let n = downmix_into(data, ch, &mut mono_scratch);
                let buf = &mut mono_scratch[..n];
                if let Ok(mut c) = chain.try_lock() {
                    c.process(buf);
                } else {
                    rt_stats.bump_chain();
                }
                meter.process(buf);
                if let Ok(mut a) = analyzer.try_lock() {
                    a.push(buf);
                } else {
                    rt_stats.bump_analyzer();
                }
                match recorder.try_lock() {
                    Ok(mut rec) => rec.push(buf),
                    Err(_) => rt_stats.bump_recorder(),
                }
                let pushed = prod.push_slice(buf);
                buffered_samples.fetch_add(pushed as u32, Ordering::Relaxed);
                if pushed < buf.len() {
                    overruns.fetch_add((buf.len() - pushed) as u64, Ordering::Relaxed);
                }
            },
            |err| eprintln!("input stream error: {err}"),
            None,
        ).map_err(|e| AurivoError::Stream(e.to_string()))?;

        let output_gain = self.output_gain.clone();
        let output_muted = self.output_muted.clone();
        let resampler = self.resampler.clone();
        let rt_stats = self.rt_stats.clone();
        let latency = self.latency.clone();
        let buffered_samples = self.buffered_samples.clone();
        let mut mono_in = vec![0.0f32; 512];
        let mut resampled = vec![0.0f32; 1024];
        let out_stream = output_dev.build_output_stream(
            out_cfg.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let frames = data.len().div_ceil(out_ch.max(1) as usize);
                if mono_in.len() < frames {
                    mono_in.resize(frames, 0.0);
                }
                if resampled.len() < frames + 8 {
                    resampled.resize(frames + 8, 0.0);
                }
                let got = cons.pop_slice(&mut mono_in[..frames]);
                let _ = buffered_samples.try_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                    Some(v.saturating_sub(got as u32))
                });
                latency.report(buffered_samples.load(Ordering::Relaxed) as usize, out_sr);
                let written = match resampler.try_lock() {
                    Ok(mut r) => prepare_output_frames(
                        &mono_in[..got],
                        &mut r,
                        &mut [],
                        &mut resampled,
                        data,
                        resample,
                        out_ch,
                    ),
                    Err(_) => {
                        rt_stats.bump_resampler();
                        0
                    }
                };
                if written < data.len() {
                    underruns.fetch_add((data.len() - written) as u64, Ordering::Relaxed);
                    data[written..].fill(0.0);
                }
                let muted = output_muted.load(Ordering::Relaxed);
                let gain = f32::from_bits(output_gain.load(Ordering::Relaxed));
                for s in data.iter_mut() {
                    if muted { *s = 0.0; } else { *s *= gain; }
                }
            },
            |err| eprintln!("output stream error: {err}"),
            None,
        ).map_err(|e| AurivoError::Stream(e.to_string()))?;

        in_stream.play().map_err(|e| AurivoError::Stream(e.to_string()))?;
        out_stream.play().map_err(|e| AurivoError::Stream(e.to_string()))?;

        self.input_stream = Some(in_stream);
        self.output_stream = Some(out_stream);
        self.running = true;
        self.sample_rate = sr;
        self.output_sample_rate = out_sr;
        self.channels = out_ch;
        self.input_channels = ch;
        if let Ok(mut rec) = self.recorder.lock() {
            rec.sample_rate = sr;
            rec.channels = 1;
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        self.input_stream = None;
        self.output_stream = None;
        self.playback_stream = None;
        self.running = false;
        if let Ok(mut rec) = self.recorder.lock() {
            rec.stop();
        }
    }

    pub fn is_running(&self) -> bool { self.running }
    pub fn audio_stats(&self) -> (u64, u64) {
        (self.overruns.load(Ordering::Relaxed), self.underruns.load(Ordering::Relaxed))
    }
    pub fn reset_audio_stats(&self) {
        self.overruns.store(0, Ordering::Relaxed);
        self.underruns.store(0, Ordering::Relaxed);
        self.rt_stats.reset();
    }

    pub fn rt_stats(&self) -> RtSnapshot {
        self.rt_stats.snapshot()
    }

    pub fn reset_rt_stats(&self) {
        self.rt_stats.reset();
    }

    pub fn buffered_ms(&self) -> u32 {
        self.latency.buffered_ms()
    }

    pub fn peak_buffered_ms(&self) -> u32 {
        self.latency.peak_ms()
    }

    pub fn reset_latency(&self) {
        self.latency.reset();
    }
    pub fn stream_info(&self) -> (u32, u32, u16) {
        (self.sample_rate, self.output_sample_rate, self.channels)
    }

    pub fn input_channels(&self) -> u16 {
        self.input_channels
    }

    pub fn stream_formats(&self) -> (StreamFormat, StreamFormat) {
        (
            StreamFormat::new(self.sample_rate, self.input_channels),
            StreamFormat::new(self.output_sample_rate, self.channels),
        )
    }
    pub fn levels(&self) -> LevelMeter {
        LevelMeter { peak: self.meter.peak(), rms: self.meter.rms(), clipped: self.meter.clipped() }
    }
    pub fn reset_clip(&self) { self.meter.reset_clip(); }
    pub fn set_input_gain_db(&self, db: f32) { self.gain.store(db.to_bits(), Ordering::Relaxed); }

    pub fn input_gain_db(&self) -> f32 { f32::from_bits(self.gain.load(Ordering::Relaxed)) }

    pub fn set_delay(&self, time_ms: f32, feedback: f32, mix: f32, damp: f32) {
        Delay::update(&self.delay, DelayParams { time_ms, feedback, mix, damp });
    }

    pub fn delay_params(&self) -> (f32, f32, f32, f32) {
        let p = Delay::snapshot(&self.delay);
        (p.time_ms, p.feedback, p.mix, p.damp)
    }

    pub fn set_reverb(&self, room_size: f32, damp: f32, wet: f32, dry: f32) {
        Reverb::update(&self.reverb, ReverbParams { room_size, damp, wet, dry });
    }

    pub fn reverb_params(&self) -> (f32, f32, f32, f32) {
        let p = Reverb::snapshot(&self.reverb);
        (p.room_size, p.damp, p.wet, p.dry)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_distortion(&self, mode: i32, drive_db: f32, tone: f32, mix: f32, downsample: f32, bit_depth: f32) {
        Distortion::update(&self.distortion, DistortionParams {
            mode: aurivo_dsp::validate::safe_mode(mode, 2),
            drive_db,
            tone,
            mix,
            downsample,
            bit_depth,
        });
    }

    pub fn distortion_params(&self) -> (i32, f32, f32, f32, f32, f32) {
        let p = Distortion::snapshot(&self.distortion);
        (p.mode, p.drive_db, p.tone, p.mix, p.downsample, p.bit_depth)
    }

    pub fn set_modulation(&self, mode: i32, rate_hz: f32, depth: f32, feedback: f32, mix: f32) {
        Modulation::update(&self.modulation, ModulationParams {
            mode: aurivo_dsp::validate::safe_mode(mode, 3),
            rate_hz,
            depth,
            feedback,
            mix,
        });
    }

    pub fn modulation_params(&self) -> (i32, f32, f32, f32, f32) {
        let p = Modulation::snapshot(&self.modulation);
        (p.mode, p.rate_hz, p.depth, p.feedback, p.mix)
    }

    pub fn set_eq_band(&self, index: usize, kind: i32, frequency: f32, q: f32, gain_db: f32, enabled: bool) {
        eq_ops::set_band(&self.eq, index, EqBand {
            kind: EqFilterKind::from_i32(kind),
            frequency,
            q,
            gain_db,
            enabled,
        });
    }

    pub fn add_eq_band(&self, kind: i32, frequency: f32, q: f32, gain_db: f32) {
        eq_ops::add_band(&self.eq, EqBand {
            kind: EqFilterKind::from_i32(kind),
            frequency,
            q,
            gain_db,
            enabled: true,
        });
    }

    pub fn remove_eq_band(&self, index: usize) {
        eq_ops::remove_band(&self.eq, index);
    }

    pub fn eq_bands(&self) -> Vec<EqBandInfo> {
        eq_ops::snapshot(&self.eq)
            .into_iter()
            .map(|b| EqBandInfo {
                kind: b.kind.as_i32(),
                frequency: b.frequency,
                q: b.q,
                gain_db: b.gain_db,
                enabled: b.enabled,
            })
            .collect()
    }

    pub fn set_pitch_semitones(&self, semitones: f32) {
        PitchShifter::set_semitones_shared(&self.pitch, semitones);
    }

    pub fn pitch_semitones(&self) -> f32 {
        f32::from_bits(self.pitch.load(Ordering::Relaxed))
    }

    pub fn spectrum(&self) -> Vec<f32> {
        SpectrumAnalyzer::snapshot(&self.spectrum)
    }

    pub fn set_output_gain_db(&self, db: f32) {
        self.output_gain.store(10f32.powf(db / 20.0).to_bits(), Ordering::Relaxed);
    }

    pub fn output_gain_db(&self) -> f32 {
        20.0 * f32::from_bits(self.output_gain.load(Ordering::Relaxed)).log10()
    }

    pub fn set_output_muted(&self, muted: bool) {
        self.output_muted.store(muted, Ordering::Relaxed);
    }

    pub fn is_output_muted(&self) -> bool {
        self.output_muted.load(Ordering::Relaxed)
    }

    pub fn start_recording(&self) {
        if let Ok(mut rec) = self.recorder.lock() { rec.start(); }
    }

    pub fn stop_recording(&self) {
        if let Ok(mut rec) = self.recorder.lock() { rec.stop(); }
    }

    pub fn is_recording(&self) -> bool {
        self.recorder.lock().map(|r| r.recording).unwrap_or(false)
    }

    pub fn recorded_samples(&self) -> usize {
        self.recorder.lock().map(|r| r.samples.len()).unwrap_or(0)
    }

    pub fn save_recording(&self, path: &str) -> Result<()> {
        let (samples, sr, channels) = {
            let rec = self.recorder.lock().map_err(|_| AurivoError::InvalidConfig("recorder lock poisoned".into()))?;
            let sr = if rec.sample_rate > 0 { rec.sample_rate } else { self.sample_rate };
            (rec.samples.clone(), sr, rec.channels)
        };
        write_wav(path, &samples, sr, channels)?;
        Ok(())
    }

    pub fn play_wav(&mut self, path: &str) -> Result<()> {
        self.playback_stream = None;
        let decoded = read_wav(path)?;
        let device = self.host.default_output_device().ok_or_else(|| AurivoError::DeviceNotFound("output".into()))?;
        let cfg = device.default_output_config().map_err(|e| AurivoError::Stream(e.to_string()))?;
        if cfg.sample_format() != cpal::SampleFormat::F32 {
            return Err(AurivoError::Stream("output device does not support f32".into()));
        }
        let file_format = StreamFormat::new(decoded.sample_rate, decoded.channels);
        let device_format = StreamFormat::new(cfg.sample_rate(), cfg.channels());
        let samples = adapt_playback(&decoded.samples, file_format, device_format);
        let state = Arc::new(Mutex::new((samples, 0usize)));
        let stream = device.build_output_stream(
            cfg.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if let Ok(mut guard) = state.lock() {
                    let (samples, index) = &mut *guard;
                    for d in data.iter_mut() {
                        if *index < samples.len() {
                            *d = samples[*index];
                            *index += 1;
                        } else {
                            *d = 0.0;
                        }
                    }
                }
            },
            |err| eprintln!("playback stream error: {err}"),
            None,
        ).map_err(|e| AurivoError::Stream(e.to_string()))?;
        stream.play().map_err(|e| AurivoError::Stream(e.to_string()))?;
        self.playback_stream = Some(stream);
        Ok(())
    }

    pub fn stop_playback(&mut self) {
        self.playback_stream = None;
    }

    pub fn effect_names(&self) -> Vec<String> {
        self.chain.lock().map(|c| c.names()).unwrap_or_default()
    }

    pub fn effect_enabled(&self, name: &str) -> bool {
        self.chain.lock().map(|c| c.enabled(name)).unwrap_or(false)
    }

    pub fn set_effect_enabled(&self, name: &str, enabled: bool) -> bool {
        self.chain.lock().map(|mut c| c.set_enabled(name, enabled)).unwrap_or(false)
    }

    pub fn remove_effect(&self, name: &str) -> bool {
        self.chain.lock().map(|mut c| c.remove(name)).unwrap_or(false)
    }

    pub fn move_effect(&self, name: &str, delta: i32) -> bool {
        self.chain.lock().map(|mut c| c.move_by(name, delta as isize)).unwrap_or(false)
    }

    pub fn reorder_effects(&self, names: &[String]) -> bool {
        self.chain.lock().map(|mut c| c.reorder(names)).unwrap_or(false)
    }

    pub fn state(&self) -> AppState {
        let d = self.delay_params();
        let r = self.reverb_params();
        let x = self.distortion_params();
        let m = self.modulation_params();
        let v = self.vocal_params();
        let mut s = AppState::default();
        s.chain = self.effect_names();
        s.enabled = s.chain.iter().map(|n| (n.clone(), self.effect_enabled(n))).collect();
        s.input_gain_db = f32::from_bits(self.gain.load(Ordering::Relaxed));
        s.output_gain_db = self.output_gain_db();
        s.output_muted = self.is_output_muted();
        s.delay = [d.0, d.1, d.2, d.3];
        s.reverb = [r.0, r.1, r.2, r.3];
        s.distortion = [x.0 as f32, x.1, x.2, x.3, x.4, x.5];
        s.modulation = [m.0 as f32, m.1, m.2, m.3, m.4];
        s.pitch_semitones = self.pitch_semitones();
        s.eq_bands = self.eq_bands();
        s.vocal = [v.0 as f32, v.1, v.2, v.3, v.4];
        s
    }

    pub fn apply_state(&self, s: &AppState) {
        if !s.chain.is_empty() {
            self.reorder_effects(&s.chain);
        }
        for (name, enabled) in &s.enabled {
            self.set_effect_enabled(name, *enabled);
        }
        self.set_input_gain_db(s.input_gain_db);
        self.set_output_gain_db(s.output_gain_db);
        self.set_output_muted(s.output_muted);
        let d = &s.delay;
        self.set_delay(d[0], d[1], d[2], d[3]);
        let r = &s.reverb;
        self.set_reverb(r[0], r[1], r[2], r[3]);
        let x = &s.distortion;
        self.set_distortion(x[0] as i32, x[1], x[2], x[3], x[4], x[5]);
        let m = &s.modulation;
        self.set_modulation(m[0] as i32, m[1], m[2], m[3], m[4]);
        self.set_pitch_semitones(s.pitch_semitones);
        let current = self.eq_bands().len();
        let target = s.eq_bands.len();
        for i in (target..current).rev() {
            self.remove_eq_band(i);
        }
        for b in s.eq_bands.iter().skip(current) {
            self.add_eq_band(b.kind, b.frequency, b.q, b.gain_db);
        }
        for (i, b) in s.eq_bands.iter().enumerate() {
            self.set_eq_band(i, b.kind, b.frequency, b.q, b.gain_db, b.enabled);
        }
        let v = &s.vocal;
        self.set_vocal(v[0] as i32, v[1], v[2], v[3], v[4]);
    }

    pub fn automation_routes(&self) -> Vec<(String, i32, f32, f32, bool)> {
        self.automation.lock().map(|bank| (&*bank).into()).unwrap_or_default()
    }

    pub fn add_automation(&self, target: &str, shape: i32, rate: f32, depth: f32) -> bool {
        let base = match target {
            "gain_db" => self.input_gain_db(),
            "output_gain_db" => self.output_gain_db(),
            "pitch" => self.pitch_semitones(),
            _ => return false,
        };
        let route = AutomationRoute::new(target, LfoShape::from_i32(shape), rate, depth);
        if let Ok(mut bank) = self.automation.lock() {
            bank.add(route);
        } else {
            return false;
        }
        if let Ok(mut bases) = self.automation_base.lock() {
            bases.insert(target.to_string(), base);
        }
        true
    }

    pub fn remove_automation(&self, target: &str) -> bool {
        if let Ok(mut bank) = self.automation.lock() {
            bank.remove(target)
        } else {
            false
        }
    }

    pub fn clear_automation(&self) {
        if let Ok(mut bank) = self.automation.lock() { bank.clear(); }
        let bases = match self.automation_base.lock() {
            Ok(mut map) => std::mem::take(&mut *map),
            Err(_) => BTreeMap::new(),
        };
        for (target, base) in bases {
            match target.as_str() {
                "gain_db" => self.set_input_gain_db(base),
                "output_gain_db" => self.set_output_gain_db(base),
                "pitch" => self.set_pitch_semitones(base),
                _ => {}
            }
        }
    }

    pub fn control_bindings(&self) -> Vec<ControlBinding> {
        self.control_map.lock().map(|m| m.bindings().to_vec()).unwrap_or_default()
    }

    pub fn bind_control(&self, channel: u8, controller: u8, target: &str, min: f32, max: f32) -> bool {
        match target {
            "gain_db" | "output_gain_db" | "pitch" => {}
            _ => return false,
        }
        if let Ok(mut map) = self.control_map.lock() {
            map.add(ControlBinding::new(channel, controller, target, min, max))
        } else {
            false
        }
    }

    pub fn unbind_control(&self, channel: u8, controller: u8) -> bool {
        self.control_map.lock().map(|mut m| m.remove_exact(channel, controller)).unwrap_or(false)
    }

    pub fn clear_controls(&self) {
        if let Ok(mut map) = self.control_map.lock() { map.clear(); }
    }

    pub fn send_control(&self, channel: u8, controller: u8, value: f32) -> usize {
        let message = ControlMessage::new(channel, controller, value);
        let applied = match self.control_map.lock() {
            Ok(map) => map.apply(&message),
            Err(_) => return 0,
        };
        for (target, value) in &applied {
            match target.as_str() {
                "gain_db" => self.set_input_gain_db(*value),
                "output_gain_db" => self.set_output_gain_db(*value),
                "pitch" => self.set_pitch_semitones(*value),
                _ => {}
            }
        }
        applied.len()
    }

    pub fn tick_automation(&self, seconds: f32) {
        let offsets = match self.automation.lock() {
            Ok(mut bank) => bank.sample_all(seconds),
            Err(_) => return,
        };
        for (target, offset) in offsets {
            let base = self.automation_base.lock()
                .ok()
                .and_then(|b| b.get(&target).copied())
                .unwrap_or(0.0);
            match target.as_str() {
                "gain_db" => self.set_input_gain_db(base + offset),
                "output_gain_db" => self.set_output_gain_db(base + offset),
                "pitch" => self.set_pitch_semitones(base + offset),
                _ => {}
            }
        }
    }

    pub fn set_vocal(&self, mode: i32, carrier_hz: f32, vowel: f32, depth: f32, mix: f32) {
        Vocal::update(&self.vocal, VocalParams {
            mode: aurivo_dsp::validate::safe_mode(mode, 3),
            carrier_hz,
            vowel,
            depth,
            mix,
        });
    }

    pub fn vocal_params(&self) -> (i32, f32, f32, f32, f32) {
        let p = Vocal::snapshot(&self.vocal);
        (p.mode, p.carrier_hz, p.vowel, p.depth, p.mix)
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}
