//! Unit tests for every DSP processor: silence safety, finite output, bypass,
//! and characteristic behaviour of each effect family.

use aurivo_dsp::*;
use std::sync::Arc;

fn sine(sr: u32, hz: f32, n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| (2.0 * std::f32::consts::PI * hz * i as f32 / sr as f32).sin() * 0.5)
        .collect()
}

fn assert_finite(buf: &[f32]) {
    for &s in buf {
        assert!(s.is_finite(), "non-finite sample in output");
    }
}

#[test]
fn gain_amplifies_and_bypasses() {
    let mut g = Gain::new(6.0);
    let mut buf = vec![0.5f32; 64];
    g.process(&mut buf);
    assert!((buf[0] - 0.5 * 10f32.powf(6.0 / 20.0)).abs() < 1e-5);
    g.set_bypass(true);
    let before = buf[0];
    g.process(&mut buf);
    assert_eq!(buf[0], before);
}

#[test]
fn meter_peak_rms() {
    let m = LevelMeterProcessor::new();
    let buf = vec![0.5f32; 64];
    m.process(&buf);
    assert!((m.peak() - 0.5).abs() < 1e-6);
    assert!((m.rms() - 0.5).abs() < 1e-6);
    m.process(&[1.2f32]);
    assert!((m.peak() - 1.2).abs() < 1e-6);
}

#[test]
fn noise_gate_silences_quiet_signal() {
    let mut gate = NoiseGate::new(-20.0, 1.0, 50.0, 48000.0);
    let mut buf = vec![0.0001f32; 4096];
    gate.process(&mut buf);
    assert!(buf.iter().all(|&s| s == 0.0));
}

#[test]
fn compressor_tames_loud_signal() {
    let mut comp = Compressor::new(-30.0, 8.0, 1.0, 100.0, 0.0, 48000.0);
    let mut buf = vec![0.9f32; 4096];
    comp.process(&mut buf);
    assert_finite(&buf);
    assert!(buf[buf.len() - 1] < 0.9);
}

#[test]
fn highpass_attenuates_low_frequency() {
    let mut hp = Biquad::new_highpass(1000.0, 0.707, 48000.0);
    let mut low = sine(48000, 60.0, 4096);
    hp.process(&mut low);
    let mut hp2 = Biquad::new_highpass(1000.0, 0.707, 48000.0);
    let mut high = sine(48000, 5000.0, 4096);
    hp2.process(&mut high);
    let rms = |b: &[f32]| (b.iter().map(|s| s * s).sum::<f32>() / b.len() as f32).sqrt();
    assert!(rms(&low) < rms(&high) * 0.5);
}

#[test]
fn delay_produces_echo_tail() {
    let (mut d, h) = Delay::shared(48000);
    Delay::update(&h, DelayParams { time_ms: 5.0, feedback: 0.6, mix: 1.0, damp: 0.0 });
    let mut buf = vec![0.0f32; 2048];
    buf[0] = 1.0;
    d.process(&mut buf);
    assert_finite(&buf);
    assert!(buf.iter().skip(300).any(|&s| s.abs() > 1e-4), "no echo tail");
}

#[test]
fn reverb_produces_tail() {
    let mut r = Reverb::new(48000);
    let mut buf = vec![0.0f32; 8192];
    buf[0] = 1.0;
    r.process(&mut buf);
    assert_finite(&buf);
    assert!(buf.iter().skip(1000).any(|&s| s.abs() > 1e-5), "no reverb tail");
}

#[test]
fn distortion_stays_bounded() {
    let (mut fx, h) = Distortion::shared(48000);
    Distortion::update(&h, DistortionParams { mode: 1, drive_db: 24.0, tone: 0.5, mix: 1.0, downsample: 1.0, bit_depth: 16.0 });
    let mut buf = vec![0.95f32; 1024];
    fx.process(&mut buf);
    assert_finite(&buf);
    assert!(buf.iter().all(|&s| s.abs() <= 8.0));
}

#[test]
fn modulation_changes_signal() {
    let (mut fx, h) = Modulation::shared(48000);
    Modulation::update(&h, ModulationParams { mode: 3, rate_hz: 2.0, depth: 0.8, feedback: 0.0, mix: 1.0 });
    let mut buf = sine(48000, 440.0, 4096);
    let original = buf.clone();
    fx.process(&mut buf);
    assert_finite(&buf);
    assert!(buf.iter().zip(original.iter()).any(|(a, b)| (a - b).abs() > 1e-3));
}

#[test]
fn eq_bands_can_be_added_edited_removed() {
    let (mut fx, h) = ParametricEq::shared(48000);
    let initial = aurivo_dsp::eq::snapshot(&h).len();
    let band = EqBand { kind: EqFilterKind::Peaking, frequency: 1000.0, q: 1.0, gain_db: 6.0, enabled: true };
    aurivo_dsp::eq::add_band(&h, band);
    assert_eq!(aurivo_dsp::eq::snapshot(&h).len(), initial + 1);
    aurivo_dsp::eq::set_band(&h, 0, EqBand { kind: EqFilterKind::Peaking, frequency: 1000.0, q: 1.0, gain_db: -3.0, enabled: true });
    assert!((aurivo_dsp::eq::snapshot(&h)[0].gain_db + 3.0).abs() < 1e-6);
    let mut buf = sine(48000, 1000.0, 2048);
    fx.process(&mut buf);
    assert_finite(&buf);
    aurivo_dsp::eq::remove_band(&h, 0);
    assert_eq!(aurivo_dsp::eq::snapshot(&h).len(), initial);
}

#[test]
fn pitch_shifter_handles_extreme_semitones() {
    let (mut fx, h) = PitchShifter::shared(48000);
    PitchShifter::set_semitones_shared(&h, -24.0);
    let mut buf = sine(48000, 220.0, 8192);
    fx.process(&mut buf);
    assert_finite(&buf);
    PitchShifter::set_semitones_shared(&h, 24.0);
    fx.process(&mut buf);
    assert_finite(&buf);
}

#[test]
fn spectrum_reports_bins_for_tone() {
    let (mut an, h) = SpectrumAnalyzer::shared(1024);
    an.push(&sine(48000, 1000.0, 4096));
    let bins = SpectrumAnalyzer::snapshot(&h);
    assert!(!bins.is_empty());
    assert!(bins.iter().all(|&v| v.is_finite()));
}

#[test]
fn pipeline_runs_all_processors() {
    let mut p = ProcessingPipeline::new(48000);
    p.add(Box::new(Gain::new(6.0)));
    p.add(Box::new(Delay::new(48000)));
    assert_eq!(p.len(), 2);
    let mut buf = vec![0.5f32; 256];
    p.process(&mut buf);
    assert_finite(&buf);
}

#[test]
fn shared_handles_are_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Arc<std::sync::Mutex<DelayParams>>>();
    assert_send_sync::<Arc<std::sync::atomic::AtomicU32>>();
}
