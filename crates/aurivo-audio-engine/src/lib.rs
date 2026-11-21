use aurivo_core::{AudioConfig, DeviceInfo, AurivoError, Result, LevelMeter};
use aurivo_dsp::{ProcessingPipeline, AudioProcessor, Gain, LevelMeterProcessor, NoiseGate, Compressor, Biquad, PitchShifter, Vocal, VocalParams};
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
    pitch: Arc<AtomicU32>,
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
        let (pitch_proc, pitch_shared) = PitchShifter::shared(sr);
        pipeline.add(Box::new(pitch_proc));
        let (vocal_proc, vocal_shared) = Vocal::shared(sr);
        pipeline.add(Box::new(vocal_proc));
        let meter = Arc::new(LevelMeterProcessor::new());
        Self {
            host,
            input_stream: None,
            output_stream: None,
            pipeline: Arc::new(Mutex::new(pipeline)),
            meter,
            gain: gain_shared,
            pitch: pitch_shared,
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
        let in_stream = input_dev.build_input_stream(
            in_cfg.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut buf = data.to_vec();
                if let Ok(mut p) = pipeline.try_lock() { p.process(&mut buf); }
                meter.process(&buf);
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
    pub fn set_pitch_semitones(&self, semitones: f32) {
        PitchShifter::set_semitones_shared(&self.pitch, semitones);
    }
    pub fn pitch_semitones(&self) -> f32 {
        f32::from_bits(self.pitch.load(Ordering::Relaxed))
    }
    pub fn set_vocal(&self, mode: i32, carrier_hz: f32, vowel: f32, depth: f32, mix: f32) {
        Vocal::update(&self.vocal, VocalParams { mode, carrier_hz, vowel, depth, mix });
    }

    pub fn vocal_params(&self) -> (i32, f32, f32, f32, f32) {
        let p = Vocal::snapshot(&self.vocal);
        (p.mode, p.carrier_hz, p.vowel, p.depth, p.mix)
    }
}
