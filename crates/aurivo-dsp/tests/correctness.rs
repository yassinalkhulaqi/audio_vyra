use aurivo_dsp::{Compressor, LimiterParams, NoiseGate, PeakLimiter};

fn tone(amplitude: f32, samples: usize) -> Vec<f32> {
    (0..samples)
        .map(|i| amplitude * (i as f32 * 0.02).sin())
        .collect()
}

#[test]
fn compressor_survives_zero_ratio() {
    let mut comp = Compressor::new(-18.0, 0.0, 10.0, 100.0, 2.0, 48000.0);
    let mut buf = tone(0.9, 1024);
    comp.process(&mut buf);
    assert!(buf.iter().all(|v| v.is_finite()));
}

#[test]
fn compressor_survives_negative_time_constants() {
    let mut comp = Compressor::new(-18.0, 4.0, -10.0, -100.0, 2.0, 48000.0);
    let mut buf = tone(0.9, 1024);
    comp.process(&mut buf);
    assert!(buf.iter().all(|v| v.is_finite()));
}

#[test]
fn compressor_ratio_is_enforced() {
    let mut comp = Compressor::new(-24.0, 8.0, 1.0, 50.0, 0.0, 48000.0);
    let mut loud = vec![0.95f32; 4800];
    comp.process(&mut loud);
    let steady = &loud[loud.len() - 256..];
    let out_peak = steady.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    assert!(out_peak < 0.6, "steady peak after compression: {out_peak}");
}

#[test]
fn gate_survives_hostile_params() {
    let mut gate = NoiseGate::new(f32::NAN, -5.0, f32::INFINITY, 48000.0);
    let mut buf = tone(0.5, 2048);
    gate.process(&mut buf);
    assert!(buf.iter().all(|v| v.is_finite()));
}

#[test]
fn gate_silences_below_threshold() {
    let mut gate = NoiseGate::new(-40.0, 1.0, 20.0, 48000.0);
    let mut buf = vec![1e-4f32; 2048];
    gate.process(&mut buf);
    assert!(buf.iter().all(|v| *v == 0.0));
}

#[test]
fn very_low_sample_rate_does_not_panic() {
    let mut gate = NoiseGate::new(-40.0, 1.0, 20.0, 16.0);
    let mut buf = vec![0.5f32; 16];
    gate.process(&mut buf);
    let mut comp = Compressor::new(-18.0, 3.0, 1.0, 20.0, 2.0, 16.0);
    comp.process(&mut buf);
    assert!(buf.iter().all(|v| v.is_finite()));
}

#[test]
fn limiter_ceiling_is_respected_across_many_blocks() {
    let mut limiter = PeakLimiter::new(48000);
    for _ in 0..50 {
        let mut buf = vec![4.0f32; 256];
        limiter.process(&mut buf);
        assert!(buf.iter().all(|v| v.abs() <= 0.99));
    }
}

#[test]
fn limiter_does_not_pump_on_silence() {
    let mut limiter = PeakLimiter::with_params(48000, LimiterParams {
        threshold_db: -6.0,
        attack_ms: 1.0,
        release_ms: 50.0,
        ceiling: 1.0,
    });
    let mut loud = vec![2.0f32; 1024];
    limiter.process(&mut loud);
    let mut silent = vec![0.0f32; 48000];
    limiter.process(&mut silent);
    assert!(limiter.gain_reduction_db() > -0.2);
}

#[test]
fn denormals_do_not_accumulate_in_feedback_loops() {
    let mut limiter = PeakLimiter::new(48000);
    let mut buf = vec![1e-30f32; 4096];
    limiter.process(&mut buf);
    assert!(buf.iter().all(|v| v.is_finite()));
    assert!(buf.iter().all(|v| v.abs() == 0.0 || v.abs() >= 1e-37));
}

#[test]
fn nan_input_does_not_poison_further_blocks() {
    let mut gate = NoiseGate::new(-40.0, 5.0, 50.0, 48000.0);
    let mut bad = vec![f32::NAN; 64];
    gate.process(&mut bad);
    let mut good = vec![0.5f32; 1024];
    gate.process(&mut good);
    assert!(good.iter().all(|v| v.is_finite()));
}

#[test]
fn output_stays_bounded_with_extreme_makeup() {
    let mut comp = Compressor::new(-30.0, 20.0, 1.0, 30.0, 36.0, 48000.0);
    let mut limiter = PeakLimiter::new(48000);
    let mut buf = tone(0.8, 8192);
    comp.process(&mut buf);
    limiter.process(&mut buf);
    assert!(buf.iter().all(|v| v.abs() <= 1.0));
}
