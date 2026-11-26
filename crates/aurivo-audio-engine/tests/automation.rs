use aurivo_audio_engine::AudioEngine;

#[test]
fn automation_routes_can_be_added_and_removed() {
    let engine = AudioEngine::new();
    assert!(engine.add_automation("gain_db", 0, 2.0, 3.0));
    assert_eq!(engine.automation_routes().len(), 1);
    assert!(engine.remove_automation("gain_db"));
    assert!(engine.automation_routes().is_empty());
}

#[test]
fn unsupported_targets_are_rejected() {
    let engine = AudioEngine::new();
    assert!(!engine.add_automation("reverb_room", 0, 1.0, 0.5));
}

#[test]
fn ticking_changes_the_modulated_parameter() {
    let engine = AudioEngine::new();
    engine.set_input_gain_db(5.0);
    engine.add_automation("gain_db", 0, 1.0, 6.0);
    engine.tick_automation(0.25);
    let value = engine.state().input_gain_db;
    assert!((value - 11.0).abs() < 0.01, "expected ~11 dB, got {value}");
    engine.tick_automation(0.5);
    assert!((engine.state().input_gain_db - (-1.0)).abs() < 0.01);
}

#[test]
fn clearing_automation_restores_control() {
    let engine = AudioEngine::new();
    engine.set_input_gain_db(2.0);
    engine.add_automation("gain_db", 1, 1.0, 6.0);
    engine.tick_automation(0.5);
    assert_ne!(engine.state().input_gain_db, 2.0);
    engine.clear_automation();
    engine.tick_automation(0.1);
    assert!((engine.state().input_gain_db - 2.0).abs() < 1e-3);
}

#[test]
fn multiple_targets_are_independent() {
    let engine = AudioEngine::new();
    engine.set_input_gain_db(0.0);
    engine.set_pitch_semitones(0.0);
    engine.add_automation("gain_db", 0, 1.0, 4.0);
    engine.add_automation("pitch", 2, 1.0, 2.0);
    engine.tick_automation(0.25);
    let state = engine.state();
    assert!((state.input_gain_db - 4.0).abs() < 0.01);
    assert!((state.pitch_semitones - 2.0).abs() < 0.01);
}

#[test]
fn zero_and_negative_time_are_safe() {
    let engine = AudioEngine::new();
    engine.add_automation("pitch", 0, 1.0, 12.0);
    engine.tick_automation(0.0);
    engine.tick_automation(-1.0);
    assert!(engine.state().pitch_semitones.is_finite());
}
