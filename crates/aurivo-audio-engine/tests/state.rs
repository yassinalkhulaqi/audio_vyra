use aurivo_audio_engine::{AppState, AudioEngine};

#[test]
fn state_captures_chain_and_bypass() {
    let engine = AudioEngine::new();
    assert_eq!(engine.state().chain.len(), 12);

    assert!(engine.set_effect_enabled("delay", false));
    assert!(!engine.effect_enabled("delay"));
    assert!(engine.move_effect("delay", 1));

    let snapshot = engine.state();
    assert_eq!(snapshot.chain[4], "reverb");
    assert_eq!(snapshot.chain[5], "delay");
    assert_eq!(snapshot.enabled.get("delay"), Some(&false));
    assert_eq!(snapshot.enabled.get("gain"), Some(&true));
}

#[test]
fn apply_state_restores_chain_bypass_and_eq_band_count() {
    let engine = AudioEngine::new();
    assert!(engine.set_effect_enabled("vocal", false));
    let reordered: Vec<String> = ["gain", "gate", "compressor", "highpass", "delay", "reverb",
        "distortion", "modulation", "eq", "pitch", "vocal", "limiter"]
        .iter()
        .rev()
        .map(|s| s.to_string())
        .collect();

    let mut snapshot = engine.state();
    snapshot.chain = reordered.clone();
    snapshot.eq_bands.truncate(2);
    engine.apply_state(&snapshot);

    assert_eq!(engine.effect_names(), reordered);
    assert!(!engine.effect_enabled("vocal"));
    assert_eq!(engine.eq_bands().len(), 2);
    assert_eq!(engine.state().chain, reordered);
}

#[test]
fn apply_state_adds_missing_eq_bands() {
    let engine = AudioEngine::new();
    let original_len = engine.eq_bands().len();
    let mut snapshot = engine.state();
    snapshot.eq_bands.clear();
    engine.apply_state(&snapshot);
    assert_eq!(engine.eq_bands().len(), 0);

    let snapshot = AppState {
        eq_bands: vec![
            aurivo_audio_engine::EqBandInfo {
                kind: 0,
                frequency: 1000.0,
                q: 1.0,
                gain_db: 3.0,
                enabled: true,
            };
            original_len + 2
        ],
        ..AppState::default()
    };
    engine.apply_state(&snapshot);
    assert_eq!(engine.eq_bands().len(), original_len + 2);
}

#[test]
fn stop_ends_recording_and_playback() {
    let mut engine = AudioEngine::new();
    engine.start_recording();
    assert!(engine.is_recording());
    engine.stop();
    assert!(!engine.is_recording());
    assert!(!engine.is_running());
}
