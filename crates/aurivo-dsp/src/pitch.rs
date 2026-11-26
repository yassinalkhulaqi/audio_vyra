//! Granular pitch shifter using overlap-add of Hann-windowed grains.
//!
//! Two read heads sweep a circular delay line at a rate set by the desired
//! pitch ratio. Fractional read positions let us shift pitch without changing
//! the playback duration (within the inherent grain-latency tradeoff).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct Grain {
    read: f64,
    phase: f32,
}

#[derive(Debug)]
pub struct PitchShifter {
    buffer: Vec<f32>,
    mask: usize,
    write: usize,
    grain: usize,
    hop: usize,
    grains: Vec<Grain>,
    semitones: Arc<AtomicU32>,
    bypass: bool,
}

impl PitchShifter {
    pub fn new(sample_rate: u32) -> Self {
        Self::with_semitones(sample_rate, 0.0)
    }

    pub fn with_semitones(sample_rate: u32, semitones: f32) -> Self {
        let grain = ((sample_rate as f32 * 0.05) as usize).clamp(256, 8192);
        let hop = (grain / 2).max(1);
        let size = (grain * 4).next_power_of_two();
        let mask = size - 1;
        let grains = vec![
            Grain { read: -(grain as f64), phase: 0.0 },
            Grain { read: -(grain as f64) - (hop as f64), phase: 0.5 },
        ];
        Self {
            buffer: vec![0.0; size],
            mask,
            write: 0,
            grain,
            hop,
            grains,
            semitones: Arc::new(AtomicU32::new(semitones.to_bits())),
            bypass: false,
        }
    }

    pub fn shared(sample_rate: u32) -> (Self, Arc<AtomicU32>) {
        let shifter = Self::new(sample_rate);
        let handle = shifter.semitones.clone();
        (shifter, handle)
    }

    pub fn set_semitones_shared(handle: &Arc<AtomicU32>, semitones: f32) {
        handle.store(semitones.clamp(-24.0, 24.0).to_bits(), Ordering::Relaxed);
    }

    pub fn semitones(&self) -> f32 {
        f32::from_bits(self.semitones.load(Ordering::Relaxed))
    }

    pub fn set_bypass(&mut self, bypass: bool) {
        self.bypass = bypass;
    }

    pub fn is_bypassed(&self) -> bool {
        self.bypass
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write = 0;
        let grain_size = self.grain;
        let hop = self.hop;
        let count = self.grains.len() as f32;
        for (i, grain) in self.grains.iter_mut().enumerate() {
            grain.read = -(grain_size as f64) - (i as f64 * hop as f64);
            grain.phase = i as f32 / count;
        }
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        if self.bypass {
            return;
        }
        let ratio = 2f64.powf(self.semitones() as f64 / 12.0);
        let phase_step = 1.0 / self.hop as f32;
        let grain_len = self.grain as f64;
        let norm = 2.0 / self.grains.len() as f32;
        let mask = self.mask;

        for sample in samples.iter_mut() {
            self.buffer[self.write] = *sample;

            let mut acc = 0.0f32;
            let write_f = self.write as f64;
            for grain in self.grains.iter_mut() {
                let window = 0.5 - 0.5 * (std::f32::consts::TAU * grain.phase).cos();
                let base = grain.read.floor();
                let frac = (grain.read - base) as f32;
                let size = mask as i64 + 1;
                let i0 = (base as i64).rem_euclid(size) as usize & mask;
                let i1 = ((base as i64 + 1).rem_euclid(size)) as usize & mask;
                let a = self.buffer[i0];
                let b = self.buffer[i1];
                acc += (a + (b - a) * frac) * window;

                grain.phase += phase_step;
                grain.read += ratio;
                if grain.phase >= 1.0 {
                    grain.phase -= 1.0;
                    grain.read = write_f - grain_len;
                }
            }

            self.write = (self.write + 1) & self.mask;
            *sample = acc * norm;
        }
    }
}
