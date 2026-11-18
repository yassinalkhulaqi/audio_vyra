#[derive(Debug, Clone)]
pub struct Compressor {
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub makeup_db: f32,
    envelope: f32,
    sample_rate: f32,
    bypass: bool,
}

impl Compressor {
    pub fn new(threshold_db: f32, ratio: f32, attack_ms: f32, release_ms: f32, makeup_db: f32, sample_rate: f32) -> Self {
        Self { threshold_db, ratio, attack_ms, release_ms, makeup_db, envelope: 0.0, sample_rate, bypass: false }
    }
    pub fn set_bypass(&mut self, b: bool) { self.bypass = b; }
    pub fn process(&mut self, samples: &mut [f32]) {
        if self.bypass { return; }
        let thr = 10f32.powf(self.threshold_db / 20.0);
        let makeup = 10f32.powf(self.makeup_db / 20.0);
        let attack = if self.attack_ms > 0.0 { (-1.0 / (self.sample_rate * self.attack_ms / 1000.0)).exp() } else { 0.0 };
        let release = if self.release_ms > 0.0 { (-1.0 / (self.sample_rate * self.release_ms / 1000.0)).exp() } else { 0.0 };
        for s in samples.iter_mut() {
            let level = s.abs();
            let coeff = if level > self.envelope { attack } else { release };
            self.envelope = coeff * self.envelope + (1.0 - coeff) * level;
            if self.envelope > thr {
                let gain = thr * (self.envelope / thr).powf(1.0 / self.ratio) / self.envelope;
                *s *= gain * makeup;
            } else {
                *s *= makeup;
            }
        }
    }
}
