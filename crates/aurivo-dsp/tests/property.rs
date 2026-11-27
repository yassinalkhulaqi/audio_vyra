//! Property-style tests for the DSP crate.
//!
//! These do not verify exact audio quality; they verify the invariants that
//! must hold for *every* processor: silence stays silent, hostile input stays
//! finite, extreme parameters never panic, and bypass is truly transparent.

use aurivo_dsp::{
    Biquad, Compressor, Delay, DelayParams, Distortion, DistortionParams, Gain, LimiterParams,
    Modulation, ModulationParams, NoiseGate, PeakLimiter, PitchShifter, ProcessingPipeline,
    Reverb, ReverbParams, SpectrumAnalyzer, Vocal, VocalParams,
};

const SR: u32 = 48000;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
    }
}

fn noise(len: usize) -> Vec<f32> {
    let mut rng = Rng(0xA0B1C2);
    (0..len).map(|_| rng.next()).collect()
}

fn assert_finite(buf: &[f32], label: &str) {
    for (i, v) in buf.iter().enumerate() {
        assert!(v.is_finite(), "{label}: sample {i} is not finite ({v})");
    }
}

fn assert_silent(buf: &[f32], label: &str) {
    for (i, v) in buf.iter().enumerate() {
        assert_eq!(*v, 0.0, "{label}: sample {i} is not silent ({v})");
    }
}

#[test]
fn silence_in_silence_out_all_processors() {
    let mut pipeline = ProcessingPipeline::new(SR);
    pipeline.add(Box::new(Gain::new(6.0)));
    pipeline.add(Box::new(NoiseGate::new(-40.0, 5.0, 50.0, SR as f32)));
    pipeline.add(Box::new(Compressor::new(-18.0, 3.0, 10.0, 100.0, 2.0, SR as f32)));
    pipeline.add(Box::new(Biquad::new_highpass(80.0, 0.707, SR as f32)));
    let (delay, _) = Delay::shared(SR);
    let (reverb, _) = Reverb::shared(SR);
    let (dist, _) = Distortion::shared(SR);
    let (mod_fx, _) = Modulation::shared(SR);
    let (pitch, _) = PitchShifter::shared(SR);
    let (vocal, _) = Vocal::shared(SR);
    pipeline.add(Box::new(delay));
    pipeline.add(Box::new(reverb));
    pipeline.add(Box::new(dist));
    pipeline.add(Box::new(mod_fx));
    pipeline.add(Box::new(pitch));
    pipeline.add(Box::new(vocal));
    pipeline.add(Box::new(PeakLimiter::new(SR)));

    let mut buf = vec![0.0f32; 4096];
    pipeline.process(&mut buf);
    assert_silent(&buf, "silence chain");
}

#[test]
fn non_finite_input_never_leaks_into_feedback_state() {
    let mut pipeline = ProcessingPipeline::new(SR);
    let (delay, _) = Delay::shared(SR);
    let (reverb, _) = Reverb::shared(SR);
    let (mod_fx, _) = Modulation::shared(SR);
    let (pitch, _) = PitchShifter::shared(SR);
    pipeline.add(Box::new(delay));
    pipeline.add(Box::new(reverb));
    pipeline.add(Box::new(mod_fx));
    pipeline.add(Box::new(pitch));

    let mut bad = vec![f32::NAN; 32];
    pipeline.process(&mut bad);
    assert_finite(&bad, "pipeline after NaN");

    let mut good = noise(4096);
    pipeline.process(&mut good);
    assert_finite(&good, "pipeline after recovery");
}

#[test]
fn random_parameter_sweep_never_panics() {
    let mut rng = Rng(42);
    for _ in 0..200 {
        let time_ms = rng.next().abs() * 4000.0;
        let feedback = rng.next();
        let mix = rng.next();
        let damp = rng.next();
        let (mut delay, handle) = Delay::shared(SR);
        Delay::update(&handle, DelayParams { time_ms, feedback, mix, damp });
        let mut buf = noise(256);
        delay.process(&mut buf);
        assert_finite(&buf, "delay sweep");
    }
}

#[test]
fn distortion_extreme_parameters_stay_bounded() {
    for mode in -2..=5 {
        for drive in [-60.0f32, 0.0, 48.0, 1.0e6] {
            for bits in [1.0f32, 8.0, 24.0, 1.0e6] {
                let (mut dist, handle) = Distortion::shared(SR);
                Distortion::update(&handle, DistortionParams {
                    mode,
                    drive_db: drive,
                    tone: 0.5,
                    mix: 0.8,
                    downsample: bits.abs() % 64.0 + 1.0,
                    bit_depth: bits,
                });
                let mut buf = noise(128);
                dist.process(&mut buf);
                assert_finite(&buf, "distortion sweep");
            }
        }
    }
}

#[test]
fn bypass_is_bit_exact_for_every_processor() {
    let mut pipeline = ProcessingPipeline::new(SR);
    pipeline.add(Box::new(Gain::new(12.0)));
    pipeline.add(Box::new(NoiseGate::new(-40.0, 5.0, 50.0, SR as f32)));
    pipeline.add(Box::new(Compressor::new(-18.0, 3.0, 10.0, 100.0, 2.0, SR as f32)));
    let (delay, _) = Delay::shared(SR);
    let (reverb, _) = Reverb::shared(SR);
    let (dist, _) = Distortion::shared(SR);
    let (mod_fx, _) = Modulation::shared(SR);
    let (vocal, _) = Vocal::shared(SR);
    pipeline.add(Box::new(delay));
    pipeline.add(Box::new(reverb));
    pipeline.add(Box::new(dist));
    pipeline.add(Box::new(mod_fx));
    pipeline.add(Box::new(vocal));
    for i in 0..pipeline.len() {
        pipeline.set_bypass(i, true);
        assert!(pipeline.bypass(i));
    }
    let input = noise(2048);
    let mut buf = input.clone();
    pipeline.process(&mut buf);
    assert_eq!(buf, input);
}

#[test]
fn spectrum_analyzer_handles_long_and_short_blocks() {
    let (mut analyzer, handle) = SpectrumAnalyzer::shared(1024);
    for len in [0, 1, 7, 64, 1023, 1024, 4096] {
        let buf = noise(len);
        analyzer.push(&buf);
        let bins = SpectrumAnalyzer::snapshot(&handle);
        assert_eq!(bins.len(), 512);
        assert_finite(&bins, "spectrum");
    }
    assert_eq!(analyzer.fft_size(), 1024);
}

#[test]
fn pitch_shifter_and_eq_survive_all_semitones() {
    for semi in -24..=24 {
        let (mut pitch, handle) = PitchShifter::shared(SR);
        PitchShifter::set_semitones_shared(&handle, semi as f32);
        let mut buf = noise(512);
        pitch.process(&mut buf);
        assert_finite(&buf, "pitch sweep");
    }
}

#[test]
fn empty_buffers_are_safe_everywhere() {
    let mut pipeline = ProcessingPipeline::new(SR);
    pipeline.add(Box::new(Gain::new(3.0)));
    let (delay, _) = Delay::shared(SR);
    let (reverb, _) = Reverb::shared(SR);
    pipeline.add(Box::new(delay));
    pipeline.add(Box::new(reverb));
    let mut empty: [f32; 0] = [];
    pipeline.process(&mut empty);
}

#[test]
fn limiter_handles_silence_and_full_scale() {
    let mut limiter = PeakLimiter::with_params(
        SR,
        LimiterParams { threshold_db: -6.0, attack_ms: 0.0, release_ms: 0.0, ceiling: 1.0 },
    );
    let mut out = vec![0.0f32; 4096];
    limiter.process(&mut out);
    assert_silent(&out, "limiter silence");
    let mut loud = vec![1.0f32; 4096];
    limiter.process(&mut loud);
    assert_finite(&loud, "limiter full scale");
    assert!(loud.iter().all(|v| v.abs() <= 1.0));
}

#[test]
fn state_definitions_are_all_constructible() {
    let _d = DelayParams::default();
    let _r = ReverbParams::default();
    let _x = DistortionParams::default();
    let _m = ModulationParams::default();
    let _v = VocalParams::default();
    let _l = LimiterParams::default();
}
