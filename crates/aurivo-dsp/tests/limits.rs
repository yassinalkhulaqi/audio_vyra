//! Numeric edge-case tests for DSP math at the limits of f32.

use aurivo_dsp::{
    Biquad, Compressor, Delay, DelayParams, LimiterParams, NoiseGate, PeakLimiter,
    PitchShifter, SmoothedGain, Vocal, VocalParams,
};

const SR: u32 = 48000;

fn finite(buf: &[f32]) -> bool { buf.iter().all(|v| v.is_finite()) }

#[test]
fn compressor_at_maximum_ratio_and_makeup() {
    let mut comp = Compressor::new(-60.0, 100.0, 0.0, 0.0, 36.0, SR as f32);
    let mut buf = vec![0.999f32; 4096];
    comp.process(&mut buf);
    assert!(finite(&buf));
}

#[test]
fn gate_at_nyquist_extreme_times() {
    let mut gate = NoiseGate::new(-120.0, 5000.0, 5000.0, 1.0);
    let mut buf = vec![0.5f32; 64];
    gate.process(&mut buf);
    assert!(finite(&buf));
}

#[test]
fn biquad_at_low_and_high_sample_rates() {
    for sr in [1u32, 8, 64, 1000, 192000] {
        let mut filter = Biquad::new_highpass(80.0, 0.707, sr as f32);
        let mut buf = vec![0.5f32; 64];
        filter.process(&mut buf);
        assert!(finite(&buf), "sr {sr} produced non-finite output");
    }
}

#[test]
fn delay_at_maximum_time() {
    let (mut delay, handle) = Delay::shared(SR);
    Delay::update(&handle, DelayParams { time_ms: 5000.0, feedback: 0.98, mix: 1.0, damp: 1.0 });
    let mut buf = vec![0.5f32; SR as usize];
    delay.process(&mut buf);
    assert!(finite(&buf));
}

#[test]
fn pitch_at_extreme_semitones_with_silence() {
    for semi in [-24.0, 0.0, 24.0] {
        let (mut pitch, handle) = PitchShifter::shared(SR);
        PitchShifter::set_semitones_shared(&handle, semi);
        let mut buf = vec![0.0f32; 4096];
        pitch.process(&mut buf);
        assert!(buf.iter().all(|v| *v == 0.0));
    }
}

#[test]
fn vocal_at_extreme_vowel_indices() {
    for vowel in [-10.0, 0.0, 2.5, 4.0, 10.0] {
        let (mut vocal, handle) = Vocal::shared(SR);
        Vocal::update(&handle, VocalParams { mode: 2, vowel, mix: 1.0, ..VocalParams::default() });
        let mut buf = vec![0.3f32; 1024];
        vocal.process(&mut buf);
        assert!(finite(&buf), "vowel {vowel} produced non-finite output");
    }
}

#[test]
fn limiter_with_zero_attack_and_release() {
    let mut limiter = PeakLimiter::with_params(
        SR,
        LimiterParams { threshold_db: -60.0, attack_ms: 0.0, release_ms: 0.0, ceiling: 0.5 },
    );
    let mut buf = vec![100.0f32; 4096];
    limiter.process(&mut buf);
    assert!(finite(&buf));
    assert!(buf.iter().all(|v| v.abs() <= 0.5001));
}

#[test]
fn smoothed_gain_with_extreme_db() {
    let mut gain = SmoothedGain::new(1, 0.0);
    let mut buf = vec![1.0f32; 32];
    gain.process(&mut buf);
    assert!(finite(&buf));
}

#[test]
fn repeated_reset_keeps_state_clean() {
    let (mut delay, _) = Delay::shared(SR);
    for _ in 0..100 {
        delay.reset();
        let mut buf = vec![0.25f32; 64];
        delay.process(&mut buf);
        assert!(finite(&buf));
    }
}

#[test]
fn empty_slices_never_panic() {
    let (mut delay, _) = Delay::shared(SR);
    let (mut vocal, _) = Vocal::shared(SR);
    let mut comp = Compressor::new(-18.0, 3.0, 1.0, 10.0, 0.0, SR as f32);
    let mut empty: [f32; 0] = [];
    delay.process(&mut empty);
    vocal.process(&mut empty);
    comp.process(&mut empty);
}
