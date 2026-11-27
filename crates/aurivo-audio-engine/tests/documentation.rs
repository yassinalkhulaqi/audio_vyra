//! Tests that guard documentation claims against the implementation.

use aurivo_audio_engine::AudioEngine;

#[test]
fn readme_effect_count_matches_chain() {
    let engine = AudioEngine::new();
    assert_eq!(engine.effect_names().len(), 12);
}

#[test]
fn limiter_is_the_final_stage() {
    let engine = AudioEngine::new();
    assert_eq!(engine.effect_names().last().map(String::as_str), Some("limiter"));
}

#[test]
fn default_state_mentions_every_effect() {
    let engine = AudioEngine::new();
    let state = engine.state();
    for name in engine.effect_names() {
        assert!(state.enabled.contains_key(&name), "state missing {name}");
    }
}

#[test]
fn stream_info_reports_both_rates() {
    let engine = AudioEngine::new();
    let (input, output, channels) = engine.stream_info();
    assert_eq!(input, 48000);
    assert_eq!(output, 48000);
    assert_eq!(channels, 1);
}

#[test]
fn diagnostics_expose_documented_counters() {
    let engine = AudioEngine::new();
    assert_eq!(engine.audio_stats(), (0, 0));
    assert_eq!(engine.rt_stats().as_array(), [0, 0, 0, 0]);
    assert_eq!(engine.buffered_ms(), 0);
}
