use aurivo_dsp::{Gain, ProcessingPipeline, SmoothedGain, SmoothedValue};

#[test]
fn shared_smoothed_gain_receives_parameter_updates() {
    let (mut gain, handle) = SmoothedGain::shared(48000);
    Gain::set_gain_db_shared(&handle, 12.0);
    let mut buf = vec![1.0f32; 256];
    gain.process(&mut buf);
    assert!(buf.iter().all(|v| (*v).is_finite()));
    assert!(buf.iter().any(|v| *v > 0.0));
}

#[test]
fn smoothed_gain_reaches_target_after_many_blocks() {
    let (mut gain, handle) = SmoothedGain::shared(48000);
    Gain::set_gain_db_shared(&handle, 6.0);
    for _ in 0..20 {
        let mut buf = vec![1.0f32; 256];
        gain.process(&mut buf);
        assert!(buf.iter().all(|v| v.is_finite()));
    }
    let mut final_buf = vec![1.0f32; 256];
    gain.process(&mut final_buf);
    assert!((final_buf[255] - 2.0).abs() < 0.01);
}

#[test]
fn smoothed_gain_bypass_is_transparent() {
    let (mut gain, _) = SmoothedGain::shared(48000);
    gain.set_bypass(true);
    let mut buf = vec![0.37f32; 64];
    let before = buf.clone();
    gain.process(&mut buf);
    assert_eq!(buf, before);
}

#[test]
fn pipeline_accepts_smoothed_gain() {
    let mut pipeline = ProcessingPipeline::new(48000);
    pipeline.add(Box::new(SmoothedGain::new(48000, 2.0)));
    assert_eq!(pipeline.names(), ["gain"]);
    let mut buf = vec![0.5f32; 128];
    pipeline.process(&mut buf);
    assert!(buf.iter().all(|v| v.is_finite()));
}

#[test]
fn smoothing_initial_values_are_equal() {
    let slow = SmoothedValue::new(0.0, 48000, 20.0);
    let fast = SmoothedValue::new(0.0, 48000, 2.0);
    assert_eq!(slow.current(), fast.current());
}
