#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiquadType { LowPass, HighPass, Peak { gain_db: i32 } }

#[derive(Debug, Clone)]
pub struct Biquad {
    pub b0: f32, pub b1: f32, pub b2: f32,
    pub a1: f32, pub a2: f32,
    z1: f32, z2: f32,
    bypass: bool,
}

impl Biquad {
    pub fn new_lowpass(freq: f32, q: f32, sr: f32) -> Self {
        let w0 = 2.0 * std::f32::consts::PI * freq / sr;
        let cosw = w0.cos();
        let sinw = w0.sin();
        let alpha = sinw / (2.0 * q);
        let b0 = (1.0 - cosw) / 2.0;
        let b1 = 1.0 - cosw;
        let b2 = (1.0 - cosw) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cosw;
        let a2 = 1.0 - alpha;
        Self { b0: b0/a0, b1: b1/a0, b2: b2/a0, a1: a1/a0, a2: a2/a0, z1:0.0, z2:0.0, bypass:false }
    }
    pub fn new_highpass(freq: f32, q: f32, sr: f32) -> Self {
        let w0 = 2.0 * std::f32::consts::PI * freq / sr;
        let cosw = w0.cos();
        let sinw = w0.sin();
        let alpha = sinw / (2.0 * q);
        let b0 = (1.0 + cosw) / 2.0;
        let b1 = -(1.0 + cosw);
        let b2 = (1.0 + cosw) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cosw;
        let a2 = 1.0 - alpha;
        Self { b0: b0/a0, b1: b1/a0, b2: b2/a0, a1: a1/a0, a2: a2/a0, z1:0.0, z2:0.0, bypass:false }
    }
    pub fn set_bypass(&mut self, b: bool) { self.bypass = b; }
    pub fn process(&mut self, samples: &mut [f32]) {
        if self.bypass { return; }
        for s in samples.iter_mut() {
            let x = *s;
            let y = self.b0 * x + self.z1;
            self.z1 = self.b1 * x - self.a1 * y + self.z2;
            self.z2 = self.b2 * x - self.a2 * y;
            *s = y;
        }
    }
    pub fn reset(&mut self) { self.z1 = 0.0; self.z2 = 0.0; }
}
