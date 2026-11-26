//! Control-rate smoothing for parameters that would otherwise click when
//! changed mid-block.
//!
//! Two strategies are provided:
//!
//! - [`SmoothedValue`] — exponential one-pole smoothing, appropriate for
//!   continuous controls such as gain and mix.
//! - [`Ramp`] — fixed-length linear ramp, appropriate for one-shot changes
//!   that should reach their target in a known time.
//!
//! Both are allocation-free and safe to call per sample in an audio callback.

#[derive(Debug, Clone)]
pub struct SmoothedValue {
    current: f32,
    target: f32,
    coefficient: f32,
}

impl SmoothedValue {
    pub fn new(value: f32, sample_rate: u32, time_ms: f32) -> Self {
        let mut result = Self { current: value, target: value, coefficient: 0.0 };
        result.set_smoothing(sample_rate, time_ms);
        result
    }

    pub fn set_smoothing(&mut self, sample_rate: u32, time_ms: f32) {
        let ms = if time_ms.is_finite() { time_ms.max(0.0) } else { 0.0 };
        let sr = sample_rate.max(1) as f32;
        self.coefficient = if ms <= 0.0 { 0.0 } else { (-1.0 / (sr * ms / 1000.0)).exp() };
    }

    pub fn current(&self) -> f32 { self.current }
    pub fn target(&self) -> f32 { self.target }

    pub fn set_target(&mut self, target: f32) {
        let target = if target.is_finite() { target } else { self.current };
        if target != self.target {
            self.target = target;
        }
    }

    pub fn jump_to(&mut self, value: f32) {
        let value = if value.is_finite() { value } else { self.current };
        self.current = value;
        self.target = value;
    }

    pub fn next(&mut self) -> f32 {
        self.current += (self.target - self.current) * (1.0 - self.coefficient);
        if (self.current - self.target).abs() < 1e-7 {
            self.current = self.target;
        }
        self.current
    }

    pub fn process_block(&mut self, samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            let gain = self.next();
            *sample *= gain;
        }
    }

    pub fn settled(&self) -> bool {
        (self.current - self.target).abs() < 1e-4
    }
}

#[derive(Debug, Clone)]
pub struct Ramp {
    start: f32,
    target: f32,
    length: u32,
    position: u32,
}

impl Ramp {
    pub fn new(value: f32, length: u32) -> Self {
        Self { start: value, target: value, length, position: 0 }
    }

    pub fn retarget(&mut self, target: f32) {
        let target = if target.is_finite() { target } else { self.target };
        if target == self.target {
            return;
        }
        self.start = self.current();
        self.target = target;
        self.position = 0;
    }

    pub fn current(&self) -> f32 {
        if self.length == 0 || self.position >= self.length {
            return self.target;
        }
        let progress = self.position as f32 / self.length as f32;
        self.start + (self.target - self.start) * progress
    }

    pub fn next(&mut self) -> f32 {
        let value = self.current();
        if self.position < self.length {
            self.position += 1;
        }
        value
    }

    pub fn is_done(&self) -> bool { self.position >= self.length }
}

/// Gain processor that ramps the linear gain instead of stepping it, which
/// removes the zipper noise heard when dragging the gain slider quickly.
#[derive(Debug, Clone)]
pub struct SmoothedGain {
    inner: crate::Gain,
    smooth: SmoothedValue,
    bypass: bool,
}

impl SmoothedGain {
    pub fn new(sample_rate: u32, time_ms: f32) -> Self {
        let inner = crate::Gain::new(0.0);
        Self {
            smooth: SmoothedValue::new(1.0, sample_rate, time_ms),
            inner,
            bypass: false,
        }
    }

    pub fn shared(sample_rate: u32) -> (Self, std::sync::Arc<std::sync::atomic::AtomicU32>) {
        let (inner, handle) = crate::Gain::shared();
        let mut smooth = SmoothedValue::new(1.0, sample_rate, 5.0);
        let target = 10f32.powf(inner.gain_db() / 20.0);
        smooth.jump_to(target);
        (Self { inner, smooth, bypass: false }, handle)
    }

    pub fn set_bypass(&mut self, bypass: bool) { self.bypass = bypass; }
    pub fn is_bypassed(&self) -> bool { self.bypass }

    pub fn process(&mut self, samples: &mut [f32]) {
        if self.bypass {
            return;
        }
        let target = 10f32.powf(self.inner.gain_db() / 20.0);
        self.smooth.set_target(target);
        self.smooth.process_block(samples);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instant_smoothing_reaches_target_immediately() {
        let mut value = SmoothedValue::new(0.0, 48000, 0.0);
        value.set_target(1.0);
        assert_eq!(value.next(), 1.0);
    }

    #[test]
    fn smoothing_never_overshoots() {
        let mut value = SmoothedValue::new(0.0, 48000, 10.0);
        value.set_target(1.0);
        let mut previous = value.current();
        for _ in 0..2000 {
            let next = value.next();
            assert!(next >= previous);
            assert!(next <= 1.0);
            previous = next;
        }
    }

    #[test]
    fn smoothing_settles() {
        let mut value = SmoothedValue::new(0.0, 48000, 5.0);
        value.set_target(0.5);
        for _ in 0..5000 { value.next(); }
        assert!(value.settled());
    }

    #[test]
    fn hostile_target_is_ignored() {
        let mut value = SmoothedValue::new(0.25, 48000, 1.0);
        value.set_target(f32::NAN);
        assert_eq!(value.target(), 0.25);
        value.jump_to(f32::INFINITY);
        assert_eq!(value.current(), 0.25);
    }

    #[test]
    fn block_processing_multiplies_samples() {
        let mut value = SmoothedValue::new(0.5, 48000, 0.0);
        let mut buf = [1.0f32; 8];
        value.process_block(&mut buf);
        assert!(buf.iter().all(|v| (v - 0.5).abs() < 1e-6));
    }

    #[test]
    fn ramp_linear_shape() {
        let mut ramp = Ramp::new(0.0, 4);
        ramp.retarget(1.0);
        let values: Vec<f32> = (0..4).map(|_| ramp.next()).collect();
        assert!((values[0] - 0.0).abs() < 1e-6);
        assert!((values[1] - 0.25).abs() < 1e-6);
        assert!((values[3] - 0.75).abs() < 1e-6);
        assert!((ramp.next() - 1.0).abs() < 1e-6);
        assert!(ramp.is_done());
    }

    #[test]
    fn ramp_retarget_starts_from_current() {
        let mut ramp = Ramp::new(0.0, 8);
        ramp.retarget(1.0);
        ramp.next();
        ramp.next();
        ramp.retarget(0.5);
        assert!(ramp.current() > 0.0 && ramp.current() < 0.5);
    }

    #[test]
    fn zero_length_ramp_is_instant() {
        let mut ramp = Ramp::new(1.0, 0);
        ramp.retarget(0.0);
        assert_eq!(ramp.next(), 0.0);
    }
}
