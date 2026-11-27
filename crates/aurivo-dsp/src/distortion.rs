//! Distortion effects: tanh soft clipping, hard clipping, fuzz and
//! bitcrushing (sample-and-hold decimation + quantization), with a post-tone
//! filter and wet/dry mix. All parameters live in a shared handle.

use std::sync::{Arc, Mutex};
use crate::rt_param::{try_snapshot, try_update, RtParam};

#[derive(Debug, Clone)]
pub struct DistortionParams {
    pub mode: i32,
    pub drive_db: f32,
    pub tone: f32,
    pub mix: f32,
    pub downsample: f32,
    pub bit_depth: f32,
}

impl Default for DistortionParams {
    fn default() -> Self {
        Self { mode: 0, drive_db: 12.0, tone: 0.6, mix: 0.5, downsample: 1.0, bit_depth: 8.0 }
    }
}

#[derive(Debug)]
pub struct Distortion {
    params: RtParam<DistortionParams>,
    hold_counter: f32,
    held: f32,
    tone_state: f32,
    bypass: bool,
}

impl Distortion {
    pub fn new(sample_rate: u32) -> Self {
        Self::with_params(sample_rate, DistortionParams::default())
    }

    pub fn with_params(_sample_rate: u32, params: DistortionParams) -> Self {
        Self {
            params: RtParam::new(Arc::new(Mutex::new(params.clone())), params),
            hold_counter: 0.0,
            held: 0.0,
            tone_state: 0.0,
            bypass: false,
        }
    }

    pub fn shared(sample_rate: u32) -> (Self, Arc<Mutex<DistortionParams>>) {
        let dist = Self::new(sample_rate);
        let handle = dist.params.handle();
        (dist, handle)
    }

    pub fn set_bypass(&mut self, bypass: bool) {
        self.bypass = bypass;
    }

    pub fn is_bypassed(&self) -> bool {
        self.bypass
    }

    pub fn reset(&mut self) {
        self.hold_counter = 0.0;
        self.held = 0.0;
        self.tone_state = 0.0;
    }

    pub fn snapshot(handle: &Arc<Mutex<DistortionParams>>) -> DistortionParams {
        try_snapshot(handle, DistortionParams::default())
    }

    pub fn update(handle: &Arc<Mutex<DistortionParams>>, params: DistortionParams) {
        try_update(handle, params);
    }

    pub fn skipped_polls(&self) -> u64 {
        self.params.skipped()
    }

    fn shape(x: f32, mode: i32) -> f32 {
        match mode {
            1 => x.clamp(-1.0, 1.0),
            2 => {
                let a = x.abs();
                if a < 1e-9 { 0.0 } else { x.signum() * (1.0 - (-4.0 * a).exp()) }
            }
            _ => x.tanh(),
        }
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        if self.bypass {
            return;
        }
        let (mode, drive_db, tone, mix, downsample, bit_depth) = {
            let p = self.params.poll();
            (p.mode, p.drive_db, p.tone, p.mix, p.downsample, p.bit_depth)
        };
        let drive = 10f32.powf(drive_db.clamp(-12.0, 48.0) / 20.0);
        let mix = mix.clamp(0.0, 1.0);
        let steps = (2f32.powf(bit_depth.clamp(1.0, 24.0) - 1.0)).max(1.0);
        let hold = downsample.clamp(1.0, 64.0);
        let tone = tone.clamp(0.0, 1.0);
        let alpha = 0.05 + 0.94 * tone;

        for sample in samples.iter_mut() {
            let dry = *sample;
            let mut wet = dry * drive;

            if hold > 1.0 {
                self.hold_counter += 1.0;
                if self.hold_counter >= hold {
                    self.hold_counter -= hold;
                    self.held = wet;
                }
                wet = self.held;
            }

            wet = (wet * steps).round() / steps;
            wet = Self::shape(wet, mode) / drive.max(1.0).sqrt();

            self.tone_state = alpha * wet + (1.0 - alpha) * self.tone_state;
            wet = self.tone_state;

            *sample = (1.0 - mix) * dry + mix * wet;
        }
    }
}
