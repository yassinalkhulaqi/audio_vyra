//! JSON preset persistence for effect parameter sets.
//!
//! A preset is a name plus a flat map of named numeric parameters, so it can
//! describe any combination of effects without a rigid schema. Files are
//! stored as `<dir>/<safe-name>.json`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub params: BTreeMap<String, f32>,
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

pub fn save(dir: &str, name: &str, params: BTreeMap<String, f32>) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let preset = Preset { name: name.to_string(), params };
    let text = serde_json::to_string_pretty(&preset).map_err(json_error)?;
    fs::write(preset_path(dir, name), text)
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
