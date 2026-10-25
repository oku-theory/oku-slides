//! Bounded, named numeric conversions, shared by import and the session.
//!
//! Rust has no fallible conversion between floats and integers: these
//! functions hold the workspace's `as` casts, each with its justification, and
//! keep their semantics exactly. Towards an integer, `as` saturates at the
//! type's bounds (a negative gives 0 for an unsigned type) and maps NaN to 0; the
//! name says which rounding is applied first (`round` to nearest, half away from
//! zero; `floor` downwards; `trunc` towards zero, the one `as` does).

/// A count (pixels, characters, entries) as a float, exact up to 2^53.
#[must_use]
#[expect(clippy::cast_precision_loss, reason = "the workspace's counts stay far below 2^53")]
pub fn count_f64(n: usize) -> f64 {
    n as f64
}

/// A 64-bit size or count as a float, nearest beyond 2^53.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "beyond 2^53 the relative error stays below 1e-15, invisible on display"
)]
pub fn size_f64(n: u64) -> f64 {
    n as f64
}

/// A signed (OOXML) integer as a float, nearest beyond 2^53.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "beyond 2^53 the relative rounding of an f64 has no effect on points, degrees or fractions"
)]
pub fn int_f64(v: i64) -> f64 {
    v as f64
}

/// A `0..=255` channel rounded to nearest, saturated at the bounds; NaN gives 0.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to [0, 255] just before the cast, and `as` maps NaN to 0"
)]
pub fn byte(v: f64) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// `v` rounded to nearest, saturated to `[0, u32::MAX]`; NaN gives 0.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the saturation of `as` (and NaN to 0) is the intended conversion"
)]
pub fn round_u32(v: f64) -> u32 {
    v.round() as u32
}

/// `v` rounded to nearest, saturated to the bounds of `i64`; NaN gives 0.
#[must_use]
#[expect(clippy::cast_possible_truncation, reason = "the saturation of `as` (and NaN to 0) is the intended conversion")]
pub fn round_i64(v: f64) -> i64 {
    v.round() as i64
}

/// `v` rounded down, saturated to the bounds of `i32`; NaN gives 0.
#[must_use]
#[expect(clippy::cast_possible_truncation, reason = "the saturation of `as` (and NaN to 0) is the intended conversion")]
pub fn floor_i32(v: f64) -> i32 {
    v.floor() as i32
}

/// `v` truncated towards zero, saturated to `[0, u32::MAX]`; NaN gives 0.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the truncation and saturation of `as` (and NaN to 0) are the intended conversion"
)]
pub fn trunc_u32(v: f64) -> u32 {
    v as u32
}

/// `v` truncated towards zero as an index, saturated to `[0, usize::MAX]`; NaN gives 0.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the truncation and saturation of `as` (and NaN to 0) are the intended conversion"
)]
pub fn trunc_usize(v: f64) -> usize {
    v as usize
}

/// `v` in single precision, to nearest; outside `f32`, infinity of the same sign.
#[must_use]
#[expect(clippy::cast_possible_truncation, reason = "rounding to f32 precision is the intended conversion")]
pub fn narrow_f32(v: f64) -> f32 {
    v as f32
}

#[cfg(test)]
#[path = "num_tests.rs"]
mod num_tests;
