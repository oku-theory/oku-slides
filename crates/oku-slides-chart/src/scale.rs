//! Value axis scale: bounds and tick count from `min`, `max` and `interval`.
//!
//! charts-rs rounds an axis to its own "nice" step unless both bounds are
//! set, in which case it splits the range evenly. A tick `interval` therefore
//! becomes two bounds (the missing ones rounded outward from the data to a
//! multiple of it, 0 included) and a whole number of steps: a top that is not
//! a whole step above the bottom rises to the next one, so every tick stays
//! `min + k × interval` and no value is cut off.

use crate::error::Result;
use crate::model::inconsistent;
use crate::option::Axis;
use oku_slides_i18n::Message;

/// Most ticks an `interval` may give an axis; a finer one is dropped.
pub(crate) const MAX_SPLITS: usize = 50;

/// A value axis resolved for the engine.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ValueScale {
    /// Lower bound; computed by the engine if absent.
    pub min: Option<f64>,
    /// Upper bound; computed by the engine if absent.
    pub max: Option<f64>,
    /// Intervals between `min` and `max`, set only with an `interval`.
    pub splits: Option<usize>,
}

/// The scale of `axis` over data spanning `low` to `high`.
///
/// Without `interval`, the axis bounds as written. With it, a missing bound
/// is the multiple of `interval` at or beyond the data and 0, and the axis
/// spans whole intervals from `min` (at least 1): its top rises to the next
/// whole interval. Beyond [`MAX_SPLITS`] intervals, or bounds the engine's
/// `f32` cannot hold, the interval is dropped and the bounds stay as written.
pub(crate) fn value_scale(axis: Option<&Axis>, low: f64, high: f64) -> ValueScale {
    let (min, max) = (axis.and_then(|a| a.min), axis.and_then(|a| a.max));
    let Some(step) = axis.and_then(|a| a.interval) else { return ValueScale { min, max, splits: None } };
    let lo = min.unwrap_or_else(|| (low.min(0.0) / step).floor() * step);
    let hi = max.unwrap_or_else(|| (high.max(0.0) / step).ceil() * step);
    // A float quotient such as 20 / 5 = 4.000…1 must not count a fifth step.
    let steps = ((hi - lo) / step - STEP_SLACK).ceil().max(1.0);
    let top = lo + step * steps;
    // An interval the engine (f32) cannot draw, or too fine to tick: the bounds as written.
    if !(in_engine(lo) && in_engine(top)) || steps > as_f64(MAX_SPLITS) {
        return ValueScale { min, max, splits: None };
    }
    ValueScale { min: Some(lo), max: Some(top), splits: Some(whole(steps)) }
}

/// Whether `v` is finite in the engine's `f32`.
fn in_engine(v: f64) -> bool {
    v.is_finite() && v.abs() <= f64::from(f32::MAX)
}

/// A small count as `f64`.
#[expect(clippy::cast_precision_loss, reason = "MAX_SPLITS is far below 2^52")]
fn as_f64(n: usize) -> f64 {
    n as f64
}

/// Fraction of an interval ignored when counting steps, for float rounding.
const STEP_SLACK: f64 = 1e-9;

/// A whole count in `1..=MAX_SPLITS` as `usize`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the caller checks 1 <= n <= MAX_SPLITS, a whole number"
)]
fn whole(n: f64) -> usize {
    n as usize
}

/// Refuses an `interval` that is not a finite number above 0, or a `max` not above `min`.
pub(crate) fn check_axis(axis: Option<&Axis>) -> Result<()> {
    if let Some(i) = axis.and_then(|a| a.interval)
        && !(i.is_finite() && i > 0.0)
    {
        return Err(inconsistent(Message::new("chart-axis-interval")));
    }
    match axis.map(|a| (a.min, a.max)) {
        Some((Some(lo), Some(hi))) if lo >= hi => Err(inconsistent(Message::new("chart-axis-bounds"))),
        _ => Ok(()),
    }
}

/// Lowest and highest value drawn: stacked series add up per category, by sign.
///
/// `series` gives each series' stack group and values. Without data, `(0, 0)`.
pub(crate) fn stacked_extent<'a>(series: impl Iterator<Item = (Option<&'a str>, &'a [Option<f64>])>) -> (f64, f64) {
    let mut sums: Vec<(&str, Vec<(f64, f64)>)> = Vec::new();
    let (mut low, mut high) = (0.0_f64, 0.0_f64);
    for (stack, values) in series {
        let Some(key) = stack else {
            for v in values.iter().flatten() {
                (low, high) = (low.min(*v), high.max(*v));
            }
            continue;
        };
        let at = if let Some(at) = sums.iter().position(|(k, _)| *k == key) {
            at
        } else {
            sums.push((key, Vec::new()));
            sums.len() - 1
        };
        let acc = &mut sums[at].1;
        if acc.len() < values.len() {
            acc.resize(values.len(), (0.0, 0.0));
        }
        for (slot, v) in acc.iter_mut().zip(values) {
            match v {
                Some(v) if *v >= 0.0 => slot.0 += v,
                Some(v) => slot.1 += v,
                None => {}
            }
            (low, high) = (low.min(slot.1), high.max(slot.0));
        }
    }
    (low, high)
}

#[cfg(test)]
#[path = "scale_tests.rs"]
mod scale_tests;
