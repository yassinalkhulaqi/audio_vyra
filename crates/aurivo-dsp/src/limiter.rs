//! Lightweight peak limiter for the master output.
//!
//! A feed-forward envelope detector tracks the recent peak. When the signal
//! exceeds the ceiling the processor applies a smooth gain reduction with a
//! short attack and a longer release. It is not a brick-wall look-ahead
//! limiter, but it prevents the obvious clipping introduced by reverb tails,
//! resonance and stacked distortion without adding latency.

use crate::denormal::flush;
use crate::validate::{safe_db, safe_ms, safe_unit};

#[derive(Debug, Clone)]
pub struct LimiterParams {
    pub threshold_db: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub ceiling: f32,
}

impl Default for LimiterParams {
    fn default() -> Self {
        Self {
            threshold_db: -3.0,
            attack_ms: 2.0,
            release_ms: 120.0,
            ceiling: 0.98,
        }
    }
}

#[derive(Debug)]
pub struct PeakLimiter {
    params: LimiterParams,
    envelope: f32,
    gain: f32,
    sample_rate: f32,
    bypass: bool,
}

impl PeakLimiter {
    pub fn new(sample_rate: u32) -> Self {
        Self::with_params(sample_rate, LimiterParams::default())
    }

    pub fn with_params(sample_rate: u32, params: LimiterParams) -> Self {
        Self {
            params,
            envelope: 0.0,
            gain: 1.0,
            sample_rate: sample_rate.max(1) as f32,
            bypass: false,
        }
    }

    pub fn set_params(&mut self, params: LimiterParams) {
        self.params = params;
    }

    pub fn params(&self) -> &LimiterParams {
        &self.params
    }

    pub fn set_bypass(&mut self, bypass: bool) {
        self.bypass = bypass;
    }

    pub fn is_bypassed(&self) -> bool {
        self.bypass
    }

    pub fn reset(&mut self) {
        self.envelope = 0.0;
        self.gain = 1.0;
    }

    pub fn gain_reduction_db(&self) -> f32 {
        20.0 * self.gain.max(1e-6).log10()
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        if self.bypass {
            return;
        }
        let threshold = 10f32.powf(safe_db(self.params.threshold_db) / 20.0);
        let ceiling = safe_unit(self.params.ceiling).clamp(0.1, 1.0);
        let attack = envelope_coefficient(self.sample_rate, self.params.attack_ms);
        let release = envelope_coefficient(self.sample_rate, self.params.release_ms);

        for sample in samples.iter_mut() {
            let input = *sample;
            let level = input.abs();
            let coeff = if level > self.envelope { attack } else { release };
            self.envelope = flush(coeff * self.envelope + (1.0 - coeff) * level);

            let target = if self.envelope > threshold {
                threshold / self.envelope
            } else {
                1.0
            };
            let smoothing = if target < self.gain { attack } else { release };
            self.gain = smoothing * self.gain + (1.0 - smoothing) * target;
            self.gain = flush(self.gain);

            let limited = input * self.gain;
            *sample = limited.clamp(-ceiling, ceiling);
        }
    }
}

fn envelope_coefficient(sample_rate: f32, ms: f32) -> f32 {
    let ms = safe_ms(ms);
    if ms <= 0.0 { 0.0 } else { (-1.0 / (sample_rate * ms / 1000.0)).exp() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(buf: &[f32]) -> f32 {
        buf.iter().fold(0.0f32, |m, v| m.max(v.abs()))
    }

    #[test]
    fn limits_loud_signal() {
        let mut limiter = PeakLimiter::new(48000);
        let mut buf = vec![2.0f32; 4800];
        limiter.process(&mut buf);
        assert!(peak(&buf) <= 0.99);
    }

    #[test]
    fn leaves_quiet_signal_alone() {
        let mut limiter = PeakLimiter::new(48000);
        let mut buf = vec![0.05f32; 1024];
        limiter.process(&mut buf);
        assert!((buf[1023] - 0.05).abs() < 1e-4);
    }

    #[test]
    fn releases_after_loud_transient() {
        let mut limiter = PeakLimiter::new(48000);
        let mut loud = vec![3.0f32; 480];
        limiter.process(&mut loud);
        assert!(limiter.gain_reduction_db() < 0.0);
        let mut quiet = vec![0.0f32; 24000];
        limiter.process(&mut quiet);
        assert!(limiter.gain_reduction_db() > -0.5);
    }

    #[test]
    fn bypass_is_transparent() {
        let mut limiter = PeakLimiter::new(48000);
        limiter.set_bypass(true);
        let mut buf = vec![3.0f32; 64];
        let before = buf.clone();
        limiter.process(&mut buf);
        assert_eq!(buf, before);
    }

    #[test]
    fn hostile_parameters_stay_finite() {
        let mut limiter = PeakLimiter::with_params(
            48000,
            LimiterParams {
                threshold_db: f32::NAN,
                attack_ms: -1.0,
                release_ms: f32::INFINITY,
                ceiling: f32::NAN,
            },
        );
        let mut buf = vec![f32::MAX / 4.0; 32];
        limiter.process(&mut buf);
        assert!(buf.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn reset_restores_unity_gain() {
        let mut limiter = PeakLimiter::new(48000);
        let mut buf = vec![4.0f32; 1000];
        limiter.process(&mut buf);
        limiter.reset();
        assert_eq!(limiter.gain_reduction_db(), 0.0);
    }

    #[test]
    fn many_blocks_do_not_drift() {
        let mut limiter = PeakLimiter::new(48000);
        for _ in 0..100 {
            let mut buf = vec![0.2f32; 128];
            limiter.process(&mut buf);
            assert!(buf.iter().all(|v| v.is_finite()));
        }
    }
}
