pub trait AudioProcessor: Send {
    fn name(&self) -> &'static str { "processor" }
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
    pub fn is_empty(&self) -> bool { self.processors.is_empty() }
    pub fn names(&self) -> Vec<&'static str> { self.processors.iter().map(|p| p.name()).collect() }
    pub fn find(&self, name: &str) -> Option<usize> { self.processors.iter().position(|p| p.name() == name) }
    pub fn bypass(&self, index: usize) -> bool { self.processors.get(index).map(|p| p.bypass()).unwrap_or(false) }
    pub fn set_bypass(&mut self, index: usize, b: bool) {
        if let Some(p) = self.processors.get_mut(index) { p.set_bypass(b); }
    }
    pub fn remove(&mut self, index: usize) -> Option<Box<dyn AudioProcessor>> {
        if index < self.processors.len() { Some(self.processors.remove(index)) } else { None }
    }
    pub fn swap(&mut self, a: usize, b: usize) {
        if a < self.processors.len() && b < self.processors.len() {
            self.processors.swap(a, b);
        }
    }
    pub fn move_by(&mut self, from: usize, to: usize) {
        if from < self.processors.len() && to < self.processors.len() && from != to {
            let p = self.processors.remove(from);
            self.processors.insert(to, p);
        }
    }
    pub fn take(&mut self) -> Vec<Box<dyn AudioProcessor>> {
        std::mem::take(&mut self.processors)
    }
}

use crate::gain::Gain;
use crate::meter::LevelMeterProcessor;
use crate::gate::NoiseGate;
use crate::compressor::Compressor;
use crate::filter::Biquad;
use crate::delay::Delay;
use crate::reverb::Reverb;
use crate::distortion::Distortion;
use crate::modulation::Modulation;
use crate::eq::ParametricEq;
use crate::pitch::PitchShifter;
use crate::vocal::Vocal;

impl AudioProcessor for Gain {
    fn name(&self) -> &'static str { "gain" }
    fn process(&mut self, samples: &mut [f32], _sr: u32) { Gain::process(self, samples); }
    fn bypass(&self) -> bool { Gain::is_bypassed(self) }
    fn set_bypass(&mut self, b: bool) { Gain::set_bypass(self, b); }
}
impl AudioProcessor for NoiseGate {
    fn name(&self) -> &'static str { "gate" }
    fn process(&mut self, samples: &mut [f32], _sr: u32) { NoiseGate::process(self, samples); }
    fn bypass(&self) -> bool { NoiseGate::is_bypassed(self) }
    fn set_bypass(&mut self, b: bool) { NoiseGate::set_bypass(self, b); }
}
impl AudioProcessor for Compressor {
    fn name(&self) -> &'static str { "compressor" }
    fn process(&mut self, samples: &mut [f32], _sr: u32) { Compressor::process(self, samples); }
    fn bypass(&self) -> bool { Compressor::is_bypassed(self) }
    fn set_bypass(&mut self, b: bool) { Compressor::set_bypass(self, b); }
}
impl AudioProcessor for Biquad {
    fn name(&self) -> &'static str { "highpass" }
    fn process(&mut self, samples: &mut [f32], _sr: u32) { Biquad::process(self, samples); }
    fn bypass(&self) -> bool { Biquad::is_bypassed(self) }
    fn set_bypass(&mut self, b: bool) { Biquad::set_bypass(self, b); }
}
impl AudioProcessor for LevelMeterProcessor {
    fn name(&self) -> &'static str { "meter" }
    fn process(&mut self, samples: &mut [f32], _sr: u32) { LevelMeterProcessor::process(self, samples); }
}
impl AudioProcessor for Delay {
    fn name(&self) -> &'static str { "delay" }
    fn process(&mut self, samples: &mut [f32], _sr: u32) { Delay::process(self, samples); }
    fn bypass(&self) -> bool { Delay::is_bypassed(self) }
    fn set_bypass(&mut self, b: bool) { Delay::set_bypass(self, b); }
}
impl AudioProcessor for Reverb {
    fn name(&self) -> &'static str { "reverb" }
    fn process(&mut self, samples: &mut [f32], _sr: u32) { Reverb::process(self, samples); }
    fn bypass(&self) -> bool { Reverb::is_bypassed(self) }
    fn set_bypass(&mut self, b: bool) { Reverb::set_bypass(self, b); }
}
impl AudioProcessor for Distortion {
    fn name(&self) -> &'static str { "distortion" }
    fn process(&mut self, samples: &mut [f32], _sr: u32) { Distortion::process(self, samples); }
    fn bypass(&self) -> bool { Distortion::is_bypassed(self) }
    fn set_bypass(&mut self, b: bool) { Distortion::set_bypass(self, b); }
}
impl AudioProcessor for Modulation {
    fn name(&self) -> &'static str { "modulation" }
    fn process(&mut self, samples: &mut [f32], _sr: u32) { Modulation::process(self, samples); }
    fn bypass(&self) -> bool { Modulation::is_bypassed(self) }
    fn set_bypass(&mut self, b: bool) { Modulation::set_bypass(self, b); }
}
impl AudioProcessor for ParametricEq {
    fn name(&self) -> &'static str { "eq" }
    fn process(&mut self, samples: &mut [f32], _sr: u32) { ParametricEq::process(self, samples); }
    fn bypass(&self) -> bool { ParametricEq::is_bypassed(self) }
    fn set_bypass(&mut self, b: bool) { ParametricEq::set_bypass(self, b); }
}
impl AudioProcessor for PitchShifter {
    fn name(&self) -> &'static str { "pitch" }
    fn process(&mut self, samples: &mut [f32], _sr: u32) { PitchShifter::process(self, samples); }
    fn bypass(&self) -> bool { PitchShifter::is_bypassed(self) }
    fn set_bypass(&mut self, b: bool) { PitchShifter::set_bypass(self, b); }
}
impl AudioProcessor for Vocal {
    fn name(&self) -> &'static str { "vocal" }
    fn process(&mut self, samples: &mut [f32], _sr: u32) { Vocal::process(self, samples); }
    fn bypass(&self) -> bool { Vocal::is_bypassed(self) }
    fn set_bypass(&mut self, b: bool) { Vocal::set_bypass(self, b); }
}
