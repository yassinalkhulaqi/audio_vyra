//! Mapping and default-value tests for the public DSP enums.

use aurivo_dsp::{
    AutomationRoute, BiquadType, EqBand, EqFilterKind, LfoShape,
    LimiterParams, VocalParams,
};

#[test]
fn eq_filter_kind_round_trips() {
    for kind in [
        EqFilterKind::Peaking,
        EqFilterKind::LowShelf,
        EqFilterKind::HighShelf,
        EqFilterKind::LowPass,
        EqFilterKind::HighPass,
    ] {
        assert_eq!(EqFilterKind::from_i32(kind.as_i32()), kind);
    }
}

#[test]
fn unknown_eq_kind_defaults_to_peaking() {
    assert_eq!(EqFilterKind::from_i32(-1), EqFilterKind::Peaking);
    assert_eq!(EqFilterKind::from_i32(99), EqFilterKind::Peaking);
}

#[test]
fn eq_band_constructors_set_expected_kinds() {
    assert_eq!(EqBand::peaking(1000.0, 1.0, 0.0).kind, EqFilterKind::Peaking);
    assert_eq!(EqBand::low_shelf(100.0, 0.0).kind, EqFilterKind::LowShelf);
    assert_eq!(EqBand::high_shelf(8000.0, 0.0).kind, EqFilterKind::HighShelf);
}

#[test]
fn lfo_shapes_round_trip() {
    for shape in [LfoShape::Sine, LfoShape::Triangle, LfoShape::Square, LfoShape::Saw, LfoShape::Random] {
        assert_eq!(LfoShape::from_i32(shape.as_i32()), shape);
    }
    assert_eq!(LfoShape::from_i32(-5), LfoShape::Sine);
    assert_eq!(LfoShape::from_i32(99), LfoShape::Sine);
}

#[test]
fn biquad_type_variants_are_comparable() {
    assert_ne!(BiquadType::LowPass, BiquadType::HighPass);
    assert_eq!(BiquadType::Peak { gain_db: 3 }, BiquadType::Peak { gain_db: 3 });
}

#[test]
fn limiter_defaults_are_sane() {
    let params = LimiterParams::default();
    assert!(params.threshold_db < 0.0);
    assert!(params.attack_ms > 0.0 && params.release_ms > params.attack_ms);
    assert!((0.0..=1.0).contains(&params.ceiling));
}

#[test]
fn vocal_defaults_are_in_range() {
    let params = VocalParams::default();
    assert!((0..=3).contains(&params.mode));
    assert!((0.0..=4.0).contains(&params.vowel));
    assert!((0.0..=1.0).contains(&params.mix));
}

#[test]
fn automation_route_stores_target() {
    let route = AutomationRoute::new("gain_db", LfoShape::Sine, 1.0, 2.0);
    assert_eq!(route.target, "gain_db");
    assert!(route.enabled);
}

#[test]
fn all_defaults_are_constructible() {
    let _ = EqBand::peaking(1000.0, 1.0, 0.0);
    let _ = VocalParams::default();
    let _ = LimiterParams::default();
}
