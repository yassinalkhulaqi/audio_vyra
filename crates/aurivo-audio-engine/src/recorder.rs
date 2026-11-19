//! Minimal RIFF/WAVE reader and writer plus a shared recorder state used by
//! the audio callback to capture the processed signal.

use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
use std::path::Path;

#[derive(Debug)]
pub struct RecorderState {
    pub recording: bool,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl RecorderState {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self { recording: false, samples: Vec::new(), sample_rate, channels }
    }

    pub fn start(&mut self) {
        self.samples.clear();
        self.recording = true;
    }

    pub fn stop(&mut self) {
        self.recording = false;
    }

    pub fn push(&mut self, block: &[f32]) {
        if self.recording {
            self.samples.extend_from_slice(block);
        }
    }
}

fn write_u16<W: Write>(w: &mut W, v: u16) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

fn write_u32<W: Write>(w: &mut W, v: u32) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

pub fn write_wav<P: AsRef<Path>>(
    path: P,
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> io::Result<()> {
    let bytes_per_sample = 2u32;
    let data_len = (samples.len() as u32) * bytes_per_sample;
    let channels = channels.max(1);
    let byte_rate = sample_rate * channels as u32 * bytes_per_sample;
    let block_align = channels as u32 * bytes_per_sample;

    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    w.write_all(b"RIFF")?;
    write_u32(&mut w, 36 + data_len)?;
    w.write_all(b"WAVE")?;
    w.write_all(b"fmt ")?;
    write_u32(&mut w, 16)?;
    write_u16(&mut w, 1)?;
    write_u16(&mut w, channels)?;
    write_u32(&mut w, sample_rate)?;
    write_u32(&mut w, byte_rate)?;
    write_u16(&mut w, block_align as u16)?;
    write_u16(&mut w, 16)?;
    w.write_all(b"data")?;
    write_u32(&mut w, data_len)?;
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let value = (clamped * 32767.0) as i16;
        w.write_all(&value.to_le_bytes())?;
    }
    w.flush()
}

fn read_u16(bytes: &[u8], offset: usize) -> io::Result<u16> {
    let slice = bytes.get(offset..offset + 2).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "truncated WAV"))?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> io::Result<u32> {
    let slice = bytes.get(offset..offset + 4).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "truncated WAV"))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

pub struct DecodedWav {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

pub fn read_wav<P: AsRef<Path>>(path: P) -> io::Result<DecodedWav> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "not a RIFF/WAVE file"));
    }

    let mut pos = 12usize;
    let mut sample_rate = 0u32;
    let mut channels = 0u16;
    let mut bits = 0u16;
    let mut format = 0u16;
    let mut data: Option<(usize, usize)> = None;

    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = read_u32(&bytes, pos + 4)? as usize;
        let body = pos + 8;
        if body + size > bytes.len() {
            break;
        }
        match id {
            b"fmt " => {
                format = read_u16(&bytes, body)?;
                channels = read_u16(&bytes, body + 2)?;
                sample_rate = read_u32(&bytes, body + 4)?;
                bits = read_u16(&bytes, body + 14)?;
            }
            b"data" => data = Some((body, size)),
            _ => {}
        }
        pos = body + size + (size & 1);
    }

    let (start, size) = data.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no data chunk"))?;
    let end = start + size;
    let mut samples = Vec::new();

    match (format, bits) {
        (1, 16) => {
            let mut i = start;
            while i + 2 <= end {
                let raw = i16::from_le_bytes([bytes[i], bytes[i + 1]]);
                samples.push(raw as f32 / 32768.0);
                i += 2;
            }
        }
        (3, 32) => {
            let mut i = start;
            while i + 4 <= end {
                let bits32 = u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
                samples.push(f32::from_bits(bits32));
                i += 4;
            }
        }
        _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "unsupported WAV format")),
    }

    Ok(DecodedWav { samples, sample_rate, channels: channels.max(1) })
}
