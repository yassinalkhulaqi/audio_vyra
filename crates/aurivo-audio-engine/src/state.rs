//! Complete serializable snapshot of everything the UI can change. Used for
//! get_state / set_state commands and as the basis for full-state presets.

use crate::EqBandInfo;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub input_gain_db: f32,
    pub output_gain_db: f32,
    pub output_muted: bool,
    pub chain: Vec<String>,
    pub enabled: BTreeMap<String, bool>,
    pub delay: [f32; 4],
    pub reverb: [f32; 4],
    pub distortion: [f32; 6],
    pub modulation: [f32; 5],
    pub pitch_semitones: f32,
    pub eq_bands: Vec<EqBandInfo>,
    pub vocal: [f32; 5],
}

impl Default for AppState {
    fn default() -> Self {
        let chain = vec![
            "gain", "gate", "compressor", "highpass", "delay", "reverb",
            "distortion", "modulation", "eq", "pitch", "vocal",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        let enabled = chain.iter().map(|n| (n.clone(), true)).collect();
        Self {
            input_gain_db: 0.0,
            output_gain_db: 0.0,
            output_muted: false,
            chain,
            enabled,
            delay: [300.0, 0.35, 0.3, 0.2],
            reverb: [0.6, 0.4, 0.25, 0.85],
            distortion: [0.0, 12.0, 0.6, 0.5, 1.0, 8.0],
            modulation: [0.0, 1.5, 0.4, 0.3, 0.5],
            pitch_semitones: 0.0,
            eq_bands: Vec::new(),
            vocal: [0.0, 40.0, 0.0, 0.6, 0.6],
        }
    }
}
