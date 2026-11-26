use aurivo_audio_engine::audio_path::{adapt_playback, mono_ring_capacity, prepare_output_frames, StreamFormat};
use aurivo_audio_engine::channels::{downmix, expand, frame_count, sample_count};
use aurivo_audio_engine::resampler::LinearResampler;

#[test]
fn input_and_output_channels_are_independent() {
    let input = StreamFormat::new(48000, 2);
    let output = StreamFormat::new(48000, 6);
    let mono = downmix(&[1.0, 3.0, 2.0, 4.0], input.channels);
    assert_eq!(mono, vec![2.0, 3.0]);
    let playback = expand(&mono, output.channels);
    assert_eq!(playback.len(), 12);
    for frame in playback.chunks(6) {
        assert!(frame.iter().all(|v| *v == frame[0]));
    }
}

#[test]
fn resampling_happens_before_channel_expansion() {
    let input = StreamFormat::new(48000, 2);
    let output = StreamFormat::new(96000, 2);
    assert!(input.needs_resampling(&output));
    let mono = vec![0.5f32; 128];
    let mut resampler = LinearResampler::new(input.sample_rate, output.sample_rate);
    let mut scratch = vec![0.0f32; 256];
    let mut pcm = vec![0.0f32; 32];
    let written = prepare_output_frames(&mono, &mut resampler, &mut [], &mut scratch, &mut pcm, true, 2);
    assert!(written > 0);
    assert_eq!(written % 2, 0);
    assert!(pcm[..written].iter().all(|v| (*v - 0.5).abs() < 0.01));
}

#[test]
fn ring_buffer_holds_mono_not_interleaved() {
    for channels in [1u16, 2, 6] {
        let capacity = mono_ring_capacity(48000, 4);
        assert_eq!(capacity, 192000);
        let interleaved = vec![0.0f32; sample_count(48, channels)];
        assert_eq!(frame_count(&interleaved, channels), 48);
        assert_eq!(downmix(&interleaved, channels).len(), 48);
    }
}

#[test]
fn playback_adapts_44k_mono_to_48k_stereo() {
    let samples: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.01).sin()).collect();
    let adapted = adapt_playback(
        &samples,
        StreamFormat::new(44100, 1),
        StreamFormat::new(48000, 2),
    );
    assert!(adapted.len() > samples.len());
    assert_eq!(adapted.len() % 2, 0);
    assert!(adapted.iter().all(|v| v.is_finite()));
}

#[test]
fn playback_adapts_high_channel_count_to_mono() {
    let samples = vec![1.0f32; 60];
    let adapted = adapt_playback(
        &samples,
        StreamFormat::new(48000, 3),
        StreamFormat::new(48000, 1),
    );
    assert_eq!(adapted.len(), 20);
    assert!(adapted.iter().all(|v| (v - 1.0).abs() < 1e-6));
}

#[test]
fn prepare_output_handles_empty_ring() {
    let mut resampler = LinearResampler::new(48000, 48000);
    let mut scratch = [0.0f32; 32];
    let mut out = [1.0f32; 8];
    let n = prepare_output_frames(&[], &mut resampler, &mut [], &mut scratch, &mut out, false, 2);
    assert_eq!(n, 0);
    assert_eq!(out, [1.0; 8]);
}
