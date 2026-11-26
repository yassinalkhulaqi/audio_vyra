use aurivo_dsp::{AudioProcessor, Biquad, Compressor, Gain, NoiseGate, ProcessingPipeline};

#[test]
fn pipeline_bypass_reports_the_real_state() {
    let mut pipeline = ProcessingPipeline::new(48000);
    pipeline.add(Box::new(Gain::new(0.0)));
    pipeline.add(Box::new(NoiseGate::new(-40.0, 5.0, 50.0, 48000.0)));
    pipeline.add(Box::new(Compressor::new(-18.0, 3.0, 10.0, 100.0, 2.0, 48000.0)));
    pipeline.add(Box::new(Biquad::new_highpass(80.0, 0.707, 48000.0)));

    for i in 0..pipeline.len() {
        assert!(!pipeline.bypass(i));
    }
    pipeline.set_bypass(0, true);
    pipeline.set_bypass(2, true);
    assert!(pipeline.bypass(0));
    assert!(!pipeline.bypass(1));
    assert!(pipeline.bypass(2));
    assert!(!pipeline.bypass(3));
}

#[test]
fn pipeline_move_by_shifts_instead_of_swapping() {
    let mut pipeline = ProcessingPipeline::new(48000);
    for name in ["a", "b", "c"] {
        pipeline.add(Box::new(Named(name)));
    }
    pipeline.move_by(0, 2);
    assert_eq!(pipeline.names(), ["b", "c", "a"]);
}

struct Named(&'static str);

impl AudioProcessor for Named {
    fn name(&self) -> &'static str { self.0 }
    fn process(&mut self, _samples: &mut [f32], _sr: u32) {}
}
