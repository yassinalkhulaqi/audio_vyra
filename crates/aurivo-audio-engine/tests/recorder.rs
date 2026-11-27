use aurivo_audio_engine::recorder::{read_wav, write_wav, DecodedWav, RecorderState};
use std::fs;
use std::path::PathBuf;

fn temp_wav(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("aurivo-{}-{}.wav", name, std::process::id()))
}

#[test]
fn recorder_captures_only_while_recording() {
    let mut rec = RecorderState::new(48000, 1);
    rec.push(&[0.1, 0.2]);
    assert!(rec.samples.is_empty());
    rec.start();
    assert!(rec.recording);
    rec.push(&[0.3, 0.4]);
    rec.stop();
    rec.push(&[0.5]);
    assert_eq!(rec.samples, vec![0.3, 0.4]);
    assert!(!rec.recording);
}

#[test]
fn recorder_start_clears_previous_take() {
    let mut rec = RecorderState::new(48000, 1);
    rec.start();
    rec.push(&[1.0, 1.0, 1.0]);
    rec.start();
    assert!(rec.samples.is_empty());
    rec.push(&[0.5]);
    assert_eq!(rec.samples, vec![0.5]);
}

#[test]
fn wav_round_trip_mono_16bit() {
    let path = temp_wav("mono");
    let samples: Vec<f32> = (0..1000).map(|i| (i as f32 / 1000.0 * 6.28).sin() * 0.5).collect();
    write_wav(&path, &samples, 48000, 1).unwrap();
    let decoded: DecodedWav = read_wav(&path).unwrap();
    assert_eq!(decoded.sample_rate, 48000);
    assert_eq!(decoded.channels, 1);
    assert_eq!(decoded.samples.len(), samples.len());
    for (a, b) in samples.iter().zip(decoded.samples.iter()) {
        assert!((a - b).abs() < 1e-4, "{a} != {b}");
    }
    fs::remove_file(path).unwrap();
}

#[test]
fn wav_round_trip_stereo() {
    let path = temp_wav("stereo");
    let samples: Vec<f32> = (0..2000).map(|i| (i % 7) as f32 / 7.0 - 0.5).collect();
    write_wav(&path, &samples, 44100, 2).unwrap();
    let decoded = read_wav(&path).unwrap();
    assert_eq!(decoded.channels, 2);
    assert_eq!(decoded.sample_rate, 44100);
    assert_eq!(decoded.samples.len(), samples.len());
    fs::remove_file(path).unwrap();
}

#[test]
fn wav_clamps_out_of_range_samples() {
    let path = temp_wav("clamp");
    write_wav(&path, &[2.0, -2.0, f32::NAN, f32::INFINITY], 48000, 1).unwrap();
    let decoded = read_wav(&path).unwrap();
    assert!(decoded.samples.iter().all(|v| v.is_finite()));
    assert!(decoded.samples.iter().all(|v| v.abs() <= 1.0));
    fs::remove_file(path).unwrap();
}

#[test]
fn wav_reader_rejects_non_wav_files() {
    let path = temp_wav("invalid");
    fs::write(&path, b"not a wav file").unwrap();
    assert!(read_wav(&path).is_err());
    fs::remove_file(path).unwrap();
}

#[test]
fn wav_reader_rejects_truncated_header() {
    let path = temp_wav("truncated");
    fs::write(&path, b"RIFF").unwrap();
    assert!(read_wav(&path).is_err());
    fs::remove_file(path).unwrap();
}

#[test]
fn wav_reader_rejects_unsupported_format() {
    let path = temp_wav("format");
    let mut bytes = b"RIFF____WAVEfmt \x10\x00\x00\x00".to_vec();
    bytes.extend_from_slice(&3u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&48000u32.to_le_bytes());
    bytes.extend_from_slice(&96000u32.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data\x00\x00\x00\x00");
    fs::write(&path, bytes).unwrap();
    assert!(read_wav(&path).is_err());
    fs::remove_file(path).unwrap();
}

#[test]
fn wav_writer_handles_empty_recording() {
    let path = temp_wav("empty");
    write_wav(&path, &[], 48000, 1).unwrap();
    let decoded = read_wav(&path).unwrap();
    assert!(decoded.samples.is_empty());
    fs::remove_file(path).unwrap();
}

#[test]
fn wav_files_have_canonical_header_size() {
    let path = temp_wav("header");
    write_wav(&path, &[0.0; 64], 48000, 1).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(&bytes[12..16], b"fmt ");
    assert_eq!(&bytes[36..40], b"data");
    assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 128);
    assert_eq!(bytes.len(), 44 + 128);
    fs::remove_file(path).unwrap();
}

#[test]
fn multiple_recorders_are_independent() {
    let mut a = RecorderState::new(48000, 1);
    let mut b = RecorderState::new(44100, 2);
    a.start();
    b.start();
    a.push(&[1.0]);
    b.push(&[2.0, 3.0]);
    assert_eq!(a.samples, vec![1.0]);
    assert_eq!(b.samples, vec![2.0, 3.0]);
    assert_ne!(a.sample_rate, b.sample_rate);
}
