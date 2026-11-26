use aurivo_dsp::{Gain, LevelMeterProcessor, ProcessingPipeline};

#[test]
fn gain_amplifies() {
    let mut g = Gain::new(6.0);
    let mut buf = vec![0.5f32; 100];
    g.process(&mut buf);
    let expected = 0.5 * 10f32.powf(6.0/20.0);
    assert!((buf[0]-expected).abs() < 1e-5);
}

#[test]
fn meter_peak_rms() {
    let m = LevelMeterProcessor::new();
    let buf = vec![0.5f32; 4];
    m.process(&buf);
    assert!((m.peak() - 0.5).abs() < 1e-6);
    assert!((m.rms() - 0.5).abs() < 1e-6);
}

#[test]
fn pipeline_runs() {
    let mut p = ProcessingPipeline::new(48000);
    p.add(Box::new(Gain::new(0.0)));
    let mut buf = vec![0.25f32; 10];
    p.process(&mut buf);
    assert!((buf[0]-0.25).abs() < 1e-6);
}
