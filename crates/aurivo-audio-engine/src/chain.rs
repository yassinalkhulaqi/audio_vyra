//! Named, reorderable effect chain. Each slot tracks its enabled state so the
//! UI can enable/bypass, move, or remove effects without touching DSP code.

use aurivo_dsp::{AudioProcessor, ProcessingPipeline};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct EffectSlot {
    pub name: String,
    pub enabled: Arc<AtomicBool>,
}

pub struct EffectChain {
    slots: Vec<EffectSlot>,
    pipeline: ProcessingPipeline,
}

impl EffectChain {
    pub fn new(sample_rate: u32) -> Self {
        Self { slots: Vec::new(), pipeline: ProcessingPipeline::new(sample_rate) }
    }

    pub fn add(&mut self, name: &str, processor: Box<dyn AudioProcessor>) {
        self.slots.push(EffectSlot {
            name: name.to_string(),
            enabled: Arc::new(AtomicBool::new(true)),
        });
        self.pipeline.add(processor);
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        self.pipeline.process(samples);
    }

    pub fn slots(&self) -> &[EffectSlot] { &self.slots }
    pub fn names(&self) -> Vec<String> { self.slots.iter().map(|s| s.name.clone()).collect() }
    pub fn len(&self) -> usize { self.slots.len() }

    pub fn index(&self, name: &str) -> Option<usize> {
        self.slots.iter().position(|s| s.name == name)
    }

    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> bool {
        if let Some(i) = self.index(name) {
            self.slots[i].enabled.store(enabled, Ordering::Relaxed);
            self.pipeline.set_bypass(i, !enabled);
            true
        } else {
            false
        }
    }

    pub fn enabled(&self, name: &str) -> bool {
        self.index(name)
            .map(|i| self.slots[i].enabled.load(Ordering::Relaxed))
            .unwrap_or(false)
    }

    pub fn remove(&mut self, name: &str) -> bool {
        if let Some(i) = self.index(name) {
            self.slots.remove(i);
            self.pipeline.remove(i);
            true
        } else {
            false
        }
    }

    pub fn move_by(&mut self, name: &str, delta: isize) -> bool {
        let Some(i) = self.index(name) else { return false };
        let last = self.slots.len().saturating_sub(1) as isize;
        let j = (i as isize + delta).clamp(0, last) as usize;
        if i != j {
            self.slots.swap(i, j);
            self.pipeline.swap(i, j);
        }
        true
    }
}
