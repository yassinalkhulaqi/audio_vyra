pub mod recorder;

use aurivo_core::{AudioConfig, DeviceInfo, AurivoError, Result, LevelMeter};
use recorder::{RecorderState, read_wav, write_wav};
use aurivo_dsp::{ProcessingPipeline, AudioProcessor, Gain, LevelMeterProcessor, NoiseGate, Compressor, Biquad};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{HeapRb, traits::{Producer, Consumer, Split}};
use std::sync::{Arc, Mutex, atomic::{AtomicU32, Ordering}};

pub struct AudioEngine {
    host: cpal::Host,
    input_stream: Option<cpal::Stream>,
    output_stream: Option<cpal::Stream>,
    pipeline: Arc<Mutex<ProcessingPipeline>>,
    meter: Arc<LevelMeterProcessor>,
    gain: Arc<AtomicU32>,
    recorder: Arc<Mutex<RecorderState>>,
    playback_stream: Option<cpal::Stream>,
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
        let meter = Arc::new(LevelMeterProcessor::new());
        let recorder = Arc::new(Mutex::new(RecorderState::new(sr, 1)));
        Self {
            host,
            input_stream: None,
            output_stream: None,
            pipeline: Arc::new(Mutex::new(pipeline)),
            meter,
            gain: gain_shared,
            recorder,
            playback_stream: None,
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
        let recorder = self.recorder.clone();
        let in_stream = input_dev.build_input_stream(
            in_cfg.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut buf = data.to_vec();
                if let Ok(mut p) = pipeline.try_lock() { p.process(&mut buf); }
                meter.process(&buf);
                if let Ok(mut rec) = recorder.lock() { rec.push(&buf); }
                let _ = prod.push_slice(&buf);
            },
            |err| eprintln!("input stream error: {err}"),
            None,
        ).map_err(|e| AurivoError::Stream(e.to_string()))?;

        let out_stream = output_dev.build_output_stream(
            out_cfg.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let n = cons.pop_slice(data);
                if n < data.len() { data[n..].fill(0.0); }
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
        Ok(())
    }

    pub fn stop(&mut self) {
        self.input_stream = None;
        self.output_stream = None;
        self.running = false;
    }

    pub fn is_running(&self) -> bool { self.running }
    pub fn levels(&self) -> LevelMeter { LevelMeter { peak: self.meter.peak(), rms: self.meter.rms() } }
    pub fn set_input_gain_db(&self, db: f32) { self.gain.store(db.to_bits(), Ordering::Relaxed); }

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
}

