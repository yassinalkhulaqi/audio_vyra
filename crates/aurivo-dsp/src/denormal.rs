//! Denormal (very small floating point) guard for feedback state in DSP hot
//! loops. Denormals can make floating-point operations 10–100× slower on many
//! CPUs; flushing them to zero costs one compare and keeps timers predictable.

pub const MIN_NORMAL: f32 = 1.0e-20;

#[inline(always)]
pub fn flush(x: f32) -> f32 {
    if x.abs() < MIN_NORMAL { 0.0 } else { x }
}
