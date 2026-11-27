//! High-level audio path helpers that keep the DSP chain mono while bridging
//! input/output devices that disagree on sample rate and channel count.
//!
//! The signal path implemented here is:
//!
//! ```text
//! interleaved input -> downmix -> DSP -> mono ring buffer
//!                                     -> resample (mono)
//!                                     -> expand -> interleaved output
//! ```
//!
//! Resampling *before* expansion avoids the classic bug where an interleaved
//! stream is treated as one mono signal and resampled at the wrong effective
//! frame rate.

use crate::channels::{downmix_into, expand_into};
use crate::resampler::LinearResampler;

/// Engine-level description of one audio device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

impl StreamFormat {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            sample_rate: sample_rate.max(1),
            channels: channels.max(1),
        }
    }

    pub fn needs_resampling(&self, output: &StreamFormat) -> bool {
        self.sample_rate != output.sample_rate
    }

    pub fn frames(&self, interleaved_samples: usize) -> usize {
        interleaved_samples.div_ceil(self.channels.max(1) as usize)
    }
}

/// Estimate the ring buffer capacity needed for `seconds` of mono audio.
pub fn mono_ring_capacity(sample_rate: u32, seconds: u32) -> usize {
    sample_rate.max(1) as usize * seconds.max(1) as usize
}

/// Convert post-DSP mono samples into output-ready interleaved frames,
/// resampling first when rates differ.
///
/// `scratch_mono` must be at least `src.len() + 8` samples; `scratch_out` must
/// be at least `src.len() * 2 + 8` samples. Both buffers are caller-owned so
/// the function performs no heap allocation.
pub fn prepare_output_frames(
    mono: &[f32],
    resampler: &mut LinearResampler,
    _scratch_mono: &mut [f32],
    scratch_out: &mut [f32],
    output: &mut [f32],
    resample: bool,
    output_channels: u16,
) -> usize {
    if resample {
        let wanted = output.len().div_ceil(output_channels.max(1) as usize);
        let limit = wanted.min(scratch_out.len());
        let n = resampler.process(mono, &mut scratch_out[..limit]);
        expand_into(&scratch_out[..n], output_channels, output)
    } else {
        let n = mono.len().min(wanted_frames(output.len(), output_channels));
        expand_into(&mono[..n], output_channels, output)
    }
}

fn wanted_frames(interleaved_samples: usize, channels: u16) -> usize {
    interleaved_samples.div_ceil(channels.max(1) as usize)
}

/// Resample a decoded WAV file to the playback device rate and channel count.
/// Returns a fresh interleaved buffer suitable for a cpal output callback.
pub fn adapt_playback(
    samples: &[f32],
    file_format: StreamFormat,
    device_format: StreamFormat,
) -> Vec<f32> {
    if file_format == device_format {
        return samples.to_vec();
    }

    let mut mono = Vec::with_capacity(samples.len());
    let mut scratch = vec![0.0f32; samples.len().max(16)];
    let mono_len = downmix_into(samples, file_format.channels, &mut scratch);
    mono.extend_from_slice(&scratch[..mono_len]);

    if file_format.sample_rate != device_format.sample_rate {
        let mut resampler = LinearResampler::new(file_format.sample_rate, device_format.sample_rate);
        let frames = mono.len() * device_format.sample_rate as usize
            / file_format.sample_rate.max(1) as usize
            + 8;
        let resampled = resampler.process_vec(&mono, frames);
        mono = resampled;
    }

    expand(&mono, device_format.channels)
}

fn expand(mono: &[f32], channels: u16) -> Vec<f32> {
    let mut out = vec![0.0f32; mono.len() * channels.max(1) as usize];
    expand_into(mono, channels, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_format_normalises_invalid_values() {
        let f = StreamFormat::new(0, 0);
        assert_eq!(f.sample_rate, 1);
        assert_eq!(f.channels, 1);
    }

    #[test]
    fn resampling_needed_only_when_rates_differ() {
        let a = StreamFormat::new(48000, 2);
        let b = StreamFormat::new(48000, 1);
        assert!(!a.needs_resampling(&b));
        let c = StreamFormat::new(44100, 2);
        assert!(a.needs_resampling(&c));
    }

    #[test]
    fn ring_capacity_is_four_seconds_of_mono() {
        assert_eq!(mono_ring_capacity(48000, 4), 192000);
    }

    #[test]
    fn adapt_playback_passthrough_on_matching_format() {
        let src = [0.1f32, -0.2, 0.3, -0.4];
        let f = StreamFormat::new(48000, 2);
        assert_eq!(adapt_playback(&src, f, f), src.to_vec());
    }

    #[test]
    fn adapt_playback_converts_channels() {
        let src = [1.0f32, 2.0, 3.0, 4.0];
        let out = adapt_playback(&src, StreamFormat::new(48000, 2), StreamFormat::new(48000, 1));
        assert_eq!(out.len(), 2);
        assert!((out[0] - 1.5).abs() < 1e-6);
        assert!((out[1] - 3.5).abs() < 1e-6);
    }

    #[test]
    fn adapt_playback_changes_rate_and_channels() {
        let src: Vec<f32> = (0..4800).map(|i| (i as f32 * 0.01).sin()).collect();
        let out = adapt_playback(&src, StreamFormat::new(48000, 1), StreamFormat::new(96000, 2));
        assert!(out.len() > src.len());
        assert_eq!(out.len() % 2, 0);
        assert!(out.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn prepare_output_without_resampling_expands_mono() {
        let mono = [0.5f32, -0.5];
        let mut resampler = LinearResampler::new(48000, 48000);
        let mut scratch_mono = [0.0f32; 16];
        let mut scratch_out = [0.0f32; 32];
        let mut output = [0.0f32; 4];
        let n = prepare_output_frames(
            &mono,
            &mut resampler,
            &mut scratch_mono,
            &mut scratch_out,
            &mut output,
            false,
            2,
        );
        assert_eq!(n, 4);
        assert_eq!(output, [0.5, 0.5, -0.5, -0.5]);
    }

    #[test]
    fn prepare_output_with_resampling_fills_output() {
        let mono = vec![0.25f32; 128];
        let mut resampler = LinearResampler::new(48000, 96000);
        let mut scratch_mono = [0.0f32; 256];
        let mut scratch_out = [0.0f32; 512];
        let mut output = vec![0.0f32; 16];
        let n = prepare_output_frames(
            &mono,
            &mut resampler,
            &mut scratch_mono,
            &mut scratch_out,
            &mut output,
            true,
            2,
        );
        assert!(n > 0 && n <= output.len());
        assert!(output[..n].iter().all(|v| *v > 0.2 && *v < 0.3));
    }
}
