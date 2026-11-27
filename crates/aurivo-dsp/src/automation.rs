//! Lightweight low-frequency oscillators for parameter automation.
//!
//! Routes are evaluated at control rate (the UI polls them, or the audio
//! engine ticks them once per block) and produce offsets that are added to a
//! base parameter value. The module has no I/O dependencies and is fully
//! deterministic, which makes it easy to test.

use crate::validate::safe_ms;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LfoShape {
    #[default]
    Sine,
    Triangle,
    Square,
    Saw,
    Random,
}

impl LfoShape {
    pub fn from_i32(value: i32) -> Self {
        match value {
            1 => Self::Triangle,
            2 => Self::Square,
            3 => Self::Saw,
            4 => Self::Random,
            _ => Self::Sine,
        }
    }

    pub fn as_i32(self) -> i32 {
        match self {
            Self::Sine => 0,
            Self::Triangle => 1,
            Self::Square => 2,
            Self::Saw => 3,
            Self::Random => 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Lfo {
    shape: LfoShape,
    rate_hz: f32,
    depth: f32,
    phase: f32,
    seed: u32,
}

impl Lfo {
    pub fn new(shape: LfoShape, rate_hz: f32, depth: f32) -> Self {
        Self {
            shape,
            rate_hz: rate_hz.clamp(0.01, 40.0),
            depth: if depth.is_finite() { depth.max(0.0) } else { 0.0 },
            phase: 0.0,
            seed: 0x9e3779b9,
        }
    }

    pub fn shape(&self) -> LfoShape { self.shape }
    pub fn rate(&self) -> f32 { self.rate_hz }
    pub fn depth(&self) -> f32 { self.depth }
    pub fn phase(&self) -> f32 { self.phase }

    pub fn set_shape(&mut self, shape: LfoShape) { self.shape = shape; }
    pub fn set_rate(&mut self, rate: f32) { self.rate_hz = rate.clamp(0.01, 40.0); }
    pub fn set_depth(&mut self, depth: f32) { self.depth = if depth.is_finite() { depth.max(0.0) } else { 0.0 }; }
    pub fn set_phase(&mut self, phase: f32) { self.phase = phase.rem_euclid(1.0); }
    pub fn reset(&mut self) { self.phase = 0.0; self.seed = 0x9e3779b9; }

    pub fn advance(&mut self, seconds: f32) {
        if !seconds.is_finite() || seconds <= 0.0 {
            return;
        }
        self.phase = (self.phase + self.rate_hz * seconds).rem_euclid(1.0);
    }

    pub fn value(&self) -> f32 {
        let p = self.phase;
        let raw = match self.shape {
            LfoShape::Sine => (std::f32::consts::TAU * p).sin(),
            LfoShape::Triangle => 1.0 - 4.0 * (p - 0.5).abs(),
            LfoShape::Square => if p < 0.5 { 1.0 } else { -1.0 },
            LfoShape::Saw => 2.0 * p - 1.0,
            LfoShape::Random => {
                let x = ((self.seed as f64 * 6364136223846793005.0 + 1.0) as u64 >> 33) as f32 / (1u64 << 31) as f32;
                x * 2.0 - 1.0
            }
        };
        raw.clamp(-1.0, 1.0) * self.depth
    }

    pub fn next(&mut self, seconds: f32) -> f32 {
        self.advance(seconds);
        self.value()
    }

    fn tick_random(&mut self) {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
    }
}

#[derive(Debug, Clone)]
pub struct AutomationRoute {
    pub target: String,
    pub lfo: Lfo,
    pub enabled: bool,
}

impl AutomationRoute {
    pub fn new(target: &str, shape: LfoShape, rate: f32, depth: f32) -> Self {
        Self { target: target.to_string(), lfo: Lfo::new(shape, rate, depth), enabled: true }
    }

    pub fn sample(&mut self, seconds: f32) -> (String, f32) {
        let value = if self.enabled { self.lfo.next(seconds) } else { 0.0 };
        if self.lfo.shape() == LfoShape::Random {
            self.lfo.tick_random();
        }
        (self.target.clone(), value)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AutomationBank {
    routes: Vec<AutomationRoute>,
}

impl AutomationBank {
    pub fn new() -> Self { Self::default() }

    pub fn add(&mut self, route: AutomationRoute) {
        if self.routes.len() < 16 {
            self.routes.push(route);
        }
    }

    pub fn remove(&mut self, target: &str) -> bool {
        let before = self.routes.len();
        self.routes.retain(|r| r.target != target);
        self.routes.len() != before
    }

    pub fn clear(&mut self) { self.routes.clear(); }

    pub fn len(&self) -> usize { self.routes.len() }
    pub fn is_empty(&self) -> bool { self.routes.is_empty() }

    pub fn routes(&self) -> &[AutomationRoute] { &self.routes }

    pub fn set_enabled(&mut self, target: &str, enabled: bool) -> bool {
        if let Some(route) = self.routes.iter_mut().find(|r| r.target == target) {
            route.enabled = enabled;
            true
        } else {
            false
        }
    }

    pub fn sample_all(&mut self, seconds: f32) -> Vec<(String, f32)> {
        let seconds = safe_ms(seconds * 1000.0) / 1000.0;
        self.routes.iter_mut().map(|r| r.sample(seconds)).collect()
    }

    pub fn offset_for(&self, target: &str) -> f32 {
        self.routes.iter().find(|r| r.target == target && r.enabled).map(|r| r.lfo.value()).unwrap_or(0.0)
    }
}

impl From<&AutomationBank> for Vec<(String, i32, f32, f32, bool)> {
    fn from(bank: &AutomationBank) -> Self {
        bank.routes().iter().map(|r| (
            r.target.clone(),
            r.lfo.shape().as_i32(),
            r.lfo.rate(),
            r.lfo.depth(),
            r.enabled,
        )).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_are_bounded() {
        for shape in [LfoShape::Sine, LfoShape::Triangle, LfoShape::Square, LfoShape::Saw, LfoShape::Random] {
            let lfo = Lfo::new(shape, 2.0, 0.8);
            for step in 0..100 {
                let mut lfo = lfo.clone();
                lfo.advance(step as f32 / 1000.0);
                assert!(lfo.value().abs() <= 0.8 + 1e-6);
            }
        }
    }

    #[test]
    fn sine_completes_one_cycle() {
        let mut lfo = Lfo::new(LfoShape::Sine, 1.0, 1.0);
        assert!((lfo.value() - 0.0).abs() < 1e-6);
        lfo.advance(0.25);
        assert!((lfo.value() - 1.0).abs() < 1e-4);
        lfo.advance(0.25);
        assert!(lfo.value().abs() < 1e-4);
        lfo.advance(0.25);
        assert!((lfo.value() + 1.0).abs() < 1e-4);
    }

    #[test]
    fn triangle_and_saw_have_expected_ranges() {
        let mut tri = Lfo::new(LfoShape::Triangle, 1.0, 1.0);
        tri.set_phase(0.25);
        assert!((tri.value() - 0.0).abs() < 1e-6);
        tri.set_phase(0.5);
        assert!((tri.value() - 1.0).abs() < 1e-6);
        tri.set_phase(0.0);
        assert!((tri.value() + 1.0).abs() < 1e-6);
        let mut saw = Lfo::new(LfoShape::Saw, 1.0, 1.0);
        saw.set_phase(0.0);
        assert!((saw.value() + 1.0).abs() < 1e-6);
        saw.set_phase(0.99);
        assert!((saw.value() - 0.98).abs() < 1e-6);
    }

    #[test]
    fn square_is_bipolar() {
        let mut lfo = Lfo::new(LfoShape::Square, 1.0, 0.5);
        lfo.set_phase(0.1);
        assert!((lfo.value() - 0.5).abs() < 1e-6);
        lfo.set_phase(0.6);
        assert!((lfo.value() + 0.5).abs() < 1e-6);
    }

    #[test]
    fn phase_wraps_and_ignores_bad_time() {
        let mut lfo = Lfo::new(LfoShape::Sine, 4.0, 1.0);
        lfo.advance(10.0);
        assert!((0.0..1.0).contains(&lfo.phase()));
        let before = lfo.phase();
        lfo.advance(f32::NAN);
        assert_eq!(lfo.phase(), before);
    }

    #[test]
    fn bank_limits_routes() {
        let mut bank = AutomationBank::new();
        for i in 0..20 {
            bank.add(AutomationRoute::new(&format!("p{i}"), LfoShape::Sine, 1.0, 0.5));
        }
        assert_eq!(bank.len(), 16);
        assert!(bank.remove("p0"));
        assert!(!bank.remove("missing"));
        assert_eq!(bank.len(), 15);
    }

    #[test]
    fn disabled_routes_return_zero() {
        let mut bank = AutomationBank::new();
        bank.add(AutomationRoute::new("gain_db", LfoShape::Sine, 1.0, 1.0));
        bank.set_enabled("gain_db", false);
        let samples = bank.sample_all(0.1);
        assert_eq!(samples[0].1, 0.0);
        assert_eq!(bank.offset_for("gain_db"), 0.0);
    }

    #[test]
    fn sampling_is_deterministic() {
        let build = || {
            let mut bank = AutomationBank::new();
            bank.add(AutomationRoute::new("pitch", LfoShape::Triangle, 3.0, 2.0));
            (0..64).map(|_| bank.sample_all(0.01)[0].1).collect::<Vec<_>>()
        };
        assert_eq!(build(), build());
    }
}
