//! Modulation effects driven by a shared LFO: chorus, flanger, phaser and
//! vibrato. Chorus/flanger/vibrato use a modulated delay line; the phaser
//! uses a cascade of first-order all-pass filters whose coefficient sweeps.

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ModulationParams {
    pub mode: i32,
    pub rate_hz: f32,
    pub depth: f32,
    pub feedback: f32,
    pub mix: f32,
}

impl Default for ModulationParams {
    fn default() -> Self {
        Self { mode: 0, rate_hz: 1.5, depth: 0.4, feedback: 0.3, mix: 0.5 }
    }
}

#[derive(Debug)]
pub struct Modulation {
    buffer: Vec<f32>,
    write: usize,
    phase: f32,
    allpass: Vec<f32>,
    sample_rate: f32,
    params: Arc<Mutex<ModulationParams>>,
    bypass: bool,
}

impl Modulation {
    pub fn new(sample_rate: u32) -> Self {
        Self::with_params(sample_rate, ModulationParams::default())
    }

    pub fn with_params(sample_rate: u32, params: ModulationParams) -> Self {
        Self {
            buffer: vec![0.0; (sample_rate as f32 * 0.1) as usize + 8],
            write: 0,
            phase: 0.0,
            allpass: vec![0.0; 4],
            sample_rate: sample_rate as f32,
            params: Arc::new(Mutex::new(params)),
            bypass: false,
        }
    }

    pub fn shared(sample_rate: u32) -> (Self, Arc<Mutex<ModulationParams>>) {
        let fx = Self::new(sample_rate);
        let handle = fx.params.clone();
        (fx, handle)
    }

    pub fn set_bypass(&mut self, bypass: bool) {
        self.bypass = bypass;
    }

    pub fn is_bypassed(&self) -> bool {
        self.bypass
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.allpass.fill(0.0);
        self.phase = 0.0;
    }

    pub fn snapshot(handle: &Arc<Mutex<ModulationParams>>) -> ModulationParams {
        handle.lock().map(|p| p.clone()).unwrap_or_default()
    }

    pub fn update(handle: &Arc<Mutex<ModulationParams>>, params: ModulationParams) {
        if let Ok(mut current) = handle.lock() {
            *current = params;
        }
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        if self.bypass {
            return;
        }
        let (mode, rate, depth, feedback, mix) = match self.params.lock() {
            Ok(p) => (p.mode, p.rate_hz, p.depth, p.feedback, p.mix),
            Err(_) => return,
        };
        let rate = rate.clamp(0.01, 20.0);
        let depth = depth.clamp(0.0, 1.0);
        let feedback = feedback.clamp(0.0, 0.95);
        let mix = mix.clamp(0.0, 1.0);
        let phase_step = rate / self.sample_rate;
        let len = self.buffer.len();
        let is_vibrato = mode == 3;
        let is_phaser = mode == 2;
        let wet_mix = if is_vibrato { 1.0 } else { mix };

        for sample in samples.iter_mut() {
            let input = *sample;
            self.phase += phase_step;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            let lfo = (std::f32::consts::TAU * self.phase).sin();

            let wet = if is_phaser {
                let coeff = 0.5 + 0.5 * depth * lfo;
                let mut y = input;
                for state in self.allpass.iter_mut() {
                    let out = coeff * y + *state;
                    *state = y - coeff * out;
                    y = out;
                }
                y
            } else {
                let base_ms = match mode {
                    1 => 2.0,
                    _ => 20.0,
                };
                let mod_ms = depth * 12.0 * lfo;
                let delay_ms = (base_ms + mod_ms).max(0.5);
                let delay = ((delay_ms * self.sample_rate / 1000.0) as usize).clamp(1, len - 1);
                let read = (self.write + len - delay) % len;
                let delayed = self.buffer[read];
                let stored = if mode == 1 { input + feedback * delayed } else { input };
                self.buffer[self.write] = stored;
                self.write = (self.write + 1) % len;
                delayed
            };

            *sample = (1.0 - wet_mix) * input + wet_mix * wet;
        }
    }
}
