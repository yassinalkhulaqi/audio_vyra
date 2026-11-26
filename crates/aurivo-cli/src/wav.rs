//! WAV inspection without loading the whole file into the audio engine.

use crate::format::{field, heading};
use std::fs;
use std::io::Read;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WavHeader {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub format: u16,
    pub data_bytes: u32,
}

pub fn read_header(path: &str) -> Result<WavHeader, String> {
    let mut file = fs::File::open(path).map_err(|e| format!("cannot open {path}: {e}"))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() < 44 {
        return Err("file is too short to be a WAV".into());
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".into());
    }
    let mut pos = 12usize;
    let mut header = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let body = pos + 8;
        if body + size > bytes.len() {
            break;
        }
        if id == b"fmt " && body + 16 <= bytes.len() {
            header = Some(WavHeader {
                format: u16::from_le_bytes(bytes[body..body + 2].try_into().unwrap()),
                channels: u16::from_le_bytes(bytes[body + 2..body + 4].try_into().unwrap()),
                sample_rate: u32::from_le_bytes(bytes[body + 4..body + 8].try_into().unwrap()),
                data_bytes: 0,
                bits_per_sample: u16::from_le_bytes(bytes[body + 14..body + 16].try_into().unwrap()),
            });
        }
        if id == b"data" {
            if let Some(h) = header.as_mut() {
                h.data_bytes = size as u32;
            }
        }
        pos = body + size + (size & 1);
    }
    header.ok_or_else(|| "no fmt chunk found".into())
}

pub fn describe(path: &str) -> Result<String, String> {
    let header = read_header(path)?;
    let frames = if header.channels > 0 && header.bits_per_sample > 0 {
        header.data_bytes / (header.channels as u32 * (header.bits_per_sample / 8) as u32)
    } else {
        0
    };
    let seconds = if header.sample_rate > 0 { frames as f32 / header.sample_rate as f32 } else { 0.0 };
    Ok([
        heading("WAV FILE"),
        field("Format", &header.format.to_string()),
        field("Channels", &header.channels.to_string()),
        field("Sample rate", &format!("{} Hz", header.sample_rate)),
        field("Bits per sample", &header.bits_per_sample.to_string()),
        field("Frames", &frames.to_string()),
        field("Duration", &format!("{seconds:.2} s")),
    ].join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_engine_wav_header() {
        use aurivo_audio_engine::recorder::write_wav;
        let path = std::env::temp_dir().join(format!("aurivo-cli-header-{}.wav", std::process::id()));
        write_wav(&path, &vec![0.0f32; 48000], 48000, 2).unwrap();
        let header = read_header(&path.to_string_lossy()).unwrap();
        assert_eq!(header.channels, 2);
        assert_eq!(header.sample_rate, 48000);
        assert_eq!(header.bits_per_sample, 16);
        assert_eq!(header.data_bytes, 48000 * 2);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_non_wav() {
        let path = std::env::temp_dir().join(format!("aurivo-cli-bad-{}.wav", std::process::id()));
        fs::write(&path, b"hello").unwrap();
        assert!(read_header(&path.to_string_lossy()).is_err());
        fs::remove_file(path).unwrap();
    }
}
