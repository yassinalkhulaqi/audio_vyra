//! Freeverb-style reverb: a bank of comb filters in parallel followed by
//! all-pass filters in series. Room size controls feedback, damping controls
//! high-frequency absorption in the comb feedback path.

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ReverbParams {
    pub room_size: f32,
    pub damp: f32,
    pub wet: f32,
    pub dry: f32,
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self { room_size: 0.6, damp: 0.4, wet: 0.25, dry: 0.85 }
    }
}

#[derive(Debug)]
struct Comb {
    buffer: Vec<f32>,
    index: usize,
    filtered: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self { buffer: vec![0.0; len.max(1)], index: 0, filtered: 0.0 }
    }

    fn process(&mut self, input: f32, feedback: f32, damp: f32) -> f32 {
        let out = self.buffer[self.index];
        self.filtered = out * (1.0 - damp) + self.filtered * damp;
        self.buffer[self.index] = input + self.filtered * feedback;
        self.index = (self.index + 1) % self.buffer.len();
        out
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.filtered = 0.0;
    }
}

#[derive(Debug)]
struct Allpass {
    buffer: Vec<f32>,
    index: usize,
}

impl Allpass {
    fn new(len: usize) -> Self {
        Self { buffer: vec![0.0; len.max(1)], index: 0 }
    }

    fn process(&mut self, input: f32, feedback: f32) -> f32 {
        let buf_out = self.buffer[self.index];
        let out = -input + buf_out;
        self.buffer[self.index] = input + buf_out * feedback;
        self.index = (self.index + 1) % self.buffer.len();
        out
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
    }
}

#[derive(Debug)]
pub struct Reverb {
    combs: Vec<Comb>,
    allpasses: Vec<Allpass>,
    params: Arc<Mutex<ReverbParams>>,
    bypass: bool,
}

impl Reverb {
    const COMB_LENGTHS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
    const ALLPASS_LENGTHS: [usize; 4] = [556, 441, 341, 225];

    pub fn new(sample_rate: u32) -> Self {
        Self::with_params(sample_rate, ReverbParams::default())
    }

    pub fn with_params(sample_rate: u32, params: ReverbParams) -> Self {
        let scale = sample_rate as f32 / 44100.0;
        let combs = Self::COMB_LENGTHS
            .iter()
            .map(|&len| Comb::new((len as f32 * scale).max(8.0) as usize))
            .collect();
        let allpasses = Self::ALLPASS_LENGTHS
            .iter()
            .map(|&len| Allpass::new((len as f32 * scale).max(4.0) as usize))
            .collect();
        Self {
            combs,
            allpasses,
            params: Arc::new(Mutex::new(params)),
            bypass: false,
        }
    }

    pub fn shared(sample_rate: u32) -> (Self, Arc<Mutex<ReverbParams>>) {
        let reverb = Self::new(sample_rate);
        let handle = reverb.params.clone();
        (reverb, handle)
    }

    pub fn set_bypass(&mut self, bypass: bool) {
        self.bypass = bypass;
    }

    pub fn is_bypassed(&self) -> bool {
        self.bypass
    }

    pub fn reset(&mut self) {
        for comb in &mut self.combs {
            comb.reset();
        }
        for allpass in &mut self.allpasses {
            allpass.reset();
        }
    }

    pub fn snapshot(handle: &Arc<Mutex<ReverbParams>>) -> ReverbParams {
        handle.lock().map(|p| p.clone()).unwrap_or_default()
    }

    pub fn update(handle: &Arc<Mutex<ReverbParams>>, params: ReverbParams) {
        if let Ok(mut current) = handle.lock() {
            *current = params;
        }
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        if self.bypass {
            return;
        }
        let (room, damp, wet, dry) = match self.params.lock() {
            Ok(p) => (p.room_size.clamp(0.0, 1.0), p.damp.clamp(0.0, 1.0), p.wet.clamp(0.0, 1.0), p.dry.clamp(0.0, 1.0)),
            Err(_) => return,
        };
        let feedback = 0.7 + 0.28 * room;
        let allpass_feedback = 0.5;
        let comb_norm = 1.0 / self.combs.len() as f32;

        for sample in samples.iter_mut() {
            let input = *sample;
            let mut sum = 0.0f32;
            for comb in self.combs.iter_mut() {
                sum += comb.process(input, feedback, damp);
            }
            let mut tail = sum * comb_norm;
            for allpass in self.allpasses.iter_mut() {
                tail = allpass.process(tail, allpass_feedback);
            }
            *sample = dry * input + wet * tail * 2.0;
        }
    }
}
