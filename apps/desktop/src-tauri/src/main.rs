#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod presets;

use aurivo_audio_engine::AudioEngine;
use aurivo_core::LevelMeter;
use presets::Preset;
use std::collections::BTreeMap;
use std::sync::Mutex;
use tauri::State;

#[tauri::command]
fn list_input_devices(state: State<'_, Mutex<AudioEngine>>) -> Vec<String> {
    state.lock().unwrap().list_input_devices()
}
#[tauri::command]
fn list_output_devices(state: State<'_, Mutex<AudioEngine>>) -> Vec<String> {
    state.lock().unwrap().list_output_devices()
}
#[tauri::command]
fn start_engine(state: State<'_, Mutex<AudioEngine>>, input: Option<String>, output: Option<String>) -> Result<(), String> {
    state.lock().unwrap().start(input, output).map_err(|e| e.to_string())
}
#[tauri::command]
fn stop_engine(state: State<'_, Mutex<AudioEngine>>) {
    state.lock().unwrap().stop()
}
#[tauri::command]
fn engine_status(state: State<'_, Mutex<AudioEngine>>) -> bool {
    state.lock().unwrap().is_running()
}
#[tauri::command]
fn levels(state: State<'_, Mutex<AudioEngine>>) -> LevelMeter {
    state.lock().unwrap().levels()
}
#[tauri::command]
fn set_input_gain(state: State<'_, Mutex<AudioEngine>>, db: f32) {
    state.lock().unwrap().set_input_gain_db(db)
}
#[tauri::command]
fn list_presets(dir: String) -> Result<Vec<String>, String> {
    presets::list(&dir).map_err(|e| e.to_string())
}
#[tauri::command]
fn save_preset(dir: String, name: String, params: BTreeMap<String, f32>) -> Result<(), String> {
    presets::save(&dir, &name, params).map_err(|e| e.to_string())
}
#[tauri::command]
fn load_preset(dir: String, name: String) -> Result<Preset, String> {
    presets::load(&dir, &name).map_err(|e| e.to_string())
}
#[tauri::command]
fn delete_preset(dir: String, name: String) -> Result<(), String> {
    presets::delete(&dir, &name).map_err(|e| e.to_string())
}
#[tauri::command]
fn apply_preset(state: State<'_, Mutex<AudioEngine>>, params: BTreeMap<String, f32>) {
    let engine = state.lock().unwrap();
    if let Some(&db) = params.get("gain_db") {
        engine.set_input_gain_db(db);
    }
    const VOCAL_KEYS: [&str; 5] = ["vocal_mode", "vocal_carrier_hz", "vocal_vowel", "vocal_depth", "vocal_mix"];
    if VOCAL_KEYS.iter().any(|k| params.contains_key(*k)) {
        let cur = engine.vocal_params();
        let mode = params.get("vocal_mode").map(|&v| v as i32).unwrap_or(cur.0);
        let carrier_hz = params.get("vocal_carrier_hz").copied().unwrap_or(cur.1);
        let vowel = params.get("vocal_vowel").copied().unwrap_or(cur.2);
        let depth = params.get("vocal_depth").copied().unwrap_or(cur.3);
        let mix = params.get("vocal_mix").copied().unwrap_or(cur.4);
        engine.set_vocal(mode, carrier_hz, vowel, depth, mix);
    }
}
#[tauri::command]
fn set_vocal(state: State<'_, Mutex<AudioEngine>>, mode: i32, carrier_hz: f32, vowel: f32, depth: f32, mix: f32) {
    state.lock().unwrap().set_vocal(mode, carrier_hz, vowel, depth, mix)
}
#[tauri::command]
fn get_vocal(state: State<'_, Mutex<AudioEngine>>) -> (i32, f32, f32, f32, f32) {
    state.lock().unwrap().vocal_params()
}

fn main() {
    tauri::Builder::default()
        .manage(Mutex::new(AudioEngine::new()))
        .invoke_handler(tauri::generate_handler![
            list_input_devices, list_output_devices, start_engine, stop_engine, engine_status, levels, set_input_gain,
            list_presets, save_preset, load_preset, delete_preset, apply_preset,
            set_vocal, get_vocal
        ])
        .run(tauri::generate_context!())
        .expect("error running AURIVO");
}