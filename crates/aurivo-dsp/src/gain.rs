use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Gain {
    gain_db: Arc<AtomicU32>,
    bypass: bool,
}

impl Gain {
    pub fn new(gain_db: f32) -> Self {
        Self { gain_db: Arc::new(AtomicU32::new(gain_db.to_bits())), bypass: false }
    }
    pub fn shared() -> (Self, Arc<AtomicU32>) {
        let a = Arc::new(AtomicU32::new(0f32.to_bits()));
        (Self { gain_db: a.clone(), bypass: false }, a)
    }
    pub fn set_gain_db(&mut self, v: f32) { self.gain_db.store(v.to_bits(), Ordering::Relaxed); }
    pub fn set_gain_db_shared(shared: &Arc<AtomicU32>, v: f32) { shared.store(v.to_bits(), Ordering::Relaxed); }
    pub fn set_bypass(&mut self, b: bool) { self.bypass = b; }
    pub fn is_bypassed(&self) -> bool { self.bypass }
    pub fn process(&mut self, samples: &mut [f32]) {
        if self.bypass { return; }
        let db = f32::from_bits(self.gain_db.load(Ordering::Relaxed));
        let g = 10f32.powf(db / 20.0);
        for s in samples.iter_mut() { *s *= g; }
    }
}
