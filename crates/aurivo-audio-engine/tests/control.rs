use aurivo_audio_engine::AudioEngine;
use aurivo_audio_engine::control::{ControlBinding, ControlMap, ControlMessage};

#[test]
fn engine_applies_bound_control() {
    let engine = AudioEngine::new();
    assert!(engine.bind_control(0, 7, "gain_db", -24.0, 24.0));
    assert_eq!(engine.send_control(0, 7, 1.0), 1);
    assert!((engine.state().input_gain_db - 24.0).abs() < 0.01);
}

#[test]
fn unbound_control_is_ignored() {
    let engine = AudioEngine::new();
    assert_eq!(engine.send_control(0, 7, 0.5), 0);
}

#[test]
fn unbinding_stops_control() {
    let engine = AudioEngine::new();
    engine.bind_control(0, 7, "pitch", -12.0, 12.0);
    assert!(engine.unbind_control(0, 7));
    assert_eq!(engine.send_control(0, 7, 1.0), 0);
}

#[test]
fn unsupported_target_is_rejected() {
    let engine = AudioEngine::new();
    assert!(!engine.bind_control(0, 7, "reverb_room", 0.0, 1.0));
}

#[test]
fn multiple_bindings_can_share_one_controller() {
    let mut map = ControlMap::new();
    map.add(ControlBinding::new(0, 7, "gain_db", -24.0, 24.0));
    map.add(ControlBinding::new(0, 7, "pitch", -12.0, 12.0));
    let applied = map.apply(&ControlMessage::new(0, 7, 0.5));
    assert_eq!(applied.len(), 2);
}

#[test]
fn clear_removes_all_bindings() {
    let engine = AudioEngine::new();
    engine.bind_control(0, 7, "gain_db", -24.0, 24.0);
    engine.clear_controls();
    assert!(engine.control_bindings().is_empty());
}
