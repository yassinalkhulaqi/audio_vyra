//! Multiband parametric equalizer.
//!
//! Implements an RBJ-cookbook biquad filter with peaking, low-shelf,
//! high-shelf, low-pass and high-pass modes. Bands are stored in a shared
//! handle so the UI thread can change them while the audio thread renders.

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EqFilterKind {
    Peaking,
    LowShelf,
    HighShelf,
    LowPass,
    HighPass,
}

impl EqFilterKind {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => EqFilterKind::Peaking,
            1 => EqFilterKind::LowShelf,
            2 => EqFilterKind::HighShelf,
            3 => EqFilterKind::LowPass,
            4 => EqFilterKind::HighPass,
            _ => EqFilterKind::Peaking,
        }
    }

    pub fn as_i32(self) -> i32 {
        match self {
            EqFilterKind::Peaking => 0,
            EqFilterKind::LowShelf => 1,
            EqFilterKind::HighShelf => 2,
            EqFilterKind::LowPass => 3,
            EqFilterKind::HighPass => 4,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EqBand {
    pub kind: EqFilterKind,
    pub frequency: f32,
    pub q: f32,
    pub gain_db: f32,
    pub enabled: bool,
}

impl EqBand {
    pub fn peaking(frequency: f32, q: f32, gain_db: f32) -> Self {
        Self { kind: EqFilterKind::Peaking, frequency, q, gain_db, enabled: true }
    }

    pub fn low_shelf(frequency: f32, gain_db: f32) -> Self {
        Self { kind: EqFilterKind::LowShelf, frequency, q: 0.707, gain_db, enabled: true }
    }

    pub fn high_shelf(frequency: f32, gain_db: f32) -> Self {
        Self { kind: EqFilterKind::HighShelf, frequency, q: 0.707, gain_db, enabled: true }
    }
}

#[derive(Debug, Clone)]
struct Coefficients {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

fn coefficients(band: &EqBand, sample_rate: f32) -> Coefficients {
    let nyquist = sample_rate * 0.5;
    let freq = band.frequency.clamp(10.0, nyquist - 10.0);
    let q = band.q.clamp(0.1, 18.0);
    let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
    let cosw = w0.cos();
    let sinw = w0.sin();
    let alpha = sinw / (2.0 * q);
    let a = 10f32.powf(band.gain_db / 40.0);
    let sqrt_a_2 = 2.0 * a.sqrt() * alpha;

    let (b0, b1, b2, a0, a1, a2) = match band.kind {
        EqFilterKind::Peaking => (
            1.0 + alpha * a,
            -2.0 * cosw,
            1.0 - alpha * a,
            1.0 + alpha / a,
            -2.0 * cosw,
            1.0 - alpha / a,
        ),
        EqFilterKind::LowShelf => (
            a * ((a + 1.0) - (a - 1.0) * cosw + sqrt_a_2),
            2.0 * a * ((a - 1.0) - (a + 1.0) * cosw),
            a * ((a + 1.0) - (a - 1.0) * cosw - sqrt_a_2),
            (a + 1.0) + (a - 1.0) * cosw + sqrt_a_2,
            -2.0 * ((a - 1.0) + (a + 1.0) * cosw),
            (a + 1.0) + (a - 1.0) * cosw - sqrt_a_2,
        ),
        EqFilterKind::HighShelf => (
            a * ((a + 1.0) + (a - 1.0) * cosw + sqrt_a_2),
            -2.0 * a * ((a - 1.0) + (a + 1.0) * cosw),
            a * ((a + 1.0) + (a - 1.0) * cosw - sqrt_a_2),
            (a + 1.0) - (a - 1.0) * cosw + sqrt_a_2,
            2.0 * ((a - 1.0) - (a + 1.0) * cosw),
            (a + 1.0) - (a - 1.0) * cosw - sqrt_a_2,
        ),
        EqFilterKind::LowPass => {
            let b0 = (1.0 - cosw) / 2.0;
            (b0, 1.0 - cosw, b0, 1.0 + alpha, -2.0 * cosw, 1.0 - alpha)
        }
        EqFilterKind::HighPass => {
            let b0 = (1.0 + cosw) / 2.0;
            (b0, -(1.0 + cosw), b0, 1.0 + alpha, -2.0 * cosw, 1.0 - alpha)
        }
    };

    let inv = 1.0 / a0;
    Coefficients {
        b0: b0 * inv,
        b1: b1 * inv,
        b2: b2 * inv,
        a1: a1 * inv,
        a2: a2 * inv,
    }
}

#[derive(Debug, Clone)]
struct BiquadState {
    coeffs: Coefficients,
    z1: f32,
    z2: f32,
}

impl BiquadState {
    fn new(band: &EqBand, sample_rate: f32) -> Self {
        Self { coeffs: coefficients(band, sample_rate), z1: 0.0, z2: 0.0 }
    }

    fn set_band(&mut self, band: &EqBand, sample_rate: f32) {
        self.coeffs = coefficients(band, sample_rate);
    }

    fn process(&mut self, x: f32) -> f32 {
        let c = &self.coeffs;
        let y = c.b0 * x + self.z1;
        self.z1 = c.b1 * x - c.a1 * y + self.z2;
        self.z2 = c.b2 * x - c.a2 * y;
        y
    }
}

#[derive(Debug)]
pub struct EqParams {
    pub bands: Vec<EqBand>,
    pub version: u64,
}

impl EqParams {
    pub fn with_defaults(sample_rate: f32) -> Self {
        let bands = vec![
            EqBand::low_shelf(120.0, 0.0),
            EqBand::peaking(350.0, 1.0, 0.0),
            EqBand::peaking(1000.0, 1.0, 0.0),
            EqBand::peaking(3500.0, 1.0, 0.0),
            EqBand::high_shelf(9000.0, 0.0),
        ];
        let _ = sample_rate;
        Self { bands, version: 1 }
    }

    pub fn touch(&mut self) {
        self.version = self.version.wrapping_add(1);
    }
}

#[derive(Debug)]
pub struct ParametricEq {
    shared: Arc<Mutex<EqParams>>,
    states: Vec<BiquadState>,
    last_version: u64,
    sample_rate: f32,
    bypass: bool,
}

impl ParametricEq {
    pub fn new(sample_rate: u32) -> Self {
        let sr = sample_rate as f32;
        let params = EqParams::with_defaults(sr);
        let states = params.bands.iter().map(|b| BiquadState::new(b, sr)).collect();
        Self {
            shared: Arc::new(Mutex::new(params)),
            states,
            last_version: 0,
            sample_rate: sr,
            bypass: false,
        }
    }

    pub fn shared(sample_rate: u32) -> (Self, Arc<Mutex<EqParams>>) {
        let eq = Self::new(sample_rate);
        let shared = eq.shared.clone();
        (eq, shared)
    }

    pub fn set_bypass(&mut self, bypass: bool) {
        self.bypass = bypass;
    }

    pub fn is_bypassed(&self) -> bool {
        self.bypass
    }

    pub fn reset(&mut self) {
        for state in &mut self.states {
            state.z1 = 0.0;
            state.z2 = 0.0;
        }
    }

    fn sync_params(&mut self) {
        if let Ok(params) = self.shared.lock() {
            if params.version != self.last_version {
                if self.states.len() != params.bands.len() {
                    self.states = params
                        .bands
                        .iter()
                        .map(|b| BiquadState::new(b, self.sample_rate))
                        .collect();
                } else {
                    for (state, band) in self.states.iter_mut().zip(params.bands.iter()) {
                        state.set_band(band, self.sample_rate);
                    }
                }
                self.last_version = params.version;
            }
        }
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        if self.bypass {
            return;
        }
        self.sync_params();
        if let Ok(params) = self.shared.lock() {
            let bands = &params.bands;
            for s in samples.iter_mut() {
                let mut value = *s;
                for (state, band) in self.states.iter_mut().zip(bands.iter()) {
                    if band.enabled {
                        value = state.process(value);
                    }
                }
                *s = value;
            }
        }
    }
}

pub fn set_band(shared: &Arc<Mutex<EqParams>>, index: usize, band: EqBand) {
    if let Ok(mut params) = shared.lock() {
        if index < params.bands.len() {
            params.bands[index] = band;
            params.touch();
        }
    }
}

pub fn add_band(shared: &Arc<Mutex<EqParams>>, band: EqBand) {
    if let Ok(mut params) = shared.lock() {
        params.bands.push(band);
        params.touch();
    }
}

pub fn remove_band(shared: &Arc<Mutex<EqParams>>, index: usize) {
    if let Ok(mut params) = shared.lock() {
        if index < params.bands.len() {
            params.bands.remove(index);
            params.touch();
        }
    }
}

pub fn snapshot(shared: &Arc<Mutex<EqParams>>) -> Vec<EqBand> {
    shared.lock().map(|p| p.bands.clone()).unwrap_or_default()
}
