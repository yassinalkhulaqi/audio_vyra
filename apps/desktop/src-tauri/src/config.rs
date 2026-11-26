//! Small JSON config for device selection and startup parameters.

use aurivo_audio_engine::AppState;
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
    #[serde(default)]
    pub state: Option<AppState>,
}

impl Default for Config {
    fn default() -> Self {
        Self { input_device: None, output_device: None, params: BTreeMap::new(), state: None }
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
    let final_path = path.as_ref();
    let temp_path = final_path.with_extension("json.tmp");
    fs::write(&temp_path, &text)?;
    match fs::rename(&temp_path, final_path) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = fs::remove_file(&temp_path);
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trips_full_state() {
        let dir = std::env::temp_dir().join(format!("aurivo-config-test-{}", std::process::id()));
        let path = dir.join("config.json");
        let cfg = Config {
            input_device: Some("mic".into()),
            output_device: Some("speakers".into()),
            params: BTreeMap::new(),
            state: Some(AppState::default()),
        };
        save(&path, &cfg).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.input_device.as_deref(), Some("mic"));
        assert_eq!(loaded.output_device.as_deref(), Some("speakers"));
        let state = loaded.state.expect("state should be saved");
        assert_eq!(state.chain.len(), 12);
        assert!(state.enabled.values().all(|v| *v));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_config_defaults_empty() {
        let path = std::env::temp_dir().join(format!("aurivo-missing-{}.json", std::process::id()));
        let cfg = load(&path).unwrap();
        assert!(cfg.state.is_none());
        assert!(cfg.input_device.is_none());
    }
}
