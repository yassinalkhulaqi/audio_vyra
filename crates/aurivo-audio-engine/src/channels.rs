//! Interleaved multi-channel helpers. The DSP chain is mono, so input frames
//! are downmixed to one stream for processing and the result is duplicated
//! across every output channel.

pub fn downmix(interleaved: &[f32], channels: u16) -> Vec<f32> {
    let c = channels.max(1) as usize;
    if c == 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks(c)
        .map(|frame| frame.iter().sum::<f32>() / c as f32)
        .collect()
}

pub fn expand(mono: &[f32], channels: u16, out: &mut [f32]) {
    let c = channels.max(1) as usize;
    for (i, &s) in mono.iter().enumerate() {
        for ch in 0..c {
            let idx = i * c + ch;
            if idx < out.len() {
                out[idx] = s;
            }
        }
    }
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
        expand(&[1.0, 2.0], 2, &mut out);
        assert_eq!(out, vec![1.0, 1.0, 2.0, 2.0]);
    }
}
