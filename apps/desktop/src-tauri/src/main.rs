#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod paths;
mod presets;

use aurivo_audio_engine::{AppState, AudioEngine, EqBandInfo};
use config::Config;
use aurivo_core::LevelMeter;
use presets::Preset;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

fn app_root(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn config_path(app: &AppHandle) -> PathBuf {
    app_root(app).join("config.json")
}

fn user_path(app: &AppHandle, input: &str) -> Result<PathBuf, String> {
    paths::resolve_under(&app_root(app), input).map_err(|e| e.to_string())
}

#[tauri::command]
fn default_paths(_app: AppHandle) -> (String, String, String) {
    (
        "config.json".into(),
        "presets".into(),
        "recording.wav".into(),
    )
}

fn preset_to_state(p: &Preset) -> AppState {
    let mut s = AppState::default();
    let get = |k: &str| p.params.get(k).copied();
    if let Some(v) = get("gain_db") { s.input_gain_db = v; }
    if let Some(v) = get("output_gain_db") { s.output_gain_db = v; }
    if let Some(v) = get("output_muted") { s.output_muted = v != 0.0; }
    if let Some(v) = get("delay_time") { s.delay[0] = v; }
    if let Some(v) = get("delay_feedback") { s.delay[1] = v; }
    if let Some(v) = get("delay_mix") { s.delay[2] = v; }
    if let Some(v) = get("delay_damp") { s.delay[3] = v; }
    if let Some(v) = get("reverb_room") { s.reverb[0] = v; }
    if let Some(v) = get("reverb_damp") { s.reverb[1] = v; }
    if let Some(v) = get("reverb_wet") { s.reverb[2] = v; }
    if let Some(v) = get("reverb_dry") { s.reverb[3] = v; }
    if let Some(v) = get("dist_mode") { s.distortion[0] = v; }
    if let Some(v) = get("dist_drive") { s.distortion[1] = v; }
    if let Some(v) = get("dist_tone") { s.distortion[2] = v; }
    if let Some(v) = get("dist_mix") { s.distortion[3] = v; }
    if let Some(v) = get("dist_down") { s.distortion[4] = v; }
    if let Some(v) = get("dist_bits") { s.distortion[5] = v; }
    if let Some(v) = get("mod_mode") { s.modulation[0] = v; }
    if let Some(v) = get("mod_rate") { s.modulation[1] = v; }
    if let Some(v) = get("mod_depth") { s.modulation[2] = v; }
    if let Some(v) = get("mod_fb") { s.modulation[3] = v; }
    if let Some(v) = get("mod_mix") { s.modulation[4] = v; }
    if let Some(v) = get("pitch") { s.pitch_semitones = v; }
    if let Some(v) = get("vocal_mode") { s.vocal[0] = v; }
    if let Some(v) = get("vocal_carrier") { s.vocal[1] = v; }
    if let Some(v) = get("vowel") { s.vocal[2] = v; }
    if let Some(v) = get("vocal_depth") { s.vocal[3] = v; }
    if let Some(v) = get("vocal_mix") { s.vocal[4] = v; }
    if !p.chain.is_empty() { s.chain = p.chain.clone(); }
    if !p.enabled.is_empty() { s.enabled = p.enabled.clone(); }
    if !p.eq_bands.is_empty() { s.eq_bands = p.eq_bands.clone(); }
    s
}

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
fn start_recording(state: State<'_, Mutex<AudioEngine>>) {
    state.lock().unwrap().start_recording()
}
#[tauri::command]
fn stop_recording(state: State<'_, Mutex<AudioEngine>>) {
    state.lock().unwrap().stop_recording()
}
#[tauri::command]
fn is_recording(state: State<'_, Mutex<AudioEngine>>) -> bool {
    state.lock().unwrap().is_recording()
}
#[tauri::command]
fn recorded_samples(state: State<'_, Mutex<AudioEngine>>) -> usize {
    state.lock().unwrap().recorded_samples()
}
#[tauri::command]
fn save_recording(app: AppHandle, state: State<'_, Mutex<AudioEngine>>, path: String) -> Result<(), String> {
    let path = user_path(&app, &path)?;
    state.lock().unwrap().save_recording(&path.to_string_lossy()).map_err(|e| e.to_string())
}
#[tauri::command]
fn play_wav(app: AppHandle, state: State<'_, Mutex<AudioEngine>>, path: String) -> Result<(), String> {
    let path = user_path(&app, &path)?;
    state.lock().unwrap().play_wav(&path.to_string_lossy()).map_err(|e| e.to_string())
}
#[tauri::command]
fn stop_playback(state: State<'_, Mutex<AudioEngine>>) {
    state.lock().unwrap().stop_playback()
}
#[tauri::command]
fn list_presets(dir: String) -> Result<Vec<String>, String> {
    presets::list(&dir).map_err(|e| e.to_string())
}
#[tauri::command]
fn save_preset(dir: String, preset: Preset) -> Result<(), String> {
    presets::save(&dir, &preset).map_err(|e| e.to_string())
}
#[tauri::command]
fn install_factory_presets(dir: String) -> Result<Vec<String>, String> {
    presets::install_factory(&dir).map_err(|e| e.to_string())
}
#[tauri::command]
fn export_presets(dir: String) -> Result<presets::PresetBundle, String> {
    presets::export_bundle(&dir).map_err(|e| e.to_string())
}
#[tauri::command]
fn import_presets(dir: String, bundle: presets::PresetBundle, overwrite: bool) -> Result<usize, String> {
    presets::import_bundle(&dir, &bundle, overwrite).map_err(|e| e.to_string())
}
#[tauri::command]
fn export_presets_to_file(dir: String, path: String) -> Result<(), String> {
    let bundle = presets::export_bundle(&dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(&bundle).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}
#[tauri::command]
fn import_presets_from_file(dir: String, path: String, overwrite: bool) -> Result<usize, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let bundle: presets::PresetBundle = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    presets::import_bundle(&dir, &bundle, overwrite).map_err(|e| e.to_string())
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
fn apply_preset(state: State<'_, Mutex<AudioEngine>>, preset: Preset) {
    let engine = state.lock().unwrap();
    let mut snapshot = preset_to_state(&preset);
    if snapshot.eq_bands.is_empty() {
        snapshot.eq_bands = engine.eq_bands();
    }
    engine.apply_state(&snapshot);
}
#[tauri::command]
fn list_effects(state: State<'_, Mutex<AudioEngine>>) -> Vec<String> {
    state.lock().unwrap().effect_names()
}
#[tauri::command]
fn set_effect_enabled(state: State<'_, Mutex<AudioEngine>>, name: String, enabled: bool) -> bool {
    state.lock().unwrap().set_effect_enabled(&name, enabled)
}
#[tauri::command]
fn effect_enabled(state: State<'_, Mutex<AudioEngine>>, name: String) -> bool {
    state.lock().unwrap().effect_enabled(&name)
}
#[tauri::command]
fn remove_effect(state: State<'_, Mutex<AudioEngine>>, name: String) -> bool {
    state.lock().unwrap().remove_effect(&name)
}
#[tauri::command]
fn move_effect(state: State<'_, Mutex<AudioEngine>>, name: String, delta: i32) -> bool {
    state.lock().unwrap().move_effect(&name, delta)
}
#[tauri::command]
fn get_state(state: State<'_, Mutex<AudioEngine>>) -> AppState {
    state.lock().unwrap().state()
}
#[tauri::command]
fn set_state(state: State<'_, Mutex<AudioEngine>>, new_state: AppState) {
    state.lock().unwrap().apply_state(&new_state);
}
#[tauri::command]
fn load_config(app: AppHandle) -> Result<Config, String> {
    config::load(config_path(&app)).map_err(|e| e.to_string())
}
#[tauri::command]
fn save_config(app: AppHandle, cfg: Config) -> Result<(), String> {
    config::save(config_path(&app), &cfg).map_err(|e| e.to_string())
}
#[tauri::command]
fn stream_info(state: State<'_, Mutex<AudioEngine>>) -> (u32, u32, u16) {
    state.lock().unwrap().stream_info()
}
#[tauri::command]
fn audio_stats(state: State<'_, Mutex<AudioEngine>>) -> (u64, u64) {
    state.lock().unwrap().audio_stats()
}
#[tauri::command]
fn reset_audio_stats(state: State<'_, Mutex<AudioEngine>>) {
    state.lock().unwrap().reset_audio_stats()
}
#[tauri::command]
fn reset_clip(state: State<'_, Mutex<AudioEngine>>) {
    state.lock().unwrap().reset_clip()
}
#[tauri::command]
fn rt_stats(state: State<'_, Mutex<AudioEngine>>) -> [u64; 4] {
    state.lock().unwrap().rt_stats().as_array()
}
#[tauri::command]
fn reset_rt_stats(state: State<'_, Mutex<AudioEngine>>) {
    state.lock().unwrap().reset_rt_stats()
}
#[tauri::command]
fn control_bindings(state: State<'_, Mutex<AudioEngine>>) -> Vec<aurivo_audio_engine::control::ControlBinding> {
    state.lock().unwrap().control_bindings()
}
#[tauri::command]
fn bind_control(state: State<'_, Mutex<AudioEngine>>, channel: u8, controller: u8, target: String, min: f32, max: f32) -> bool {
    state.lock().unwrap().bind_control(channel, controller, &target, min, max)
}
#[tauri::command]
fn unbind_control(state: State<'_, Mutex<AudioEngine>>, channel: u8, controller: u8) -> bool {
    state.lock().unwrap().unbind_control(channel, controller)
}
#[tauri::command]
fn clear_controls(state: State<'_, Mutex<AudioEngine>>) {
    state.lock().unwrap().clear_controls()
}
#[tauri::command]
fn send_control(state: State<'_, Mutex<AudioEngine>>, channel: u8, controller: u8, value: f32) -> usize {
    state.lock().unwrap().send_control(channel, controller, value)
}
#[tauri::command]
fn automation_routes(state: State<'_, Mutex<AudioEngine>>) -> Vec<(String, i32, f32, f32, bool)> {
    state.lock().unwrap().automation_routes()
}
#[tauri::command]
fn add_automation(state: State<'_, Mutex<AudioEngine>>, target: String, shape: i32, rate: f32, depth: f32) -> bool {
    state.lock().unwrap().add_automation(&target, shape, rate, depth)
}
#[tauri::command]
fn remove_automation(state: State<'_, Mutex<AudioEngine>>, target: String) -> bool {
    state.lock().unwrap().remove_automation(&target)
}
#[tauri::command]
fn clear_automation(state: State<'_, Mutex<AudioEngine>>) {
    state.lock().unwrap().clear_automation()
}
#[tauri::command]
fn tick_automation(state: State<'_, Mutex<AudioEngine>>, seconds: f32) {
    state.lock().unwrap().tick_automation(seconds)
}
#[tauri::command]
fn buffered_ms(state: State<'_, Mutex<AudioEngine>>) -> u32 {
    state.lock().unwrap().buffered_ms()
}
#[tauri::command]
fn reorder_effects(state: State<'_, Mutex<AudioEngine>>, names: Vec<String>) -> bool {
    state.lock().unwrap().reorder_effects(&names)
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
        .setup(|app| {
            let cfg = config::load(config_path(app.handle())).unwrap_or_default();
            let engine = app.state::<Mutex<AudioEngine>>();
            let engine = engine.lock().unwrap();
            if let Some(s) = cfg.state {
                engine.apply_state(&s);
            } else if let Some(db) = cfg.params.get("gain_db") {
                engine.set_input_gain_db(*db);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_input_devices, list_output_devices, start_engine, stop_engine, engine_status, levels, set_input_gain,
            set_delay, get_delay, set_reverb, get_reverb, set_distortion, get_distortion, set_modulation, get_modulation,
            get_eq_bands, set_eq_band, add_eq_band, remove_eq_band, set_pitch, get_pitch,
            spectrum, set_output_gain, get_output_gain, set_output_muted, is_output_muted,
            start_recording, stop_recording, is_recording, recorded_samples, save_recording, play_wav, stop_playback,
            list_presets, save_preset, load_preset, delete_preset, install_factory_presets, export_presets, import_presets, export_presets_to_file, import_presets_from_file, apply_preset, get_state, set_state, load_config, save_config, stream_info,
            audio_stats, reset_audio_stats, reset_clip, rt_stats, reset_rt_stats, buffered_ms, default_paths,
            automation_routes, add_automation, remove_automation, clear_automation, tick_automation,
            control_bindings, bind_control, unbind_control, clear_controls, send_control,
            list_effects, set_effect_enabled, effect_enabled, remove_effect, move_effect, reorder_effects, set_vocal, get_vocal
        ])
        .run(tauri::generate_context!())
        .expect("error running AURIVO");
}
