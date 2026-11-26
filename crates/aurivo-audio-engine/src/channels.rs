//! Interleaved multi-channel helpers. The DSP chain is mono, so input frames
//! are downmixed to one stream for processing and the result is duplicated
//! across every output channel.
//!
//! Every helper in this module is allocation-free when given caller-owned
//! scratch buffers. The `*_into` variants are safe to call from a real-time
//! audio callback; the convenience `Vec` variants are intended for setup code
//! and tests.

/// Number of input frames represented by `samples` interleaved samples.
pub fn frame_count(samples: &[f32], channels: u16) -> usize {
    let c = channels.max(1) as usize;
    samples.len().div_ceil(c)
}

/// Convert a frame count to an interleaved sample count.
pub fn sample_count(frames: usize, channels: u16) -> usize {
    frames * channels.max(1) as usize
}

/// Downmix interleaved audio into a mono buffer without allocating.
///
/// Returns the number of mono samples written to `out`. When the input ends
/// with a partial frame, the available channels of that frame are averaged.
pub fn downmix_into(interleaved: &[f32], channels: u16, out: &mut [f32]) -> usize {
    let c = channels.max(1) as usize;
    if c == 1 {
        let n = interleaved.len().min(out.len());
        out[..n].copy_from_slice(&interleaved[..n]);
        return n;
    }
    let frames = interleaved.len().div_ceil(c);
    let written = frames.min(out.len());
    for frame in 0..written {
        let start = frame * c;
        let end = (start + c).min(interleaved.len());
        let mut sum = 0.0f32;
        for &s in &interleaved[start..end] {
            sum += s;
        }
        out[frame] = sum / (end - start) as f32;
    }
    written
}

/// Allocate a mono buffer and downmix into it.
pub fn downmix(interleaved: &[f32], channels: u16) -> Vec<f32> {
    let mut out = vec![0.0f32; frame_count(interleaved, channels)];
    let n = downmix_into(interleaved, channels, &mut out);
    out.truncate(n);
    out
}

/// Duplicate each mono sample across `channels` interleaved output slots.
/// Returns the number of interleaved samples written.
pub fn expand_into(mono: &[f32], channels: u16, out: &mut [f32]) -> usize {
    let c = channels.max(1) as usize;
    let frames = mono.len().min(out.len() / c);
    for frame in 0..frames {
        let value = mono[frame];
        let start = frame * c;
        for sample in out.iter_mut().take(start + c).skip(start) {
            *sample = value;
        }
    }
    frames * c
}

/// Allocate an interleaved buffer and expand mono into it.
pub fn expand(mono: &[f32], channels: u16) -> Vec<f32> {
    let mut out = vec![0.0f32; sample_count(mono.len(), channels)];
    let n = expand_into(mono, channels, &mut out);
    out.truncate(n);
    out
}

/// Copy one channel out of an interleaved stream into a mono buffer.
/// Unlike `downmix_into`, this preserves the exact sample values.
pub fn take_channel_into(
    interleaved: &[f32],
    channels: u16,
    channel: u16,
    out: &mut [f32],
) -> usize {
    let c = channels.max(1) as usize;
    let wanted = channel.max(1).min(channels) as usize - 1;
    let frames = frame_count(interleaved, channels);
    let written = frames.min(out.len());
    for frame in 0..written {
        out[frame] = interleaved[frame * c + wanted];
    }
    written
}

/// Write one mono stream into a single channel of an interleaved buffer,
/// leaving all other channels untouched.
pub fn place_channel_into(mono: &[f32], channels: u16, channel: u16, out: &mut [f32]) -> usize {
    let c = channels.max(1) as usize;
    let wanted = channel.max(1).min(channels) as usize - 1;
    let frames = mono.len().min(out.len().div_ceil(c));
    for frame in 0..frames {
        let idx = frame * c + wanted;
        if idx < out.len() {
            out[idx] = mono[frame];
        }
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_stereo() {
        assert_eq!(downmix(&[1.0, 3.0, 2.0, 4.0], 2), vec![2.0, 3.0]);
    }

    #[test]
    fn expand_duplicates() {
        let mut out = vec![0.0; 4];
        expand_into(&[1.0, 2.0], 2, &mut out);
        assert_eq!(out, vec![1.0, 1.0, 2.0, 2.0]);
        assert_eq!(expand(&[1.0, 2.0], 2), vec![1.0, 1.0, 2.0, 2.0]);
    }

    #[test]
    fn frame_helpers_round_trip() {
        for channels in 1u16..=8 {
            let interleaved = vec![0.5f32; sample_count(7, channels)];
            assert_eq!(frame_count(&interleaved, channels), 7);
            let mono = downmix(&interleaved, channels);
            assert_eq!(mono.len(), 7);
            assert!(mono.iter().all(|v| (v - 0.5).abs() < 1e-6));
            assert_eq!(expand(&mono, channels).len(), sample_count(7, channels));
        }
    }

    #[test]
    fn downmix_partial_frame_does_not_panic() {
        let mut out = [0.0f32; 4];
        let n = downmix_into(&[1.0, 3.0, 2.0], 2, &mut out);
        assert_eq!(n, 2);
        assert!((out[0] - 2.0).abs() < 1e-6);
        assert!((out[1] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn downmix_into_respects_output_length() {
        let mut out = [0.0f32; 2];
        let n = downmix_into(&[1.0; 100], 2, &mut out);
        assert_eq!(n, 2);
    }

    #[test]
    fn take_and_place_channel() {
        let stereo = [1.0f32, 2.0, 3.0, 4.0];
        let mut left = [0.0f32; 2];
        let mut right = [0.0f32; 2];
        assert_eq!(take_channel_into(&stereo, 2, 1, &mut left), 2);
        assert_eq!(take_channel_into(&stereo, 2, 2, &mut right), 2);
        assert_eq!(left, [1.0, 3.0]);
        assert_eq!(right, [2.0, 4.0]);

        let mut out = [9.0f32; 4];
        place_channel_into(&[7.0, 8.0], 2, 2, &mut out);
        assert_eq!(out, [9.0, 7.0, 9.0, 8.0]);
    }

    #[test]
    fn mono_passthrough_is_exact() {
        let input = [0.1f32, -0.2, 0.3];
        let mut out = [0.0f32; 3];
        assert_eq!(downmix_into(&input, 1, &mut out), 3);
        assert_eq!(out, input);
    }

    #[test]
    fn zero_length_buffers_are_safe() {
        let mut out: [f32; 0] = [];
        assert_eq!(downmix_into(&[], 2, &mut out), 0);
        assert_eq!(expand_into(&[], 2, &mut out), 0);
        assert_eq!(take_channel_into(&[], 2, 1, &mut out), 0);
        assert_eq!(frame_count(&[], 2), 0);
    }
}
