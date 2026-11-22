//! Small JSON config for device selection and startup parameters.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub params: BTreeMap<String, f32>,
}

impl Default for Config {
    fn default() -> Self {
        Self { input_device: None, output_device: None, params: BTreeMap::new() }
    }
}

pub fn load<P: AsRef<Path>>(path: P) -> io::Result<Config> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e),
    }
}

pub fn save<P: AsRef<Path>>(path: P, cfg: &Config) -> io::Result<()> {
    if let Some(dir) = path.as_ref().parent() {
        if !dir.as_os_str().is_empty() {
            fs::create_dir_all(dir)?;
        }
    }
    let text = serde_json::to_string_pretty(cfg)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    fs::write(path, text)
}
