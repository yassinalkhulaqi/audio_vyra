#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aurivo_audio_engine::AudioEngine;
use aurivo_core::LevelMeter;
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
            set_vocal, get_vocal
        ])
        .run(tauri::generate_context!())
        .expect("error running AURIVO");
}
