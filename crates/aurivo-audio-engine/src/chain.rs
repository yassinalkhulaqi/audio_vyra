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
    pub fn is_empty(&self) -> bool { self.slots.is_empty() }

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
            let slot = self.slots.remove(i);
            self.slots.insert(j, slot);
            self.pipeline.move_by(i, j);
        }
        true
    }

    /// Rebuild the chain in the given order. Slots and processors are moved
    /// together so bypass state stays attached to the correct effect.
    pub fn reorder(&mut self, names: &[String]) -> bool {
        if names.len() != self.slots.len() {
            return false;
        }
        let mut order = Vec::with_capacity(names.len());
        for name in names {
            let Some(i) = self.slots.iter().position(|s| &s.name == name) else { return false };
            if order.contains(&i) {
                return false;
            }
            order.push(i);
        }
        let old_slots = std::mem::take(&mut self.slots);
        let mut old_processors: Vec<Option<Box<dyn AudioProcessor>>> =
            self.pipeline.take().into_iter().map(Some).collect();
        for i in order {
            self.slots.push(old_slots[i].clone());
            self.pipeline.add(old_processors[i].take().expect("processor index verified"));
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dummy(&'static str, Arc<AtomicBool>);

    impl Dummy {
        fn new(name: &'static str) -> Self {
            Self(name, Arc::new(AtomicBool::new(false)))
        }
    }

    impl AudioProcessor for Dummy {
        fn name(&self) -> &'static str { self.0 }
        fn process(&mut self, _samples: &mut [f32], _sr: u32) {}
        fn bypass(&self) -> bool { self.1.load(Ordering::Relaxed) }
        fn set_bypass(&mut self, b: bool) { self.1.store(b, Ordering::Relaxed); }
    }

    #[test]
    fn move_by_inserts_instead_of_swapping() {
        let mut chain = EffectChain::new(48000);
        chain.add("a", Box::new(Dummy::new("a")));
        chain.add("b", Box::new(Dummy::new("b")));
        chain.add("c", Box::new(Dummy::new("c")));
        assert!(chain.move_by("a", 2));
        assert_eq!(chain.names(), ["b", "c", "a"]);
    }

    #[test]
    fn reorder_keeps_bypass_attached_to_effect() {
        let mut chain = EffectChain::new(48000);
        chain.add("a", Box::new(Dummy::new("a")));
        chain.add("b", Box::new(Dummy::new("b")));
        chain.add("c", Box::new(Dummy::new("c")));
        assert!(chain.set_enabled("a", false));
        let order: Vec<String> = ["c", "a", "b"].iter().map(|s| s.to_string()).collect();
        assert!(chain.reorder(&order));
        assert_eq!(chain.names(), order);
        assert!(!chain.enabled("a"));
        assert!(chain.enabled("b"));
        assert!(chain.pipeline.bypass(1));
        assert!(!chain.pipeline.bypass(0));
    }

    #[test]
    fn reorder_rejects_unknown_or_duplicate_names() {
        let mut chain = EffectChain::new(48000);
        chain.add("a", Box::new(Dummy::new("a")));
        chain.add("b", Box::new(Dummy::new("b")));
        let bad: Vec<String> = ["a", "a"].iter().map(|s| s.to_string()).collect();
        assert!(!chain.reorder(&bad));
        let missing: Vec<String> = ["a", "z"].iter().map(|s| s.to_string()).collect();
        assert!(!chain.reorder(&missing));
        assert_eq!(chain.names(), ["a", "b"]);
    }
}
