//! Real-time spectrum analysis with an in-house radix-2 FFT.
//! The analyzer publishes dB magnitudes to a shared buffer so a UI thread can
//! poll the spectrum without touching the audio thread's state.

use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug)]
struct Complex {
    re: f32,
    im: f32,
}

impl Complex {
    fn new(re: f32, im: f32) -> Self { Self { re, im } }
    fn add(self, other: Self) -> Self { Self::new(self.re + other.re, self.im + other.im) }
    fn sub(self, other: Self) -> Self { Self::new(self.re - other.re, self.im - other.im) }
    fn mul(self, other: Self) -> Self {
        Self::new(self.re * other.re - self.im * other.im, self.re * other.im + self.im * other.re)
    }
    fn magnitude(self) -> f32 { (self.re * self.re + self.im * self.im).sqrt() }
}

fn fft(input: &mut [Complex]) {
    let n = input.len();
    if n < 2 || !n.is_power_of_two() {
        return;
    }

    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            input.swap(i, j);
        }
    }

    let mut len = 2usize;
    while len <= n {
        let angle = -2.0 * std::f32::consts::PI / len as f32;
        let wlen = Complex::new(angle.cos(), angle.sin());
        for i in (0..n).step_by(len) {
            let mut w = Complex::new(1.0, 0.0);
            for k in 0..len / 2 {
                let u = input[i + k];
                let v = input[i + k + len / 2].mul(w);
                input[i + k] = u.add(v);
                input[i + k + len / 2] = u.sub(v);
                w = w.mul(wlen);
            }
        }
        len <<= 1;
    }
}

#[derive(Debug)]
pub struct SpectrumAnalyzer {
    fft_size: usize,
    input: Vec<f32>,
    write: usize,
    window: Vec<f32>,
    bins: Vec<Complex>,
    output: Arc<Mutex<Vec<f32>>>,
}

impl SpectrumAnalyzer {
    pub fn new(fft_size: usize) -> Self {
        let size = fft_size.max(64).next_power_of_two();
        let window = (0..size)
            .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / size as f32).cos())
            .collect();
        Self {
            fft_size: size,
            input: vec![0.0; size],
            write: 0,
            window,
            bins: vec![Complex::new(0.0, 0.0); size],
            output: Arc::new(Mutex::new(vec![ -90.0f32; size / 2 ])),
        }
    }

    pub fn shared(fft_size: usize) -> (Self, Arc<Mutex<Vec<f32>>>) {
        let analyzer = Self::new(fft_size);
        let handle = analyzer.output.clone();
        (analyzer, handle)
    }

    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    pub fn snapshot(handle: &Arc<Mutex<Vec<f32>>>) -> Vec<f32> {
        handle.lock().map(|v| v.clone()).unwrap_or_default()
    }

    fn analyze(&mut self) {
        for i in 0..self.fft_size {
            self.bins[i] = Complex::new(self.input[i] * self.window[i], 0.0);
        }
        fft(&mut self.bins);
        if let Ok(mut out) = self.output.lock() {
            for i in 0..out.len() {
                let magnitude = self.bins[i].magnitude() / self.fft_size as f32;
                out[i] = 20.0 * magnitude.max(1e-7).log10();
            }
        }
    }

    pub fn push(&mut self, samples: &[f32]) {
        for &s in samples {
            self.input[self.write] = s;
            self.write += 1;
            if self.write >= self.fft_size {
                self.write = 0;
                self.analyze();
            }
        }
    }
}
