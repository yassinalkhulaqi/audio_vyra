pub mod recorder;

use aurivo_core::{AudioConfig, DeviceInfo, AurivoError, Result, LevelMeter};
use recorder::{RecorderState, read_wav, write_wav};
use aurivo_dsp::{ProcessingPipeline, AudioProcessor, Gain, LevelMeterProcessor, NoiseGate, Compressor, Biquad, Delay, DelayParams, Reverb, ReverbParams, Distortion, DistortionParams, Modulation, ModulationParams, ParametricEq, EqBand, EqFilterKind, EqParams, PitchShifter, SpectrumAnalyzer, Vocal, VocalParams};
use aurivo_dsp::eq as eq_ops;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{HeapRb, traits::{Producer, Consumer, Split}};
use serde::Serialize;
use std::sync::{Arc, Mutex, atomic::{AtomicU32, AtomicU64, AtomicBool, Ordering}};

#[derive(Debug, Clone, Serialize)]
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
    pipeline: Arc<Mutex<ProcessingPipeline>>,
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
    playback_stream: Option<cpal::Stream>,
    vocal: Arc<Mutex<VocalParams>>,
    running: bool,
    sample_rate: u32,
    channels: u16,
}

impl AudioEngine {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let sr = 48000u32;
        let (gain_proc, gain_shared) = Gain::shared();
        let mut pipeline = ProcessingPipeline::new(sr);
        pipeline.add(Box::new(gain_proc));
        pipeline.add(Box::new(NoiseGate::new(-40.0, 5.0, 50.0, sr as f32)));
        pipeline.add(Box::new(Compressor::new(-18.0, 3.0, 10.0, 100.0, 2.0, sr as f32)));
        pipeline.add(Box::new(Biquad::new_highpass(80.0, 0.707, sr as f32)));
        let (delay_proc, delay_shared) = Delay::shared(sr);
        pipeline.add(Box::new(delay_proc));
        let (reverb_proc, reverb_shared) = Reverb::shared(sr);
        pipeline.add(Box::new(reverb_proc));
        let (distortion_proc, distortion_shared) = Distortion::shared(sr);
        pipeline.add(Box::new(distortion_proc));
        let (modulation_proc, modulation_shared) = Modulation::shared(sr);
        pipeline.add(Box::new(modulation_proc));
        let (eq_proc, eq_shared) = ParametricEq::shared(sr);
        pipeline.add(Box::new(eq_proc));
        let (pitch_proc, pitch_shared) = PitchShifter::shared(sr);
        pipeline.add(Box::new(pitch_proc));
        let (vocal_proc, vocal_shared) = Vocal::shared(sr);
        pipeline.add(Box::new(vocal_proc));
        let meter = Arc::new(LevelMeterProcessor::new());
        let (analyzer, spectrum_handle) = SpectrumAnalyzer::shared(1024);
        let analyzer = Arc::new(Mutex::new(analyzer));
        let output_gain = Arc::new(AtomicU32::new(1.0f32.to_bits()));
        let output_muted = Arc::new(AtomicBool::new(false));
        let recorder = Arc::new(Mutex::new(RecorderState::new(sr, 1)));
        let overruns = Arc::new(AtomicU64::new(0));
        let underruns = Arc::new(AtomicU64::new(0));
        Self {
            host,
            input_stream: None,
            output_stream: None,
            pipeline: Arc::new(Mutex::new(pipeline)),
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
            playback_stream: None,
            vocal: vocal_shared,
            running: false,
            sample_rate: sr,
            channels: 1,
        }
    }

    pub fn list_input_devices(&self) -> Vec<String> {
        self.host.input_devices().map(|mut d| d.map(|dev| dev.description().map(|d| d.name().to_string()).unwrap_or_default()).collect()).unwrap_or_default()
    }
    pub fn list_output_devices(&self) -> Vec<String> {
        self.host.output_devices().map(|mut d| d.map(|dev| dev.description().map(|d| d.name().to_string()).unwrap_or_default()).collect()).unwrap_or_default()
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

        let ring = HeapRb::<f32>::new((sr as usize * ch as usize * 4) as usize);
        let (mut prod, mut cons) = ring.split();

        let pipeline = self.pipeline.clone();
        let meter = self.meter.clone();
        let analyzer = self.analyzer.clone();
        let recorder = self.recorder.clone();
        let overruns = self.overruns.clone();
        let underruns = self.underruns.clone();
        let in_stream = input_dev.build_input_stream(
            in_cfg.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut buf = data.to_vec();
                if let Ok(mut p) = pipeline.try_lock() { p.process(&mut buf); }
                meter.process(&buf);
                if let Ok(mut a) = analyzer.try_lock() { a.push(&buf); }
                if let Ok(mut rec) = recorder.lock() { rec.push(&buf); }
                let pushed = prod.push_slice(&buf);
                if pushed < buf.len() {
                    overruns.fetch_add((buf.len() - pushed) as u64, Ordering::Relaxed);
                }
            },
            |err| eprintln!("input stream error: {err}"),
            None,
        ).map_err(|e| AurivoError::Stream(e.to_string()))?;

        let output_gain = self.output_gain.clone();
        let output_muted = self.output_muted.clone();
        let out_stream = output_dev.build_output_stream(
            out_cfg.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let n = cons.pop_slice(data);
                if n < data.len() {
                    underruns.fetch_add((data.len() - n) as u64, Ordering::Relaxed);
                    data[n..].fill(0.0);
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
        self.channels = ch;
        if let Ok(mut rec) = self.recorder.lock() {
            rec.sample_rate = sr;
            rec.channels = ch;
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        self.input_stream = None;
        self.output_stream = None;
        self.running = false;
    }

    pub fn is_running(&self) -> bool { self.running }
    pub fn audio_stats(&self) -> (u64, u64) {
        (
            self.overruns.load(Ordering::Relaxed),
            self.underruns.load(Ordering::Relaxed),
        )
    }
    pub fn reset_audio_stats(&self) {
        self.overruns.store(0, Ordering::Relaxed);
        self.underruns.store(0, Ordering::Relaxed);
    }
    pub fn levels(&self) -> LevelMeter { LevelMeter { peak: self.meter.peak(), rms: self.meter.rms() } }
    pub fn set_input_gain_db(&self, db: f32) { self.gain.store(db.to_bits(), Ordering::Relaxed); }

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
        Distortion::update(&self.distortion, DistortionParams { mode, drive_db, tone, mix, downsample, bit_depth });
    }

    pub fn distortion_params(&self) -> (i32, f32, f32, f32, f32, f32) {
        let p = Distortion::snapshot(&self.distortion);
        (p.mode, p.drive_db, p.tone, p.mix, p.downsample, p.bit_depth)
    }

    pub fn set_modulation(&self, mode: i32, rate_hz: f32, depth: f32, feedback: f32, mix: f32) {
        Modulation::update(&self.modulation, ModulationParams { mode, rate_hz, depth, feedback, mix });
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
        let rec = self.recorder.lock().map_err(|_| AurivoError::InvalidConfig("recorder lock poisoned".into()))?;
        let sr = if rec.sample_rate > 0 { rec.sample_rate } else { self.sample_rate };
        write_wav(path, &rec.samples, sr, rec.channels)?;
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
        let state = Arc::new(Mutex::new((decoded.samples, 0usize)));
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

    pub fn set_vocal(&self, mode: i32, carrier_hz: f32, vowel: f32, depth: f32, mix: f32) {
        Vocal::update(&self.vocal, VocalParams { mode, carrier_hz, vowel, depth, mix });
    }

    pub fn vocal_params(&self) -> (i32, f32, f32, f32, f32) {
        let p = Vocal::snapshot(&self.vocal);
        (p.mode, p.carrier_hz, p.vowel, p.depth, p.mix)
    }
}
