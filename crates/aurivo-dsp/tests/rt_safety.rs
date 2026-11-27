use aurivo_dsp::{Delay, DelayParams, Distortion, DistortionParams, Modulation, Reverb, Vocal, VocalParams};
use std::sync::{Arc, Mutex};
use std::thread;

fn run_effect<F: FnMut(&mut [f32])>(mut f: F) -> Vec<f32> {
    let mut buf = vec![0.25f32; 128];
    f(&mut buf);
    buf
}

#[test]
fn delay_survives_contended_parameter_lock() {
    let (mut delay, handle) = Delay::shared(48000);
    let holder = handle.lock().unwrap();
    let out = run_effect(|b| delay.process(b));
    drop(holder);
    assert!(out.iter().all(|v| v.is_finite()));
    assert!(delay.skipped_polls() > 0);
    Delay::update(&handle, DelayParams { time_ms: 500.0, ..DelayParams::default() });
    assert_eq!(Delay::snapshot(&handle).time_ms, 500.0);
}

#[test]
fn reverb_survives_contended_parameter_lock() {
    let (mut reverb, handle) = Reverb::shared(48000);
    let holder = handle.lock().unwrap();
    let out = run_effect(|b| reverb.process(b));
    drop(holder);
    assert!(out.iter().all(|v| v.is_finite()));
    assert!(reverb.skipped_polls() > 0);
}

#[test]
fn distortion_survives_contended_parameter_lock() {
    let (mut dist, handle) = Distortion::shared(48000);
    let holder = handle.lock().unwrap();
    let out = run_effect(|b| dist.process(b));
    drop(holder);
    assert!(out.iter().all(|v| v.is_finite()));
    assert!(dist.skipped_polls() > 0);
    Distortion::update(&handle, DistortionParams { mode: 2, drive_db: 24.0, ..DistortionParams::default() });
    assert_eq!(Distortion::snapshot(&handle).mode, 2);
}

#[test]
fn modulation_survives_contended_parameter_lock() {
    let (mut mod_fx, handle) = Modulation::shared(48000);
    let holder = handle.lock().unwrap();
    let out = run_effect(|b| mod_fx.process(b));
    drop(holder);
    assert!(out.iter().all(|v| v.is_finite()));
    assert!(mod_fx.skipped_polls() > 0);
}

#[test]
fn vocal_survives_contended_parameter_lock() {
    let (mut vocal, handle) = Vocal::shared(48000);
    let holder = handle.lock().unwrap();
    let out = run_effect(|b| vocal.process(b));
    drop(holder);
    assert!(out.iter().all(|v| v.is_finite()));
    assert!(vocal.skipped_polls() > 0);
    Vocal::update(&handle, VocalParams { mode: 2, vowel: 3.0, ..VocalParams::default() });
    assert_eq!(Vocal::snapshot(&handle).vowel, 3.0);
}

#[test]
fn concurrent_ui_writer_never_blocks_renderer() {
    let (mut delay, handle) = Delay::shared(48000);
    let writer_handle = handle.clone();
    let writer = thread::spawn(move || {
        for i in 0..2000 {
            Delay::update(&writer_handle, DelayParams {
                time_ms: 100.0 + (i % 500) as f32,
                feedback: 0.3,
                mix: 0.4,
                damp: 0.2,
            });
        }
    });
    for _ in 0..2000 {
        let out = run_effect(|b| delay.process(b));
        assert!(out.iter().all(|v| v.is_finite()));
    }
    writer.join().unwrap();
}

#[test]
fn snapshot_returns_default_when_lock_is_poisoned() {
    let handle: Arc<Mutex<DelayParams>> = Arc::new(Mutex::new(DelayParams::default()));
    let clone = handle.clone();
    let _ = thread::spawn(move || {
        let _guard = clone.lock().unwrap();
        panic!("poison");
    }).join();
    assert!(handle.is_poisoned());
    assert_eq!(Delay::snapshot(&handle).time_ms, 300.0);
}

#[test]
fn all_effects_passthrough_when_bypassed() {
    let (mut delay, _) = Delay::shared(48000);
    let (mut reverb, _) = Reverb::shared(48000);
    let (mut dist, _) = Distortion::shared(48000);
    let (mut mod_fx, _) = Modulation::shared(48000);
    let (mut vocal, _) = Vocal::shared(48000);
    delay.set_bypass(true);
    reverb.set_bypass(true);
    dist.set_bypass(true);
    mod_fx.set_bypass(true);
    vocal.set_bypass(true);
    let input = vec![0.123f32; 64];
    let mut a = input.clone();
    delay.process(&mut a);
    assert_eq!(a, input);
    let mut b = input.clone();
    reverb.process(&mut b);
    assert_eq!(b, input);
    let mut c = input.clone();
    dist.process(&mut c);
    assert_eq!(c, input);
    let mut d = input.clone();
    mod_fx.process(&mut d);
    assert_eq!(d, input);
    let mut e = input.clone();
    vocal.process(&mut e);
    assert_eq!(e, input);
}
