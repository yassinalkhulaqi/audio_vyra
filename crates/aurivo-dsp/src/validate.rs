//! Defensive parameter sanitising for public DSP setters.
//!
//! Every effect exposes plain `pub` parameter fields or accepts values from a
//! JSON preset. A malformed preset must never produce NaN, infinity, or a
//! division by zero in the audio callback. These helpers give every effect a
//! single, consistent definition of "safe".

/// Clamp a time constant in milliseconds to a usable range.
pub fn safe_ms(value: f32) -> f32 {
    if value.is_finite() { value.clamp(0.0, 5000.0) } else { 0.0 }
}

/// Clamp a compressor ratio. Values below 1:1 are expanded, so they are
/// replaced by a neutral 1:1 rather than producing a division by zero.
pub fn safe_ratio(value: f32) -> f32 {
    if value.is_finite() && value >= 1.0 { value.clamp(1.0, 100.0) } else { 1.0 }
}

/// Clamp a dB value.
pub fn safe_db(value: f32) -> f32 {
    if value.is_finite() { value.clamp(-120.0, 36.0) } else { 0.0 }
}

/// Clamp a 0..1 mix or depth parameter.
pub fn safe_unit(value: f32) -> f32 {
    if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.0 }
}

/// Clamp a frequency to the Nyquist range of the current sample rate.
pub fn safe_frequency(value: f32, sample_rate: f32) -> f32 {
    let sr = if sample_rate.is_finite() && sample_rate >= 64.0 { sample_rate.max(64.0) } else { 48000.0 };
    let max = sr * 0.5 - 10.0;
    if value.is_finite() { value.clamp(10.0, max) } else { 1000.0 }
}

/// Clamp an integer mode into a known range.
pub fn safe_mode(value: i32, max: i32) -> i32 {
    if value >= 0 && value <= max { value } else { 0 }
}

/// Convert a possibly non-finite f32 mode into a valid range.
pub fn safe_mode_f32(value: f32, max: i32) -> i32 {
    if value.is_finite() && value >= 0.0 && value <= max as f32 {
        value as i32
    } else {
        0
    }
}

/// One-pole envelope coefficient for a time constant in milliseconds.
#[inline]
pub fn envelope_coefficient(sample_rate: f32, ms: f32) -> f32 {
    let ms = safe_ms(ms);
    if ms <= 0.0 {
        0.0
    } else {
        (-1.0 / (sample_rate.max(1.0) * ms / 1000.0)).exp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_constants_never_produce_infinity() {
        for ms in [-10.0, 0.0, 5.0, f32::NAN, f32::INFINITY] {
            let c = envelope_coefficient(48000.0, ms);
            assert!(c.is_finite());
            assert!((0.0..=1.0).contains(&c));
        }
    }

    #[test]
    fn ratio_has_no_zero_division() {
        for r in [-1.0, 0.0, 0.5, 1.0, 4.0, f32::NAN] {
            let safe = safe_ratio(r);
            assert!(safe >= 1.0);
            assert!(safe.is_finite());
            assert!(1.0 / safe <= 1.0);
        }
    }

    #[test]
    fn frequencies_respect_nyquist() {
        assert!((safe_frequency(30000.0, 44100.0) - (22050.0 - 10.0)).abs() < 1e-3);
        assert_eq!(safe_frequency(20000.0, 44100.0), 20000.0);
        assert_eq!(safe_frequency(-5.0, 48000.0), 10.0);
        assert_eq!(safe_frequency(f32::NAN, 0.0), 1000.0);
    }

    #[test]
    fn modes_are_bounded() {
        assert_eq!(safe_mode(-1, 3), 0);
        assert_eq!(safe_mode(2, 3), 2);
        assert_eq!(safe_mode(9, 3), 0);
        assert_eq!(safe_mode_f32(f32::NAN, 3), 0);
        assert_eq!(safe_mode_f32(2.9, 3), 2);
    }

    #[test]
    fn unit_and_db_are_finite() {
        assert_eq!(safe_unit(f32::INFINITY), 0.0);
        assert_eq!(safe_unit(2.0), 1.0);
        assert_eq!(safe_db(f32::NAN), 0.0);
        assert_eq!(safe_db(-500.0), -120.0);
    }
}
