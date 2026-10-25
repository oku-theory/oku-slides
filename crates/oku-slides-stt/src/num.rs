//! Numeric conversions for audio: counts, durations and sample positions.
//!
//! Every lossy conversion lives here, with its reason, rather than in scattered `as`.
//! The names are those of `oku_slides_core::num` (which this crate does not depend on):
//! `trunc` truncates toward zero, and `as` saturates at the bounds (negative or NaN gives 0).

use crate::audio::SAMPLE_RATE_F32;

/// A count (samples, characters, channels) as `f32`.
#[expect(
    clippy::cast_precision_loss,
    reason = "beyond 2^24, the relative rounding stays below 1e-7: no effect on a duration or a ratio"
)]
pub(crate) fn count_f32(n: usize) -> f32 {
    n as f32
}

/// A count as `f64`, exact up to 2^53.
#[expect(clippy::cast_precision_loss, reason = "exact up to 2^53 samples, far beyond any audio")]
pub(crate) fn count_f64(n: usize) -> f64 {
    n as f64
}

/// Duration (s) of `n` samples at 16 kHz.
pub(crate) fn seconds(n: usize) -> f32 {
    count_f32(n) / SAMPLE_RATE_F32
}

/// Samples contained in `secs` seconds at 16 kHz, truncated.
pub(crate) fn samples(secs: f32) -> usize {
    trunc_usize(f64::from(secs * SAMPLE_RATE_F32))
}

/// `x` truncated toward zero, saturated to `[0, usize::MAX]`; NaN gives 0.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "truncation intended; `as` saturates: negative or NaN gives 0, infinity usize::MAX"
)]
pub(crate) fn trunc_usize(x: f64) -> usize {
    x as usize
}

/// Like [`trunc_usize`], as `u64`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "truncation intended; `as` saturates: negative or NaN gives 0, infinity u64::MAX"
)]
pub(crate) fn trunc_u64(x: f64) -> u64 {
    x as u64
}

/// An `f64` brought down to the nearest `f32`.
///
/// Outside the `f32` range, it becomes the infinity of its sign.
#[expect(clippy::cast_possible_truncation, reason = "rounding intended at the precision of audio and timings")]
pub(crate) fn narrow_f32(x: f64) -> f32 {
    x as f32
}
