//! Value axis scale: bounds and tick count from `min`, `max` and `interval`.
//!
//! charts-rs rounds an axis to its own "nice" step unless both bounds are
//! set, in which case it splits the range evenly. A tick `interval` therefore
//! becomes two bounds (the missing ones rounded outward from the data to a
//! multiple of it, 0 included) and a whole number of steps: a top that is not
//! a whole step above the bottom rises to the next one, so every tick stays
//! `min + k × interval` and no value is cut off.
//!
//! Without `interval`, unless both bounds are written, the axis takes
//! PowerPoint's and LibreOffice's automatic scale instead of charts-rs's: the
//! data, 0 included, widened by 5 % of its span on each nonzero side, ticked
//! by the smallest 1, 2 or 5 × 10ⁿ step giving at most [`AUTO_SPLITS`]
//! intervals. It is computed at each render, so a data edit rescales the axis
//! (t-1493). charts-rs writes ticks with one decimal, so steps start at
//! [`MIN_AUTO_STEP`]: data too small for it, or a written bound off the
//! step's grid, keeps the engine's own scale.

use crate::error::Result;
use crate::model::inconsistent;
use crate::option::Axis;
use oku_slides_i18n::Message;

/// Most ticks an `interval` may give an axis; a finer one is dropped.
pub(crate) const MAX_SPLITS: usize = 50;

/// Most intervals of an automatic axis, as in PowerPoint and LibreOffice.
const AUTO_SPLITS: usize = 10;

/// Smallest automatic step whose ticks charts-rs's one-decimal labels can tell apart.
const MIN_AUTO_STEP: f64 = 0.1;

/// Share of the data span added beyond its nonzero ends on an automatic axis.
const AUTO_PADDING: f64 = 0.05;

/// Fraction of an interval ignored when counting steps, for float rounding.
const STEP_SLACK: f64 = 1e-9;

/// A value axis resolved for the engine.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ValueScale {
    /// Lower bound; computed by the engine if absent.
    pub min: Option<f64>,
    /// Upper bound; computed by the engine if absent.
    pub max: Option<f64>,
    /// Intervals between `min` and `max`, set with an `interval` or an automatic step.
    pub splits: Option<usize>,
}

/// The scale of `axis` over data spanning `low` to `high`.
///
/// With both bounds and no `interval`, the axis as written. Without
/// `interval`, the automatic scale of the module docs. With it, a missing bound
/// is the multiple of `interval` at or beyond the data and 0, and the axis
/// spans whole intervals from `min` (at least 1): its top rises to the next
/// whole interval. Beyond [`MAX_SPLITS`] intervals, or bounds the engine's
/// `f32` cannot hold, the interval is dropped and the bounds stay as written.
pub(crate) fn value_scale(axis: Option<&Axis>, low: f64, high: f64) -> ValueScale {
    let (min, max) = (axis.and_then(|a| a.min), axis.and_then(|a| a.max));
    if let Some(step) = axis.and_then(|a| a.interval) {
        return interval_scale(min, max, step, low, high);
    }
    if min.is_some() && max.is_some() {
        return ValueScale { min, max, splits: None };
    }
    let (low, high) = (low.min(0.0), high.max(0.0));
    let pad = (high - low) * AUTO_PADDING;
    let (low, high) = (if low < 0.0 { low - pad } else { low }, if high > 0.0 { high + pad } else { high });
    let engine = ValueScale { min, max, splits: None };
    let Some(step) = auto_step(min.unwrap_or(low), max.unwrap_or(high)) else { return engine };
    let lo = min.unwrap_or_else(|| grid_below(low, step));
    let hi = max.unwrap_or_else(|| grid_above(high, step));
    let steps = (hi - lo) / step;
    // A written bound off the step's grid: no whole count of steps reaches it.
    if (steps - steps.round()).abs() > STEP_SLACK || !(in_engine(lo) && in_engine(hi)) {
        return engine;
    }
    ValueScale { min: Some(lo), max: Some(hi), splits: Some(whole(steps.round())) }
}

/// The smallest 1, 2 or 5 × 10ⁿ step ticking `lo` to `hi` in at most [`AUTO_SPLITS`] intervals.
///
/// Steps below [`MIN_AUTO_STEP`] are skipped. `None` for an empty or non-finite
/// span, or when no step from [`MIN_AUTO_STEP`] up fits.
fn auto_step(lo: f64, hi: f64) -> Option<f64> {
    let span = hi - lo;
    if !(span.is_finite() && span > 0.0) {
        return None;
    }
    let exp = (span / as_f64(AUTO_SPLITS)).log10().floor();
    let base = if exp < 0.0 { 1.0 / 10_f64.powf(-exp) } else { 10_f64.powf(exp) };
    [1.0, 2.0, 5.0, 10.0, 20.0]
        .into_iter()
        .map(|m| m * base)
        .filter(|&step| step >= MIN_AUTO_STEP * (1.0 - STEP_SLACK))
        .find(|&step| grid_above(hi, step) - grid_below(lo, step) <= step * (as_f64(AUTO_SPLITS) + STEP_SLACK))
}

/// The multiple of `step` at or below `v`, ignoring float noise just under one.
fn grid_below(v: f64, step: f64) -> f64 {
    (v / step + STEP_SLACK).floor() * step
}

/// The multiple of `step` at or above `v`, ignoring float noise just over one.
fn grid_above(v: f64, step: f64) -> f64 {
    (v / step - STEP_SLACK).ceil() * step
}

/// The scale of a tick `step` from the written bounds, or the data rounded outward.
fn interval_scale(min: Option<f64>, max: Option<f64>, step: f64, low: f64, high: f64) -> ValueScale {
    let lo = min.unwrap_or_else(|| grid_below(low.min(0.0), step));
    let hi = max.unwrap_or_else(|| grid_above(high.max(0.0), step));
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

/// A whole count in `1..=MAX_SPLITS` as `usize`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a whole number in 1..=MAX_SPLITS: checked by interval_scale, bounded by auto_step's cap"
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
