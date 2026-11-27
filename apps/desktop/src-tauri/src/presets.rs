//! JSON preset persistence for complete effect state.

use aurivo_audio_engine::EqBandInfo;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub params: BTreeMap<String, f32>,
    #[serde(default)]
    pub enabled: BTreeMap<String, bool>,
    #[serde(default)]
    pub chain: Vec<String>,
    #[serde(default)]
    pub eq_bands: Vec<EqBandInfo>,
}

pub fn default_chain() -> Vec<String> {
    vec![
        "gain", "gate", "compressor", "highpass", "delay", "reverb",
        "distortion", "modulation", "eq", "pitch", "vocal", "limiter",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

pub fn default_enabled() -> BTreeMap<String, bool> {
    default_chain().into_iter().map(|n| (n, true)).collect()
}

pub fn safe_name(name: &str) -> String {
    let filtered: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let trimmed = filtered.trim_matches('_');
    if trimmed.is_empty() { "untitled".to_string() } else { trimmed.to_string() }
}

fn preset_path(dir: &str, name: &str) -> PathBuf {
    Path::new(dir).join(format!("{}.json", safe_name(name)))
}

fn json_error(e: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

pub fn save(dir: &str, preset: &Preset) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let text = serde_json::to_string_pretty(preset).map_err(json_error)?;
    fs::write(preset_path(dir, &preset.name), text)
}

pub fn load(dir: &str, name: &str) -> io::Result<Preset> {
    let text = fs::read_to_string(preset_path(dir, name))?;
    serde_json::from_str(&text).map_err(json_error)
}

pub fn list(dir: &str) -> io::Result<Vec<String>> {
    let mut names = Vec::new();
    if !Path::new(dir).exists() {
        return Ok(names);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

pub fn delete(dir: &str, name: &str) -> io::Result<()> {
    fs::remove_file(preset_path(dir, name))
}

pub fn factory_presets() -> Vec<Preset> {
    let mut warm = BTreeMap::new();
    warm.insert("gain_db".into(), 3.0);
    warm.insert("output_gain_db".into(), 0.0);
    warm.insert("delay_time".into(), 220.0);
    warm.insert("delay_feedback".into(), 0.25);
    warm.insert("delay_mix".into(), 0.18);
    warm.insert("delay_damp".into(), 0.35);
    warm.insert("reverb_room".into(), 0.45);
    warm.insert("reverb_damp".into(), 0.5);
    warm.insert("reverb_wet".into(), 0.2);
    warm.insert("reverb_dry".into(), 0.9);
    warm.insert("dist_mode".into(), 0.0);
    warm.insert("dist_drive".into(), 4.0);
    warm.insert("dist_mix".into(), 0.15);
    warm.insert("mod_mode".into(), 0.0);
    warm.insert("mod_rate".into(), 0.8);
    warm.insert("mod_depth".into(), 0.2);
    warm.insert("mod_mix".into(), 0.12);
    warm.insert("pitch".into(), 0.0);
    warm.insert("vocal_mode".into(), 2.0);
    warm.insert("vowel".into(), 1.0);
    warm.insert("vocal_depth".into(), 0.5);
    warm.insert("vocal_mix".into(), 0.35);

    let mut radio = BTreeMap::new();
    radio.insert("gain_db".into(), 6.0);
    radio.insert("dist_mode".into(), 1.0);
    radio.insert("dist_drive".into(), 18.0);
    radio.insert("dist_down".into(), 8.0);
    radio.insert("dist_bits".into(), 8.0);
    radio.insert("dist_mix".into(), 0.7);
    radio.insert("mod_mode".into(), 3.0);
    radio.insert("mod_rate".into(), 5.0);
    radio.insert("mod_depth".into(), 0.4);
    radio.insert("mod_mix".into(), 0.5);
    radio.insert("pitch".into(), 0.0);
    radio.insert("vocal_mode".into(), 1.0);
    radio.insert("vocal_mix".into(), 0.4);

    let mut shred = BTreeMap::new();
    shred.insert("gain_db".into(), -6.0);
    shred.insert("dist_mode".into(), 2.0);
    shred.insert("dist_drive".into(), 32.0);
    shred.insert("dist_tone".into(), 0.8);
    shred.insert("dist_mix".into(), 0.85);
    shred.insert("pitch".into(), -2.0);
    shred.insert("mod_mode".into(), 1.0);
    shred.insert("mod_rate".into(), 2.0);
    shred.insert("mod_depth".into(), 0.3);
    shred.insert("mod_mix".into(), 0.25);
    shred.insert("vocal_mode".into(), 0.0);
    shred.insert("vocal_mix".into(), 0.2);

    vec![
        Preset { name: "Warm Vocal".into(), params: warm, enabled: default_enabled(), chain: default_chain(), eq_bands: Vec::new() },
        Preset { name: "Radio".into(), params: radio, enabled: default_enabled(), chain: default_chain(), eq_bands: Vec::new() },
        Preset { name: "Shred".into(), params: shred, enabled: default_enabled(), chain: default_chain(), eq_bands: Vec::new() },
    ]
}

pub fn install_factory(dir: &str) -> io::Result<Vec<String>> {
    fs::create_dir_all(dir)?;
    for p in factory_presets() {
        save(dir, &p)?;
    }
    list(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_round_trips_state_fields() {
        let dir = std::env::temp_dir().join(format!("aurivo-presets-test-{}", std::process::id()));
        let mut params = BTreeMap::new();
        params.insert("gain_db".into(), 3.0);
        let preset = Preset {
            name: "My Voice!".into(),
            params,
            enabled: default_enabled(),
            chain: default_chain(),
            eq_bands: Vec::new(),
        };
        save(dir.to_str().unwrap(), &preset).unwrap();
        let loaded = load(dir.to_str().unwrap(), "My Voice!").unwrap();
        assert_eq!(loaded.name, preset.name);
        assert_eq!(loaded.params.get("gain_db"), Some(&3.0));
        assert_eq!(loaded.chain.len(), 12);
        assert!(loaded.enabled.values().all(|v| *v));
        assert!(list(dir.to_str().unwrap()).unwrap().contains(&"My_Voice".to_string()));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn factory_presets_have_no_dead_highpass_key() {
        for p in factory_presets() {
            assert!(!p.params.contains_key("highpass"));
            assert_eq!(p.chain.len(), p.enabled.len());
        }
    }
}
