//! Vocal-oriented effects: ring modulation, robotizer, vowel formant filter
//! bank and tremolo. Formant targets are the classic F1/F2/F3 frequencies of
//! five vowels and can be morphed continuously by a fractional vowel index.

use std::sync::{Arc, Mutex};
use crate::rt_param::{try_snapshot, try_update, RtParam};
use crate::denormal::flush;

#[derive(Debug, Clone)]
pub struct VocalParams {
    pub mode: i32,
    pub carrier_hz: f32,
    pub vowel: f32,
    pub depth: f32,
    pub mix: f32,
}

impl Default for VocalParams {
    fn default() -> Self {
        Self { mode: 0, carrier_hz: 40.0, vowel: 0.0, depth: 0.6, mix: 0.6 }
    }
}

const FORMANT_A: [[f32; 3]; 5] = [
    [800.0, 1200.0, 2800.0],
    [400.0, 2000.0, 2550.0],
    [250.0, 2300.0, 3000.0],
    [400.0, 800.0, 2300.0],
    [350.0, 600.0, 2400.0],
];

#[derive(Debug, Clone)]
struct Bandpass {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Bandpass {
    fn new(freq: f32, q: f32, sample_rate: f32) -> Self {
        let freq = crate::validate::safe_frequency(freq, sample_rate).max(20.0);
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let alpha = w0.sin() / (2.0 * q.clamp(0.1, 20.0));
        let a0 = 1.0 + alpha;
        Self {
            b0: alpha / a0,
            b1: 0.0,
            b2: -alpha / a0,
            a1: (-2.0 * w0.cos()) / a0,
            a2: (1.0 - alpha) / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    /// Retune the filter without discarding its state. Replacing the whole
    /// biquad at every block boundary produced audible clicks in formant mode;
    /// a small coefficient glide keeps the filter continuous while it follows
    /// a morphing vowel.
    fn update(&mut self, freq: f32, q: f32, sample_rate: f32) {
        let next = Self::new(freq, q, sample_rate);
        const GLIDE: f32 = 0.08;
        self.b0 += (next.b0 - self.b0) * GLIDE;
        self.b1 += (next.b1 - self.b1) * GLIDE;
        self.b2 += (next.b2 - self.b2) * GLIDE;
        self.a1 += (next.a1 - self.a1) * GLIDE;
        self.a2 += (next.a2 - self.a2) * GLIDE;
    }

    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = flush(self.b1 * x - self.a1 * y + self.z2);
        self.z2 = flush(self.b2 * x - self.a2 * y);
        y
    }
}

#[derive(Debug)]
pub struct Vocal {
    filters: [Bandpass; 3],
    phase: f32,
    hold_counter: f32,
    held_amplitude: f32,
    sample_rate: f32,
    params: RtParam<VocalParams>,
    bypass: bool,
}

impl Vocal {
    pub fn new(sample_rate: u32) -> Self {
        Self::with_params(sample_rate, VocalParams::default())
    }

    pub fn with_params(sample_rate: u32, params: VocalParams) -> Self {
        let sr = sample_rate as f32;
        let f = FORMANT_A[0];
        Self {
            filters: [
                Bandpass::new(f[0], 8.0, sr),
                Bandpass::new(f[1], 10.0, sr),
                Bandpass::new(f[2], 12.0, sr),
            ],
            phase: 0.0,
            hold_counter: 0.0,
            held_amplitude: 0.0,
            sample_rate: sr,
            params: RtParam::new(Arc::new(Mutex::new(params.clone())), params),
            bypass: false,
        }
    }

    pub fn shared(sample_rate: u32) -> (Self, Arc<Mutex<VocalParams>>) {
        let fx = Self::new(sample_rate);
        let handle = fx.params.handle();
        (fx, handle)
    }

    pub fn set_bypass(&mut self, bypass: bool) {
        self.bypass = bypass;
    }

    pub fn is_bypassed(&self) -> bool {
        self.bypass
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.hold_counter = 0.0;
        self.held_amplitude = 0.0;
    }

    pub fn snapshot(handle: &Arc<Mutex<VocalParams>>) -> VocalParams {
        try_snapshot(handle, VocalParams::default())
    }

    pub fn update(handle: &Arc<Mutex<VocalParams>>, params: VocalParams) {
        try_update(handle, params);
    }

    pub fn skipped_polls(&self) -> u64 {
        self.params.skipped()
    }

    fn formant_targets(vowel: f32) -> [f32; 3] {
        let v = vowel.clamp(0.0, 4.0);
        let lo = v.floor() as usize;
        let hi = (lo + 1).min(4);
        let frac = v - lo as f32;
        let mut out = [0.0; 3];
        for i in 0..3 {
            out[i] = FORMANT_A[lo][i] * (1.0 - frac) + FORMANT_A[hi][i] * frac;
        }
        out
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        if self.bypass {
            return;
        }
        let (mode, carrier, vowel, depth, mix) = {
            let p = self.params.poll();
            (p.mode, p.carrier_hz, p.vowel, p.depth, p.mix)
        };
        let mix = mix.clamp(0.0, 1.0);
        let depth = depth.clamp(0.0, 1.0);
        let phase_step = carrier.clamp(1.0, 4000.0) / self.sample_rate;
        let targets = Self::formant_targets(vowel);
        for (band, freq) in self.filters.iter_mut().zip(targets.iter()) {
            band.update(*freq, 9.0, self.sample_rate);
        }

        for sample in samples.iter_mut() {
            let input = *sample;
            self.phase += phase_step;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            let sine = (std::f32::consts::TAU * self.phase).sin();

            let wet = match mode {
                1 => {
                    let square = if sine >= 0.0 { 1.0 } else { -1.0 };
                    self.hold_counter += 1.0;
                    if self.hold_counter >= 16.0 {
                        self.hold_counter -= 16.0;
                        self.held_amplitude = (input.abs() * 8.0).round() / 8.0;
                    }
                    square * self.held_amplitude * input.signum()
                }
                2 => {
                    let mut acc = 0.0f32;
                    for band in self.filters.iter_mut() {
                        acc += band.process(input);
                    }
                    acc / 3.0 * 4.0
                }
                3 => input * (1.0 - depth + depth * (0.5 + 0.5 * sine)),
                _ => input * sine,
            };

            *sample = (1.0 - mix) * input + mix * wet;
        }
    }
}
