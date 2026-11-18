use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug)]
pub struct LevelMeterProcessor {
    peak: AtomicU32,
    rms_acc: AtomicU32,
    count: AtomicU32,
}

impl LevelMeterProcessor {
    pub fn new() -> Self {
        Self { peak: AtomicU32::new(0), rms_acc: AtomicU32::new(0), count: AtomicU32::new(0) }
    }
    pub fn process(&self, samples: &[f32]) {
        let mut peak = 0f32;
        let mut sumsq = 0f32;
        for &s in samples {
            let a = s.abs();
            if a > peak { peak = a; }
            sumsq += s * s;
        }
        // store peak as f32 bits
        self.peak.store(peak.to_bits(), Ordering::Relaxed);
        let n = samples.len() as f32;
        let rms = if n > 0.0 { (sumsq / n).sqrt() } else { 0.0 };
        self.rms_acc.store(rms.to_bits(), Ordering::Relaxed);
        self.count.store(samples.len() as u32, Ordering::Relaxed);
    }
    pub fn peak(&self) -> f32 { f32::from_bits(self.peak.load(Ordering::Relaxed)) }
    pub fn rms(&self) -> f32 { f32::from_bits(self.rms_acc.load(Ordering::Relaxed)) }
}
impl Default for LevelMeterProcessor { fn default() -> Self { Self::new() } }
