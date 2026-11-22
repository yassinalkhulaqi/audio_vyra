#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aurivo_audio_engine::{AudioEngine, EqBandInfo};
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
fn set_delay(state: State<'_, Mutex<AudioEngine>>, time_ms: f32, feedback: f32, mix: f32, damp: f32) {
    state.lock().unwrap().set_delay(time_ms, feedback, mix, damp)
}
#[tauri::command]
fn get_delay(state: State<'_, Mutex<AudioEngine>>) -> (f32, f32, f32, f32) {
    state.lock().unwrap().delay_params()
}
#[tauri::command]
fn set_reverb(state: State<'_, Mutex<AudioEngine>>, room_size: f32, damp: f32, wet: f32, dry: f32) {
    state.lock().unwrap().set_reverb(room_size, damp, wet, dry)
}
#[tauri::command]
fn get_reverb(state: State<'_, Mutex<AudioEngine>>) -> (f32, f32, f32, f32) {
    state.lock().unwrap().reverb_params()
}
#[allow(clippy::too_many_arguments)]
#[tauri::command]
fn set_distortion(state: State<'_, Mutex<AudioEngine>>, mode: i32, drive_db: f32, tone: f32, mix: f32, downsample: f32, bit_depth: f32) {
    state.lock().unwrap().set_distortion(mode, drive_db, tone, mix, downsample, bit_depth)
}
#[tauri::command]
fn get_distortion(state: State<'_, Mutex<AudioEngine>>) -> (i32, f32, f32, f32, f32, f32) {
    state.lock().unwrap().distortion_params()
}
#[tauri::command]
fn set_modulation(state: State<'_, Mutex<AudioEngine>>, mode: i32, rate_hz: f32, depth: f32, feedback: f32, mix: f32) {
    state.lock().unwrap().set_modulation(mode, rate_hz, depth, feedback, mix)
}
#[tauri::command]
fn get_modulation(state: State<'_, Mutex<AudioEngine>>) -> (i32, f32, f32, f32, f32) {
    state.lock().unwrap().modulation_params()
}
#[tauri::command]
fn get_eq_bands(state: State<'_, Mutex<AudioEngine>>) -> Vec<EqBandInfo> {
    state.lock().unwrap().eq_bands()
}
#[tauri::command]
fn set_eq_band(state: State<'_, Mutex<AudioEngine>>, index: usize, kind: i32, frequency: f32, q: f32, gain_db: f32, enabled: bool) {
    state.lock().unwrap().set_eq_band(index, kind, frequency, q, gain_db, enabled)
}
#[tauri::command]
fn add_eq_band(state: State<'_, Mutex<AudioEngine>>, kind: i32, frequency: f32, q: f32, gain_db: f32) {
    state.lock().unwrap().add_eq_band(kind, frequency, q, gain_db)
}
#[tauri::command]
fn remove_eq_band(state: State<'_, Mutex<AudioEngine>>, index: usize) {
    state.lock().unwrap().remove_eq_band(index)
}
#[tauri::command]
fn set_pitch(state: State<'_, Mutex<AudioEngine>>, semitones: f32) {
    state.lock().unwrap().set_pitch_semitones(semitones)
}
#[tauri::command]
fn get_pitch(state: State<'_, Mutex<AudioEngine>>) -> f32 {
    state.lock().unwrap().pitch_semitones()
}
#[tauri::command]
fn spectrum(state: State<'_, Mutex<AudioEngine>>) -> Vec<f32> {
    state.lock().unwrap().spectrum()
}
#[tauri::command]
fn set_output_gain(state: State<'_, Mutex<AudioEngine>>, db: f32) {
    state.lock().unwrap().set_output_gain_db(db)
}
#[tauri::command]
fn get_output_gain(state: State<'_, Mutex<AudioEngine>>) -> f32 {
    state.lock().unwrap().output_gain_db()
}
#[tauri::command]
fn set_output_muted(state: State<'_, Mutex<AudioEngine>>, muted: bool) {
    state.lock().unwrap().set_output_muted(muted)
}
#[tauri::command]
fn is_output_muted(state: State<'_, Mutex<AudioEngine>>) -> bool {
    state.lock().unwrap().is_output_muted()
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
            set_delay, get_delay, set_reverb, get_reverb, set_distortion, get_distortion, set_modulation, get_modulation,
            get_eq_bands, set_eq_band, add_eq_band, remove_eq_band, set_pitch, get_pitch,
            spectrum, set_output_gain, get_output_gain, set_output_muted, is_output_muted, set_vocal, get_vocal
        ])
        .run(tauri::generate_context!())
        .expect("error running AURIVO");
}
