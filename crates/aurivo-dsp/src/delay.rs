//! Feedback delay / echo with a damping filter in the feedback path.
//! A fixed circular buffer (2 seconds) holds the delay line so the delay time
//! can be changed at control rate without reallocating in the audio thread.

use std::sync::{Arc, Mutex};
use crate::denormal::flush;
use crate::rt_param::{try_snapshot, try_update, RtParam};

#[derive(Debug, Clone)]
pub struct DelayParams {
    pub time_ms: f32,
    pub feedback: f32,
    pub mix: f32,
    pub damp: f32,
}

impl Default for DelayParams {
    fn default() -> Self {
        Self { time_ms: 300.0, feedback: 0.35, mix: 0.3, damp: 0.2 }
    }
}

#[derive(Debug)]
pub struct Delay {
    buffer: Vec<f32>,
    write: usize,
    filtered: f32,
    sample_rate: f32,
    params: RtParam<DelayParams>,
    bypass: bool,
}

impl Delay {
    pub fn new(sample_rate: u32) -> Self {
        Self::with_params(sample_rate, DelayParams::default())
    }

    pub fn with_params(sample_rate: u32, params: DelayParams) -> Self {
        let max_delay = (sample_rate as f32 * 2.0) as usize;
        Self {
            buffer: vec![0.0; max_delay.max(8)],
            write: 0,
            filtered: 0.0,
            sample_rate: sample_rate as f32,
            params: RtParam::new(Arc::new(Mutex::new(params.clone())), params),
            bypass: false,
        }
    }

    pub fn shared(sample_rate: u32) -> (Self, Arc<Mutex<DelayParams>>) {
        let delay = Self::new(sample_rate);
        let handle = delay.params.handle();
        (delay, handle)
    }

    pub fn set_bypass(&mut self, bypass: bool) {
        self.bypass = bypass;
    }

    pub fn is_bypassed(&self) -> bool {
        self.bypass
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.filtered = 0.0;
    }

    pub fn snapshot(handle: &Arc<Mutex<DelayParams>>) -> DelayParams {
        try_snapshot(handle, DelayParams::default())
    }

    pub fn update(handle: &Arc<Mutex<DelayParams>>, params: DelayParams) {
        try_update(handle, params);
    }

    /// Number of parameter polls that reused the cached snapshot.
    pub fn skipped_polls(&self) -> u64 {
        self.params.skipped()
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        if self.bypass {
            return;
        }
        let (time_ms, feedback, mix, damp) = {
            let p = self.params.poll();
            (p.time_ms, p.feedback, p.mix, p.damp)
        };
        let len = self.buffer.len();
        let delay_samples = ((time_ms * self.sample_rate / 1000.0) as usize)
            .clamp(1, len - 1);
        let feedback = feedback.clamp(0.0, 0.98);
        let mix = mix.clamp(0.0, 1.0);
        let damp = damp.clamp(0.0, 1.0);

        for sample in samples.iter_mut() {
            let input = *sample;
            let read = (self.write + len - delay_samples) % len;
            let delayed = self.buffer[read];

            self.filtered = flush(damp * self.filtered + (1.0 - damp) * delayed);
            self.buffer[self.write] = input + feedback * self.filtered;
            self.write = (self.write + 1) % len;

            *sample = (1.0 - mix) * input + mix * delayed;
        }
    }
}
