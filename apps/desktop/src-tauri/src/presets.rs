//! JSON preset persistence for complete effect state.
//!
//! Presets are plain JSON files in a user-selected directory. They store the
//! full `AppState` snapshot (all effect parameters, chain order, bypass states
//! and EQ bands) plus a flat `params` map for backwards compatibility with
//! presets created by earlier versions.
//!
//! Writes are atomic: data goes to a temporary file first and is renamed into
//! place only after a successful write, so an interrupted save cannot corrupt
//! an existing preset.

use aurivo_audio_engine::EqBandInfo;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const PRESET_FORMAT_VERSION: u32 = 3;

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
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub created_at: u64,
}

fn default_version() -> u32 {
    PRESET_FORMAT_VERSION
}

impl Default for Preset {
    fn default() -> Self {
        Self {
            name: "Untitled".into(),
            params: BTreeMap::new(),
            enabled: default_enabled(),
            chain: default_chain(),
            eq_bands: Vec::new(),
            description: String::new(),
            tags: Vec::new(),
            version: PRESET_FORMAT_VERSION,
            created_at: unix_seconds(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetBundle {
    #[serde(default = "default_version")]
    pub version: u32,
    pub presets: Vec<Preset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetSummary {
    pub file: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub version: u32,
    pub param_count: usize,
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

fn unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
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

/// Upgrade an older preset in place: fill empty chain/bypass maps and set a
/// current format version. Unknown extra parameters are preserved.
pub fn migrate(mut preset: Preset) -> Preset {
    if preset.chain.is_empty() {
        preset.chain = default_chain();
    }
    if preset.enabled.is_empty() {
        preset.enabled = default_enabled();
    }
    if preset.version == 0 {
        preset.version = PRESET_FORMAT_VERSION;
    }
    for name in &preset.chain {
        preset.enabled.entry(name.clone()).or_insert(true);
    }
    preset
}

/// Report any structural problems without rejecting the preset.
pub fn validate(preset: &Preset) -> Vec<String> {
    let mut problems = Vec::new();
    if preset.name.trim().is_empty() {
        problems.push("preset name is empty".into());
    }
    if preset.chain.is_empty() {
        problems.push("effect chain is empty".into());
    }
    if preset.version > PRESET_FORMAT_VERSION {
        problems.push(format!("preset version {} is newer than this app", preset.version));
    }
    for (key, value) in &preset.params {
        if !value.is_finite() {
            problems.push(format!("parameter {key} is not finite"));
        }
    }
    for name in &preset.chain {
        if !preset.enabled.contains_key(name) {
            problems.push(format!("chain effect {name} has no bypass state"));
        }
    }
    let known = default_chain();
    for name in preset.enabled.keys() {
        if !known.contains(name) {
            problems.push(format!("unknown effect {name} in bypass map"));
        }
    }
    for band in &preset.eq_bands {
        if !band.frequency.is_finite() || !band.q.is_finite() || !band.gain_db.is_finite() {
            problems.push("EQ band has a non-finite parameter".into());
        }
        if band.frequency <= 0.0 {
            problems.push("EQ band frequency must be positive".into());
        }
    }
    problems
}

pub fn save(dir: &str, preset: &Preset) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let preset = migrate(preset.clone());
    let text = serde_json::to_string_pretty(&preset).map_err(json_error)?;
    let final_path = preset_path(dir, &preset.name);
    let temp_path = final_path.with_extension("json.tmp");
    fs::write(&temp_path, text)?;
    match fs::rename(&temp_path, &final_path) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            let _ = fs::remove_file(&temp_path);
            Err(rename_error)
        }
    }
}

pub fn load(dir: &str, name: &str) -> io::Result<Preset> {
    let text = fs::read_to_string(preset_path(dir, name))?;
    let preset: Preset = serde_json::from_str(&text).map_err(json_error)?;
    Ok(migrate(preset))
}

pub fn list(dir: &str) -> io::Result<Vec<String>> {
    Ok(list_detailed(dir)?.into_iter().map(|s| s.name).collect())
}

pub fn list_detailed(dir: &str) -> io::Result<Vec<PresetSummary>> {
    let mut summaries = Vec::new();
    if !Path::new(dir).exists() {
        return Ok(summaries);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue,
        };
        let preset: Preset = match serde_json::from_str(&text) {
            Ok(preset) => migrate(preset),
            Err(_) => continue,
        };
        summaries.push(PresetSummary {
            file: stem.to_string(),
            name: preset.name,
            description: preset.description,
            tags: preset.tags,
            version: preset.version,
            param_count: preset.params.len(),
        });
    }
    summaries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(summaries)
}

pub fn delete(dir: &str, name: &str) -> io::Result<()> {
    fs::remove_file(preset_path(dir, name))
}

pub fn export_bundle(dir: &str) -> io::Result<PresetBundle> {
    let mut presets = Vec::new();
    for summary in list_detailed(dir)? {
        if let Ok(preset) = load(dir, &summary.file) {
            presets.push(preset);
        }
    }
    Ok(PresetBundle { version: PRESET_FORMAT_VERSION, presets })
}

pub fn import_bundle(dir: &str, bundle: &PresetBundle, overwrite: bool) -> io::Result<usize> {
    fs::create_dir_all(dir)?;
    let mut imported = 0;
    for preset in &bundle.presets {
        let path = preset_path(dir, &preset.name);
        if path.exists() && !overwrite {
            continue;
        }
        save(dir, preset)?;
        imported += 1;
    }
    Ok(imported)
}

fn make_preset(name: &str, description: &str, tags: &[&str], params: BTreeMap<String, f32>) -> Preset {
    Preset {
        name: name.into(),
        params,
        enabled: default_enabled(),
        chain: default_chain(),
        eq_bands: Vec::new(),
        description: description.into(),
        tags: tags.iter().map(|t| t.to_string()).collect(),
        version: PRESET_FORMAT_VERSION,
        created_at: 0,
    }
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

    let mut clean = BTreeMap::new();
    clean.insert("gain_db".into(), 0.0);
    clean.insert("delay_mix".into(), 0.0);
    clean.insert("reverb_wet".into(), 0.0);
    clean.insert("dist_mix".into(), 0.0);
    clean.insert("mod_mix".into(), 0.0);
    clean.insert("vocal_mix".into(), 0.0);
    clean.insert("pitch".into(), 0.0);

    let mut space = warm.clone();
    space.insert("reverb_room".into(), 0.85);
    space.insert("reverb_wet".into(), 0.55);
    space.insert("delay_time".into(), 480.0);
    space.insert("delay_feedback".into(), 0.45);
    space.insert("delay_mix".into(), 0.35);
    space.insert("mod_mode".into(), 0.0);
    space.insert("mod_depth".into(), 0.3);

    let mut robot = radio.clone();
    robot.insert("vocal_mode".into(), 1.0);
    robot.insert("vocal_carrier".into(), 80.0);
    robot.insert("vocal_mix".into(), 0.75);
    robot.insert("mod_mode".into(), 3.0);
    robot.insert("mod_rate".into(), 7.0);

    let mut whisper = clean.clone();
    whisper.insert("gain_db".into(), 8.0);
    whisper.insert("vocal_mode".into(), 2.0);
    whisper.insert("vowel".into(), 3.0);
    whisper.insert("vocal_mix".into(), 0.25);
    whisper.insert("pitch".into(), 4.0);

    let mut bass = shred.clone();
    bass.insert("pitch".into(), -12.0);
    bass.insert("dist_drive".into(), 18.0);
    bass.insert("dist_mix".into(), 0.4);
    bass.insert("mod_rate".into(), 0.4);

    vec![
        make_preset("Warm Vocal", "Gentle compression and a short room", &["vocal", "warm"], warm),
        make_preset("Radio", "Lo-fi broadcast character", &["retro", "distortion"], radio),
        make_preset("Shred", "Heavy fuzz and pitch drop", &["rock", "distortion"], shred),
        make_preset("Clean", "Neutral passthrough", &["clean", "utility"], clean),
        make_preset("Space", "Long tails and wide modulation", &["ambient", "reverb"], space),
        make_preset("Robot", "Ring-modulated vocal texture", &["vocal", "fx"], robot),
        make_preset("Whisper", "Airy formant shift", &["vocal", "fx"], whisper),
        make_preset("Bass", "Down-pitched distortion", &["bass", "distortion"], bass),
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

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aurivo-presets-v3-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn safe_name_sanitises_path_characters() {
        assert_eq!(safe_name("My Voice!"), "My_Voice");
        assert_eq!(safe_name("../../../etc/passwd"), "etc_passwd");
        assert_eq!(safe_name("!!!"), "untitled");
        assert_eq!(safe_name("A-B_9"), "A-B_9");
    }

    #[test]
    fn preset_round_trips_state_fields() {
        let dir = temp_dir("round");
        let mut params = BTreeMap::new();
        params.insert("gain_db".into(), 3.0);
        let preset = Preset {
            name: "My Voice!".into(),
            params,
            enabled: default_enabled(),
            chain: default_chain(),
            eq_bands: Vec::new(),
            description: "test".into(),
            tags: vec!["vocal".into()],
            version: PRESET_FORMAT_VERSION,
            created_at: 0,
        };
        save(dir.to_str().unwrap(), &preset).unwrap();
        let loaded = load(dir.to_str().unwrap(), "My Voice!").unwrap();
        assert_eq!(loaded.name, preset.name);
        assert_eq!(loaded.params.get("gain_db"), Some(&3.0));
        assert_eq!(loaded.description, "test");
        assert_eq!(loaded.tags, vec!["vocal"]);
        assert_eq!(list(dir.to_str().unwrap()).unwrap(), vec!["My Voice!"]);
        let detailed = list_detailed(dir.to_str().unwrap()).unwrap();
        assert_eq!(detailed[0].file, "My_Voice");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn old_presets_are_migrated() {
        let dir = temp_dir("migrate");
        let json = r#"{"name":"Old","params":{"gain_db":1.0}}"#;
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Old.json"), json).unwrap();
        let loaded = load(dir.to_str().unwrap(), "Old").unwrap();
        assert_eq!(loaded.version, PRESET_FORMAT_VERSION);
        assert_eq!(loaded.chain.len(), 12);
        assert_eq!(loaded.enabled.len(), 12);
        assert!(validate(&loaded).is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn validation_reports_bad_values() {
        let mut preset = Preset::default();
        preset.params.insert("gain_db".into(), f32::NAN);
        preset.name = " ".into();
        let problems = validate(&preset);
        assert!(problems.iter().any(|p| p.contains("empty")));
        assert!(problems.iter().any(|p| p.contains("gain_db")));
    }

    #[test]
    fn export_import_bundle_round_trip() {
        let dir = temp_dir("bundle");
        install_factory(dir.to_str().unwrap()).unwrap();
        let bundle = export_bundle(dir.to_str().unwrap()).unwrap();
        assert_eq!(bundle.presets.len(), 8);
        let target = temp_dir("bundle-target");
        assert_eq!(import_bundle(target.to_str().unwrap(), &bundle, false).unwrap(), 8);
        assert_eq!(import_bundle(target.to_str().unwrap(), &bundle, false).unwrap(), 0);
        assert_eq!(import_bundle(target.to_str().unwrap(), &bundle, true).unwrap(), 8);
        fs::remove_dir_all(dir).unwrap();
        fs::remove_dir_all(target).unwrap();
    }

    #[test]
    fn atomic_save_leaves_no_temp_file() {
        let dir = temp_dir("atomic");
        let preset = Preset::default();
        save(dir.to_str().unwrap(), &preset).unwrap();
        assert!(!dir.join("Untitled.json.tmp").exists());
        assert!(dir.join("Untitled.json").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn factory_presets_are_valid() {
        for preset in factory_presets() {
            assert!(validate(&preset).is_empty(), "{}: {:?}", preset.name, validate(&preset));
            assert!(!preset.params.contains_key("highpass"));
            assert_eq!(preset.chain.len(), preset.enabled.len());
        }
    }

    #[test]
    fn detailed_listing_is_sorted() {
        let dir = temp_dir("sorted");
        install_factory(dir.to_str().unwrap()).unwrap();
        let list = list_detailed(dir.to_str().unwrap()).unwrap();
        assert_eq!(list.len(), 8);
        for pair in list.windows(2) {
            assert!(pair[0].name.to_lowercase() <= pair[1].name.to_lowercase());
        }
        assert!(list.iter().all(|p| p.param_count > 5));
        fs::remove_dir_all(dir).unwrap();
    }
}
