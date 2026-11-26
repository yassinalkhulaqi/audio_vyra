use aurivo_audio_engine::{AppState, AudioEngine, EqBandInfo};

fn band(kind: i32, freq: f32, gain: f32) -> EqBandInfo {
    EqBandInfo { kind, frequency: freq, q: 1.0, gain_db: gain, enabled: true }
}

#[test]
fn state_round_trip_is_stable_over_many_iterations() {
    let engine = AudioEngine::new();
    let mut snapshot = engine.state();
    snapshot.input_gain_db = -7.5;
    snapshot.output_gain_db = 3.0;
    snapshot.output_muted = true;
    snapshot.delay = [250.0, 0.5, 0.4, 0.1];
    snapshot.reverb = [0.7, 0.3, 0.2, 0.9];
    snapshot.distortion = [1.0, 20.0, 0.5, 0.6, 4.0, 12.0];
    snapshot.modulation = [2.0, 3.0, 0.6, 0.2, 0.7];
    snapshot.pitch_semitones = -5.0;
    snapshot.vocal = [3.0, 120.0, 2.0, 0.8, 0.4];

    for _ in 0..10 {
        engine.apply_state(&snapshot);
        let again = engine.state();
        assert_eq!(again.input_gain_db, snapshot.input_gain_db);
        assert_eq!(again.delay, snapshot.delay);
        assert_eq!(again.reverb, snapshot.reverb);
        assert_eq!(again.distortion[0] as i32, 1);
        assert_eq!(again.modulation[0] as i32, 2);
        assert_eq!(again.vocal[0] as i32, 3);
        assert_eq!(again.chain, snapshot.chain);
        assert_eq!(again.enabled, snapshot.enabled);
        snapshot = again;
    }
}

#[test]
fn bypass_matrix_survives_reordering() {
    let engine = AudioEngine::new();
    let names = engine.effect_names();
    for name in names.iter().step_by(2) {
        engine.set_effect_enabled(name, false);
    }
    let mut reversed = names.clone();
    reversed.reverse();
    engine.reorder_effects(&reversed);
    for name in names.iter().step_by(2) {
        assert!(!engine.effect_enabled(name), "{name} lost its bypass state");
    }
    for name in names.iter().skip(1).step_by(2) {
        assert!(engine.effect_enabled(name), "{name} was unexpectedly bypassed");
    }
}

#[test]
fn eq_band_count_and_values_round_trip() {
    let engine = AudioEngine::new();
    let mut snapshot = engine.state();
    snapshot.eq_bands.clear();
    snapshot.eq_bands.push(band(0, 120.0, 3.0));
    snapshot.eq_bands.push(band(1, 900.0, -4.5));
    snapshot.eq_bands.push(band(2, 5000.0, 2.0));
    engine.apply_state(&snapshot);
    let bands = engine.eq_bands();
    assert_eq!(bands.len(), 3);
    assert_eq!(bands[1].kind, snapshot.eq_bands[1].kind);
    assert!((bands[1].gain_db + 4.5).abs() < 1e-4);
}

#[test]
fn partial_state_defaults_do_not_destroy_chain() {
    let engine = AudioEngine::new();
    let partial = AppState {
        input_gain_db: 2.0,
        chain: Vec::new(),
        enabled: Default::default(),
        ..AppState::default()
    };
    engine.apply_state(&partial);
    assert!(!engine.effect_names().is_empty());
    assert!(engine.effect_enabled("gain"));
    assert!((engine.state().input_gain_db - 2.0).abs() < 1e-4);
}

#[test]
fn out_of_range_mode_values_are_sanitised() {
    let engine = AudioEngine::new();
    let mut snapshot = engine.state();
    snapshot.distortion[0] = 999.0;
    snapshot.modulation[0] = -5.0;
    snapshot.vocal[0] = f32::NAN;
    engine.apply_state(&snapshot);
    let d = engine.distortion_params();
    let m = engine.modulation_params();
    let v = engine.vocal_params();
    assert_eq!(d.0, 0);
    assert_eq!(m.0, 0);
    assert_eq!(v.0, 0);
}

#[test]
fn output_gain_and_mute_round_trip() {
    let engine = AudioEngine::new();
    for db in [-24.0, -6.0, 0.0, 6.0, 12.0] {
        let mut snapshot = engine.state();
        snapshot.output_gain_db = db;
        snapshot.output_muted = db < 0.0;
        engine.apply_state(&snapshot);
        let again = engine.state();
        assert!((again.output_gain_db - db).abs() < 0.01);
        assert_eq!(again.output_muted, db < 0.0);
    }
}

#[test]
fn chain_reorder_rejects_invalid_permutations() {
    let engine = AudioEngine::new();
    let original = engine.effect_names();
    let mut missing = original.clone();
    missing.pop();
    assert!(!engine.reorder_effects(&missing));
    assert_eq!(engine.effect_names(), original);

    let mut duplicate = original.clone();
    duplicate[1] = duplicate[0].clone();
    assert!(!engine.reorder_effects(&duplicate));
    assert_eq!(engine.effect_names(), original);
}

#[test]
fn move_effect_clamps_at_boundaries() {
    let engine = AudioEngine::new();
    let first = engine.effect_names()[0].clone();
    assert!(engine.move_effect(&first, -100));
    assert_eq!(engine.effect_names()[0], first);
    let last = engine.effect_names().last().unwrap().clone();
    assert!(engine.move_effect(&last, 100));
    assert_eq!(engine.effect_names().last().unwrap(), &last);
    assert!(!engine.move_effect("does-not-exist", 1));
}

#[test]
fn unknown_effect_toggles_are_no_ops() {
    let engine = AudioEngine::new();
    assert!(!engine.set_effect_enabled("nope", false));
    assert!(!engine.effect_enabled("nope"));
}

#[test]
fn state_is_deterministic_after_stop() {
    let mut engine = AudioEngine::new();
    let before = engine.state();
    engine.start_recording();
    engine.stop();
    let after = engine.state();
    assert_eq!(before.chain, after.chain);
    assert_eq!(before.enabled, after.enabled);
    assert!(!engine.is_recording());
}

#[test]
fn stats_reset_is_independent_of_state() {
    let engine = AudioEngine::new();
    let snapshot = engine.state();
    engine.reset_audio_stats();
    engine.reset_rt_stats();
    assert_eq!(engine.audio_stats(), (0, 0));
    assert_eq!(engine.state().chain, snapshot.chain);
}
