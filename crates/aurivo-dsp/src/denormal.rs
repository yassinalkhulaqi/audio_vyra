//! Denormal (very small floating point) guard for feedback state in DSP hot
//! loops. Denormals can make floating-point operations 10–100× slower on many
//! CPUs; flushing them to zero costs one compare and keeps timers predictable.

/// f32's smallest normal value is about `1.18e-38`. We flush slightly above
/// that threshold so gradual underflow never reaches the slow path, while
/// keeping every audible signal (even -180 dBFS) untouched.
pub const MIN_NORMAL: f32 = 1.0e-37;

#[inline(always)]
pub fn flush(x: f32) -> f32 {
    if x.abs() < MIN_NORMAL { 0.0 } else { x }
}

#[inline(always)]
pub fn flush_slice(samples: &mut [f32]) {
    for s in samples {
        *s = flush(*s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn very_small_values_are_flushed() {
        assert_eq!(flush(1e-40), 0.0);
        assert_eq!(flush(-1e-40), 0.0);
        assert_eq!(flush(f32::from_bits(1)), 0.0);
    }

    #[test]
    fn audible_values_survive() {
        let quiet = 1e-12;
        assert_eq!(flush(quiet), quiet);
        assert_eq!(flush(-quiet), -quiet);
        assert_eq!(flush(1e-20), 1e-20);
    }

    #[test]
    fn non_finite_values_are_kept_for_sanitising_elsewhere() {
        assert!(flush(f32::NAN).is_nan());
        assert_eq!(flush(f32::INFINITY), f32::INFINITY);
    }

    #[test]
    fn slice_flushing() {
        let mut buf = [1e-40, 0.5, -1e-40, -0.25];
        flush_slice(&mut buf);
        assert_eq!(buf, [0.0, 0.5, 0.0, -0.25]);
    }
}
