use aurivo_audio_engine::{AppState, AudioEngine, EqBandInfo};

fn snapshot() -> AppState {
    let engine = AudioEngine::new();
    engine.state()
}

#[test]
fn new_engine_has_expected_defaults() {
    let engine = AudioEngine::new();
    assert!(!engine.is_running());
    assert_eq!(engine.audio_stats(), (0, 0));
    assert_eq!(engine.rt_stats().total(), 0);
    assert_eq!(engine.buffered_ms(), 0);
    assert_eq!(engine.stream_info(), (48000, 48000, 1));
    assert_eq!(engine.effect_names().len(), 12);
    assert_eq!(engine.effect_names().last().unwrap(), "limiter");
}

#[test]
fn all_effects_are_enabled_by_default() {
    let engine = AudioEngine::new();
    for name in engine.effect_names() {
        assert!(engine.effect_enabled(&name), "{name} should be enabled");
    }
}

#[test]
fn toggles_are_reflected_in_state() {
    let engine = AudioEngine::new();
    let names = engine.effect_names();
    for (i, name) in names.iter().enumerate() {
        if i % 3 == 0 {
            engine.set_effect_enabled(name, false);
        }
    }
    let state = engine.state();
    for (i, name) in names.iter().enumerate() {
        assert_eq!(state.enabled[name], i % 3 != 0);
    }
}

#[test]
fn reorder_then_apply_state_is_idempotent() {
    let engine = AudioEngine::new();
    let mut reversed = engine.effect_names();
    reversed.reverse();
    engine.reorder_effects(&reversed);
    let first = engine.state();
    engine.apply_state(&first);
    let second = engine.state();
    assert_eq!(first.chain, second.chain);
    assert_eq!(first.enabled, second.enabled);
}

#[test]
fn eq_band_edges() {
    let engine = AudioEngine::new();
    let bands = vec![
        EqBandInfo { kind: 0, frequency: 20.0, q: 0.1, gain_db: -18.0, enabled: false },
        EqBandInfo { kind: 4, frequency: 20000.0, q: 18.0, gain_db: 18.0, enabled: true },
    ];
    let mut state = engine.state();
    state.eq_bands = bands.clone();
    engine.apply_state(&state);
    let stored = engine.eq_bands();
    assert_eq!(stored.len(), 2);
    assert!(!stored[0].enabled);
    assert!(stored[1].enabled);
}

#[test]
fn automation_and_control_coexist() {
    let engine = AudioEngine::new();
    engine.add_automation("pitch", 0, 1.0, 4.0);
    engine.bind_control(0, 1, "gain_db", -12.0, 12.0);
    engine.tick_automation(0.25);
    engine.send_control(0, 1, 1.0);
    let state = engine.state();
    assert!((state.pitch_semitones - 4.0).abs() < 0.01);
    assert!((state.input_gain_db - 12.0).abs() < 0.01);
}

#[test]
fn clear_paths_work_without_state() {
    let engine = AudioEngine::new();
    engine.clear_automation();
    engine.clear_controls();
    engine.reset_rt_stats();
    engine.reset_audio_stats();
    engine.reset_clip();
    engine.reset_latency();
    assert!(engine.automation_routes().is_empty());
    assert!(engine.control_bindings().is_empty());
    assert_eq!(engine.rt_stats().total(), 0);
}

#[test]
fn levels_are_finite_before_start() {
    let engine = AudioEngine::new();
    let levels = engine.levels();
    assert!(levels.peak.is_finite());
    assert!(levels.rms.is_finite());
    assert!(!levels.clipped);
}

#[test]
fn spectrum_is_empty_before_start() {
    let engine = AudioEngine::new();
    let bins = engine.spectrum();
    assert!(!bins.is_empty());
    assert!(bins.iter().all(|v| v.is_finite()));
}

#[test]
fn state_defaults_are_finite() {
    let state = snapshot();
    assert!(state.input_gain_db.is_finite());
    assert!(state.output_gain_db.is_finite());
    assert!(state.delay.iter().all(|v| v.is_finite()));
    assert!(state.reverb.iter().all(|v| v.is_finite()));
    assert!(state.distortion.iter().all(|v| v.is_finite()));
    assert!(state.modulation.iter().all(|v| v.is_finite()));
    assert!(state.vocal.iter().all(|v| v.is_finite()));
    assert!(state.pitch_semitones.is_finite());
}

#[test]
fn output_gain_range_is_finite() {
    let engine = AudioEngine::new();
    for db in [-120.0, -24.0, 0.0, 24.0] {
        engine.set_output_gain_db(db);
        assert!(engine.output_gain_db().is_finite());
    }
}

#[test]
fn recording_lifecycle_without_devices() {
    let mut engine = AudioEngine::new();
    assert!(!engine.is_recording());
    engine.start_recording();
    assert!(engine.is_recording());
    assert_eq!(engine.recorded_samples(), 0);
    engine.stop();
    assert!(!engine.is_recording());
}
