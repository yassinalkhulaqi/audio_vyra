//! Small linear resampler used when the input and output devices run at
//! different sample rates. Keeps leftover source samples between callback
//! blocks so the interpolation stays continuous.

pub struct LinearResampler {
    step: f64,
    pos: f64,
    pending: Vec<f32>,
}

impl LinearResampler {
    pub fn new(from_rate: u32, to_rate: u32) -> Self {
        Self {
            step: from_rate as f64 / to_rate.max(1) as f64,
            pos: 0.0,
            pending: Vec::new(),
        }
    }

    pub fn set_rates(&mut self, from_rate: u32, to_rate: u32) {
        self.step = from_rate as f64 / to_rate.max(1) as f64;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.pos = 0.0;
        self.pending.clear();
    }

    pub fn process(&mut self, src: &[f32], dst: &mut [f32]) -> usize {
        self.pending.extend_from_slice(src);
        let mut n = 0;
        while n < dst.len() && self.pos + 1.0 < self.pending.len() as f64 {
            let i = self.pos.floor() as usize;
            let frac = (self.pos - i as f64) as f32;
            dst[n] = self.pending[i] * (1.0 - frac) + self.pending[i + 1] * frac;
            n += 1;
            self.pos += self.step;
        }
        let len = self.pending.len() as f64;
        if self.pos >= len {
            self.pos -= len;
            self.pending.clear();
        } else {
            let keep = self.pos.floor() as usize;
            if keep > 0 {
                self.pending.drain(..keep);
                self.pos -= keep as f64;
            }
        }
        n
    }
}
