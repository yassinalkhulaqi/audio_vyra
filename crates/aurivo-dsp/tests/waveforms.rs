//! Detailed DSP behaviour tests. These complement the property tests by
//! checking the shape and timing of each effect's response.

use aurivo_dsp::{
    Biquad, Compressor, Delay, DelayParams, Distortion, DistortionParams, Gain, LimiterParams,
    Modulation, ModulationParams, NoiseGate, PeakLimiter, PitchShifter, Reverb, ReverbParams,
    SpectrumAnalyzer, Vocal, VocalParams,
};

const SR: u32 = 48000;

fn impulse(len: usize) -> Vec<f32> {
    let mut buf = vec![0.0f32; len];
    buf[0] = 1.0;
    buf
}

fn rms(buf: &[f32]) -> f32 {
    if buf.is_empty() { return 0.0; }
    (buf.iter().map(|v| v * v).sum::<f32>() / buf.len() as f32).sqrt()
}

fn peak(buf: &[f32]) -> f32 {
    buf.iter().fold(0.0f32, |m, v| m.max(v.abs()))
}

fn sine(freq: f32, len: usize, sr: u32) -> Vec<f32> {
    (0..len).map(|i| (std::f32::consts::TAU * freq * i as f32 / sr as f32).sin()).collect()
}

#[test]
fn gain_applies_exact_db() {
    for db in [-24.0, -6.0, 0.0, 6.0, 24.0] {
        let mut gain = Gain::new(db);
        let mut buf = vec![0.5f32; 64];
        gain.process(&mut buf);
        let expected = 0.5 * 10f32.powf(db / 20.0);
        assert!((buf[0] - expected).abs() < 1e-5, "{db} dB: {} != {}", buf[0], expected);
    }
}

#[test]
fn gain_shared_handle_updates_live() {
    let (mut gain, handle) = Gain::shared();
    Gain::set_gain_db_shared(&handle, -6.0);
    let mut buf = vec![1.0f32; 32];
    gain.process(&mut buf);
    assert!((buf[0] - 0.501187).abs() < 1e-4);
}

#[test]
fn highpass_rejects_low_frequency() {
    let mut filter = Biquad::new_highpass(1000.0, 0.707, SR as f32);
    let low = sine(60.0, SR as usize, SR);
    let high = sine(5000.0, SR as usize, SR);
    let mut a = low.clone();
    let mut b = high.clone();
    filter.process(&mut a);
    let mut filter = Biquad::new_highpass(1000.0, 0.707, SR as f32);
    filter.process(&mut b);
    assert!(rms(&a[SR as usize / 2..]) < rms(&low[SR as usize / 2..]) * 0.2);
    assert!(rms(&b[SR as usize / 2..]) > rms(&high[SR as usize / 2..]) * 0.7);
}

#[test]
fn lowpass_rejects_high_frequency() {
    let mut filter = Biquad::new_lowpass(1000.0, 0.707, SR as f32);
    let high = sine(8000.0, SR as usize, SR);
    let mut out = high.clone();
    filter.process(&mut out);
    assert!(rms(&out[SR as usize / 2..]) < rms(&high[SR as usize / 2..]) * 0.2);
}

#[test]
fn gate_envelope_opens_and_closes() {
    let mut gate = NoiseGate::new(-40.0, 1.0, 20.0, SR as f32);
    let mut quiet = vec![0.001f32; 1000];
    gate.process(&mut quiet);
    assert!(quiet.iter().all(|v| *v == 0.0));
    let mut loud = vec![0.5f32; 1000];
    gate.process(&mut loud);
    assert!(peak(&loud[500..]) > 0.4);
}

#[test]
fn compressor_ratio_changes_gain_reduction() {
    let get_output = |ratio| {
        let mut comp = Compressor::new(-24.0, ratio, 1.0, 20.0, 0.0, SR as f32);
        let mut buf = vec![0.8f32; 4800];
        comp.process(&mut buf);
        rms(&buf[4000..])
    };
    assert!(get_output(8.0) < get_output(2.0));
}

#[test]
fn delay_echo_lands_at_requested_time() {
    let (mut delay, handle) = Delay::shared(SR);
    Delay::update(&handle, DelayParams { time_ms: 100.0, feedback: 0.0, mix: 0.5, damp: 0.0 });
    let mut buf = impulse(SR as usize);
    delay.process(&mut buf);
    assert!(buf[0] > 0.4, "dry sample {}", buf[0]);
    assert!(buf[4790..4810].iter().any(|v| v.abs() > 0.1), "echo missing");
    assert!(buf[2400..2420].iter().all(|v| v.abs() < 1e-6), "unexpected early echo");
}

#[test]
fn delay_feedback_decays() {
    let (mut delay, handle) = Delay::shared(SR);
    Delay::update(&handle, DelayParams { time_ms: 50.0, feedback: 0.7, mix: 0.5, damp: 0.0 });
    let mut buf = impulse(SR as usize);
    delay.process(&mut buf);
    let first = peak(&buf[2300..2600]);
    let later = peak(&buf[7000..7300]);
    assert!(first > later);
}

#[test]
fn reverb_has_a_tail() {
    let (mut reverb, _) = Reverb::shared(SR);
    let mut buf = impulse(SR as usize);
    reverb.process(&mut buf);
    let early = rms(&buf[100..2000]);
    let late = rms(&buf[16000..18000]);
    assert!(early > late);
    assert!(late > 0.0);
}

#[test]
fn reverb_room_size_changes_tail_length() {
    let tail = |room| {
        let (mut reverb, handle) = Reverb::shared(SR);
        Reverb::update(&handle, ReverbParams { room_size: room, damp: 0.0, wet: 1.0, dry: 0.0 });
        let mut buf = impulse(SR as usize * 2);
        reverb.process(&mut buf);
        rms(&buf[SR as usize..])
    };
    assert!(tail(0.9) > tail(0.1));
}

#[test]
fn distortion_modes_are_bounded() {
    for mode in 0..=2 {
        let (mut dist, handle) = Distortion::shared(SR);
        Distortion::update(&handle, DistortionParams { mode, drive_db: 36.0, mix: 1.0, ..DistortionParams::default() });
        let mut buf = vec![2.0f32; 512];
        dist.process(&mut buf);
        assert!(peak(&buf) < 8.0, "mode {mode} peak {}", peak(&buf));
    }
}

#[test]
fn distortion_bitcrush_reduces_unique_values() {
    let (mut dist, handle) = Distortion::shared(SR);
    Distortion::update(&handle, DistortionParams {
        mode: 0, drive_db: 0.0, tone: 1.0, mix: 1.0, downsample: 8.0, bit_depth: 4.0,
    });
    let mut buf: Vec<f32> = (0..512).map(|i| i as f32 / 512.0).collect();
    dist.process(&mut buf);
    let unique: std::collections::HashSet<u32> = buf.iter().map(|v| v.to_bits()).collect();
    assert!(unique.len() < buf.len() / 4);
}

#[test]
fn modulation_modes_produce_output() {
    for mode in 0..=3 {
        let (mut mod_fx, handle) = Modulation::shared(SR);
        Modulation::update(&handle, ModulationParams { mode, rate_hz: 2.0, depth: 0.7, feedback: 0.3, mix: 1.0 });
        let mut buf = sine(440.0, 4096, SR);
        mod_fx.process(&mut buf);
        assert!(peak(&buf) > 0.01, "mode {mode} produced silence");
    }
}

#[test]
fn pitch_shifter_changes_zero_crossing_rate() {
    let count_crossings = |buf: &[f32]| -> usize {
        buf.windows(2).filter(|w| w[0].signum() != w[1].signum()).count()
    };
    let input = sine(440.0, SR as usize, SR);
    let (mut pitch, handle) = PitchShifter::shared(SR);
    PitchShifter::set_semitones_shared(&handle, 12.0);
    let mut out = input.clone();
    pitch.process(&mut out);
    let a = count_crossings(&input[SR as usize / 4..SR as usize / 2]);
    let b = count_crossings(&out[SR as usize / 4..SR as usize / 2]);
    assert!(b > a, "expected more crossings after +12 st: {a} vs {b}");
}

#[test]
fn spectrum_finds_dominant_bin() {
    let (mut analyzer, handle) = SpectrumAnalyzer::shared(2048);
    let buf = sine(1000.0, 8192, SR);
    analyzer.push(&buf);
    let bins = SpectrumAnalyzer::snapshot(&handle);
    let expected_bin = (1000.0 * 2048.0 / SR as f32) as usize;
    let best = bins.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).unwrap().0;
    assert!((best as i32 - expected_bin as i32).abs() <= 4, "best {best}, expected {expected_bin}");
}

#[test]
fn vocal_modes_are_distinct() {
    let mut outputs = Vec::new();
    for mode in 0..=3 {
        let (mut vocal, handle) = Vocal::shared(SR);
        Vocal::update(&handle, VocalParams { mode, carrier_hz: 80.0, vowel: 2.0, depth: 0.5, mix: 1.0 });
        let mut buf = sine(220.0, 2048, SR);
        vocal.process(&mut buf);
        outputs.push(buf);
    }
    for pair in outputs.windows(2) {
        assert!(rms(&pair[0][512..]) != rms(&pair[1][512..]));
    }
}

#[test]
fn limiter_ceiling_is_never_exceeded() {
    let mut limiter = PeakLimiter::with_params(
        SR,
        LimiterParams { threshold_db: -3.0, attack_ms: 1.0, release_ms: 100.0, ceiling: 0.9 },
    );
    let mut buf: Vec<f32> = (0..SR as usize).map(|i| (i as f32 * 0.01).sin() * 4.0).collect();
    limiter.process(&mut buf);
    assert!(peak(&buf) <= 0.9001);
}
