pub trait AudioProcessor: Send {
    fn process(&mut self, samples: &mut [f32], sample_rate: u32);
    fn bypass(&self) -> bool { false }
    fn set_bypass(&mut self, _b: bool) {}
}

pub struct ProcessingPipeline {
    processors: Vec<Box<dyn AudioProcessor>>,
    sample_rate: u32,
}

impl ProcessingPipeline {
    pub fn new(sample_rate: u32) -> Self { Self { processors: Vec::new(), sample_rate } }
    pub fn add(&mut self, p: Box<dyn AudioProcessor>) { self.processors.push(p); }
    pub fn process(&mut self, samples: &mut [f32]) {
        for p in self.processors.iter_mut() {
            p.process(samples, self.sample_rate);
        }
    }
    pub fn len(&self) -> usize { self.processors.len() }
}

use crate::gain::Gain;
use crate::meter::LevelMeterProcessor;
use crate::gate::NoiseGate;
use crate::compressor::Compressor;
use crate::filter::Biquad;
use crate::delay::Delay;
use crate::vocal::Vocal;

impl AudioProcessor for Gain {
    fn process(&mut self, samples: &mut [f32], _sr: u32) { Gain::process(self, samples); }
    fn bypass(&self) -> bool { false }
    fn set_bypass(&mut self, b: bool) { Gain::set_bypass(self, b); }
}
impl AudioProcessor for NoiseGate {
    fn process(&mut self, samples: &mut [f32], _sr: u32) { NoiseGate::process(self, samples); }
    fn set_bypass(&mut self, b: bool) { NoiseGate::set_bypass(self, b); }
}
impl AudioProcessor for Compressor {
    fn process(&mut self, samples: &mut [f32], _sr: u32) { Compressor::process(self, samples); }
    fn set_bypass(&mut self, b: bool) { Compressor::set_bypass(self, b); }
}
impl AudioProcessor for Biquad {
    fn process(&mut self, samples: &mut [f32], _sr: u32) { Biquad::process(self, samples); }
    fn set_bypass(&mut self, b: bool) { Biquad::set_bypass(self, b); }
}
impl AudioProcessor for LevelMeterProcessor {
    fn process(&mut self, samples: &mut [f32], _sr: u32) { LevelMeterProcessor::process(self, samples); }
}
impl AudioProcessor for Delay {
    fn process(&mut self, samples: &mut [f32], _sr: u32) { Delay::process(self, samples); }
    fn bypass(&self) -> bool { Delay::is_bypassed(self) }
    fn set_bypass(&mut self, b: bool) { Delay::set_bypass(self, b); }
}
impl AudioProcessor for Vocal {
    fn process(&mut self, samples: &mut [f32], _sr: u32) { Vocal::process(self, samples); }
    fn bypass(&self) -> bool { Vocal::is_bypassed(self) }
    fn set_bypass(&mut self, b: bool) { Vocal::set_bypass(self, b); }
}