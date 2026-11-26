//! Linear resampler used when input and output devices run at different
//! sample rates. Keeps leftover source samples between callback blocks so
//! interpolation stays continuous across buffer boundaries.
//!
//! The resampler is frame-agnostic: feed it a single mono stream (or one
//! channel of an interleaved stream at a time). For multi-channel audio use
//! [`crate::channels::take_channel_into`] and
//! [`crate::channels::place_channel_into`] around it, or the higher-level
//! helpers in [`crate::audio_path`].

/// Stateful linear interpolator with a fractional read position.
#[derive(Debug)]
pub struct LinearResampler {
    step: f64,
    pos: f64,
    pending: Vec<f32>,
}

impl LinearResampler {
    /// Build a resampler converting `from_rate` Hz into `to_rate` Hz.
    pub fn new(from_rate: u32, to_rate: u32) -> Self {
        Self {
            step: ratio(from_rate, to_rate),
            pos: 0.0,
            pending: Vec::new(),
        }
    }

    /// Change rates and discard all buffered audio.
    pub fn set_rates(&mut self, from_rate: u32, to_rate: u32) {
        self.step = ratio(from_rate, to_rate);
        self.reset();
    }

    /// Current conversion ratio (source samples per destination sample).
    pub fn step(&self) -> f64 {
        self.step
    }

    /// Number of source samples still buffered.
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Discard buffered audio and reset the read position.
    pub fn reset(&mut self) {
        self.pos = 0.0;
        self.pending.clear();
    }

    /// Interpolate as many destination samples as `dst` can hold.
    ///
    /// Returns the number of destination samples written. Unwritten slots are
    /// left untouched so callers can zero-fill them or count an underrun.
    pub fn process(&mut self, src: &[f32], dst: &mut [f32]) -> usize {
        self.pending.extend_from_slice(src);
        let mut written = 0;
        while written < dst.len() && self.pos + 1.0 < self.pending.len() as f64 {
            let i = self.pos.floor() as usize;
            let frac = (self.pos - i as f64) as f32;
            let a = self.pending[i];
            let b = self.pending[i + 1];
            dst[written] = a + (b - a) * frac;
            written += 1;
            self.pos += self.step;
        }
        self.trim_consumed();
        written
    }

    /// Produce exactly `count` destination samples into a freshly allocated
    /// buffer. Intended for setup code and tests, not audio callbacks.
    pub fn process_vec(&mut self, src: &[f32], count: usize) -> Vec<f32> {
        let mut out = vec![0.0f32; count];
        let n = self.process(src, &mut out);
        out.truncate(n);
        out
    }

    fn trim_consumed(&mut self) {
        let len = self.pending.len();
        if len <= 1 {
            return;
        }
        let keep = self.pos.floor() as usize;
        let drop = keep.min(len - 1);
        if drop > 0 {
            self.pending.drain(..drop);
            self.pos -= drop as f64;
        }
    }
}

fn ratio(from: u32, to: u32) -> f64 {
    if to == 0 {
        return 1.0;
    }
    let r = from as f64 / to as f64;
    if r.is_finite() && r > 0.0 { r } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_rate_preserves_samples() {
        let mut r = LinearResampler::new(48000, 48000);
        let src: Vec<f32> = (0..64).map(|i| i as f32 / 64.0).collect();
        let out = r.process_vec(&src, 64);
        assert_eq!(out.len(), 63);
        for (i, &v) in out.iter().enumerate() {
            assert!((v - src[i]).abs() < 1e-5, "sample {i}: {v} != {}", src[i]);
        }
    }

    #[test]
    fn upsampling_doubles_length_and_interpolates() {
        let mut r = LinearResampler::new(48000, 96000);
        let src = [0.0f32, 1.0, 0.0, 1.0];
        let mut out = vec![0.0f32; 16];
        let n = r.process(&src, &mut out);
        assert!(n >= 6);
        assert!(out.iter().any(|v| *v > 0.1 && *v < 0.9));
    }

    #[test]
    fn downsampling_is_stable() {
        let mut r = LinearResampler::new(96000, 48000);
        let mut out = Vec::new();
        for block in 0..20 {
            let src: Vec<f32> = (0..32).map(|i| ((block * 32 + i) as f32 * 0.01).sin()).collect();
            let mut dst = vec![0.0f32; 16];
            let n = r.process(&src, &mut dst);
            assert!(n <= 16);
            assert!(dst[..n].iter().all(|v| v.is_finite()));
            out.extend_from_slice(&dst[..n]);
        }
        assert!(!out.is_empty());
    }

    #[test]
    fn block_boundaries_match_single_block() {
        let src: Vec<f32> = (0..256).map(|i| (i as f32 * 0.05).sin()).collect();
        let mut one = LinearResampler::new(44100, 48000);
        let whole = one.process_vec(&src, 512);

        let mut split = LinearResampler::new(44100, 48000);
        let mut parts = Vec::new();
        for chunk in src.chunks(17) {
            let mut dst = vec![0.0f32; 32];
            let n = split.process(chunk, &mut dst);
            parts.extend_from_slice(&dst[..n]);
        }
        assert_eq!(whole.len(), parts.len());
        for (i, (a, b)) in whole.iter().zip(parts.iter()).enumerate() {
            assert!((a - b).abs() < 1e-4, "boundary sample {i}: {a} != {b}");
        }
    }

    #[test]
    fn extreme_ratios_do_not_hang() {
        for (from, to) in [(1, 48000), (48000, 1), (192000, 8000)] {
            let mut r = LinearResampler::new(from, to);
            let mut dst = vec![0.0f32; 8];
            let n = r.process(&[1.0, 0.5], &mut dst);
            assert!(n <= 8);
            assert!(dst.iter().all(|v| v.is_finite()));
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut r = LinearResampler::new(48000, 44100);
        let mut dst = vec![0.0f32; 8];
        r.process(&[1.0; 64], &mut dst);
        assert!(r.pending_len() > 0);
        r.reset();
        assert_eq!(r.pending_len(), 0);
        r.set_rates(44100, 48000);
        assert!((r.step() - 44100.0 / 48000.0).abs() < 1e-12);
    }

    #[test]
    fn zero_rate_falls_back_to_unity() {
        let r = LinearResampler::new(48000, 0);
        assert!((r.step() - 1.0).abs() < f64::EPSILON);
    }
}
