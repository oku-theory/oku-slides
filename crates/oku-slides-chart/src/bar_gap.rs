//! Bar thickness from `barCategoryGap`, which charts-rs 1.1 cannot set.
//!
//! charts-rs splits each category band into a 5 pt margin on both sides and
//! one slot per bar series (a stack shares one), 3 pt apart. Its bar
//! `<rect>`s, the only ones with a `data-series` attribute, are redrawn
//! afterwards: each keeps its slot's place around the band centre, at the
//! thickness that leaves the requested gap free, bars touching. Value labels
//! drawn after the bars follow their bar's centre.

use snafu::ensure;

use crate::error::{BarLayoutSnafu, Result};
use crate::model::{Cartesian, CartesianMark};

/// Band margin charts-rs leaves on each side of a category.
const MARGIN: f32 = 5.0;
/// Space charts-rs leaves between two slots.
const SLOT_GAP: f32 = 3.0;
/// Below this thickness charts-rs has squeezed its spacing: the band is not measurable.
const MIN_THICKNESS: f32 = 1.5;
/// Thinnest bar drawn, in points.
const MIN_BAR: f32 = 1.0;
/// Distance from a bar centre within which a label is that bar's.
///
/// charts-rs rounds a label's centre and the bar's x and width apart (one
/// decimal each), so they may differ by 0.1; bars stay more than
/// `MIN_THICKNESS + SLOT_GAP` apart, so the nearest centre is never ambiguous.
const TOLERANCE: f32 = 0.25;

/// `svg` with the bars of `c` resized to its `bar_gap`; unchanged without one.
///
/// # Errors
///
/// Fails if charts-rs drew another number of bars than `c` has values.
pub(crate) fn narrow(svg: String, c: &Cartesian) -> Result<String> {
    let Some(gap) = &c.bar_gap else { return Ok(svg) };
    let (slots, count) = slots(c);
    let rects = bar_rects(&svg);
    ensure!(rects.len() == slots.len(), BarLayoutSnafu { expected: slots.len(), found: rects.len() });
    let axis = if c.horizontal { ("y", "height") } else { ("x", "width") };
    let Some(thickness) = rects.first().and_then(|r| attr(&svg[r.clone()], axis.1)) else { return Ok(svg) };
    if thickness <= MIN_THICKNESS {
        return Ok(svg);
    }
    let n = as_f32(count);
    let unit = n * thickness + 2.0 * MARGIN + SLOT_GAP * (n - 1.0);
    let free = gap.resolve(f64::from(unit)).map_or(0.0, as_f32_lossy);
    let bar = ((unit - free) / n).max(MIN_BAR);
    let mut moves = Vec::with_capacity(rects.len());
    let mut out = String::with_capacity(svg.len());
    let mut done = 0;
    for (range, slot) in rects.iter().zip(slots) {
        let tag = &svg[range.clone()];
        let pos = attr(tag, axis.0).unwrap_or_default();
        let offset = as_f32(slot) - (n - 1.0) / 2.0;
        let centre = pos + thickness / 2.0 - offset * (thickness + SLOT_GAP);
        let new_centre = centre + offset * bar;
        moves.push((pos + thickness / 2.0, new_centre));
        out.push_str(&svg[done..range.start]);
        out.push_str(&with_attr(&with_attr(tag, axis.0, new_centre - bar / 2.0), axis.1, bar));
        done = range.end;
    }
    out.push_str(&svg[done..]);
    Ok(move_labels(&out, rects.first().map_or(0, |r| r.start), axis.0, &moves))
}

/// Slot of each bar charts-rs draws, in drawing order, and the slot count.
///
/// Series by series, a value per category (one at least), skipping empty
/// ones; series of one stack group share a slot. Columns mirror charts-rs's
/// `render_bar`: a plain series or a new group takes the next slot, a known
/// group the slot at its rank among groups. Horizontal bars mirror
/// `HorizontalBarChart`: groups first, by rank, then the plain series.
fn slots(c: &Cartesian) -> (Vec<usize>, usize) {
    let bars = || c.series.iter().filter(|s| s.mark == CartesianMark::Bar);
    let mut groups: Vec<&str> = Vec::new();
    let mut count = 0;
    let mut ranks = Vec::new();
    for s in bars() {
        let known = s.stack.as_deref().map(|key| (key, groups.iter().position(|g| *g == key)));
        let rank = if let Some((_, Some(at))) = known {
            Rank::Group(at)
        } else {
            count += 1;
            if let Some((key, _)) = known {
                groups.push(key);
                Rank::NewGroup(groups.len() - 1, count - 1)
            } else {
                Rank::Plain(count - 1)
            }
        };
        ranks.push((rank, s));
    }
    let plain_before = |n: usize| ranks.iter().take(n).filter(|(r, _)| matches!(r, Rank::Plain(_))).count();
    let mut out = Vec::new();
    for (i, (rank, s)) in ranks.iter().enumerate() {
        let slot = match (*rank, c.horizontal) {
            (Rank::Group(g), _) | (Rank::NewGroup(g, _), true) => g,
            (Rank::Plain(_), true) => groups.len() + plain_before(i),
            (Rank::NewGroup(_, slot) | Rank::Plain(slot), false) => slot,
        };
        let drawn = s.values.iter().take(c.categories.len().max(1)).filter(|v| v.is_some()).count();
        out.extend(std::iter::repeat_n(slot, drawn));
    }
    (out, count.max(1))
}

/// How a bar series takes its slot: its stack group's rank, or its order.
#[derive(Clone, Copy)]
enum Rank {
    /// A group seen before, at this rank among groups.
    Group(usize),
    /// A new group: its rank among groups, and the slot charts-rs's columns give it.
    NewGroup(usize, usize),
    /// A plain series: the slot charts-rs's columns give it.
    Plain(usize),
}

/// Byte ranges of the `<rect …>` tags carrying `data-series`.
fn bar_rects(svg: &str) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(start) = svg[from..].find("<rect").map(|i| i + from) {
        let end = svg[start..].find('>').map_or(svg.len(), |i| start + i + 1);
        if svg[start..end].contains(" data-series=\"") {
            out.push(start..end);
        }
        from = end;
    }
    out
}

/// Value of the numeric attribute `name` of `tag`.
fn attr(tag: &str, name: &str) -> Option<f32> {
    let key = format!(" {name}=\"");
    let rest = &tag[tag.find(&key)? + key.len()..];
    rest[..rest.find('"')?].parse().ok()
}

/// `tag` with attribute `name` set to `value`, written as charts-rs does (one decimal).
fn with_attr(tag: &str, name: &str, value: f32) -> String {
    let key = format!(" {name}=\"");
    let Some(at) = tag.find(&key).map(|i| i + key.len()) else { return tag.to_owned() };
    let end = tag[at..].find('"').map_or(tag.len(), |i| at + i);
    format!("{}{}{}", &tag[..at], number(value), &tag[end..])
}

/// A coordinate with one decimal, `.0` dropped.
fn number(v: f32) -> String {
    let s = format!("{v:.1}");
    s.strip_suffix(".0").map_or_else(|| s.clone(), str::to_owned)
}

/// Moves the `<text>`s after `from` whose `name` is a bar centre of `centres` to its new centre.
fn move_labels(svg: &str, from: usize, name: &str, centres: &[(f32, f32)]) -> String {
    let mut out = String::with_capacity(svg.len());
    out.push_str(&svg[..from]);
    let mut done = from;
    while let Some(start) = svg[done..].find("<text").map(|i| i + done) {
        let end = svg[start..].find('>').map_or(svg.len(), |i| start + i + 1);
        let tag = &svg[start..end];
        out.push_str(&svg[done..start]);
        let nearest = |at: f32| {
            centres
                .iter()
                .map(|(before, after)| ((before - at).abs(), after))
                .filter(|(d, _)| *d <= TOLERANCE)
                .min_by(|a, b| a.0.total_cmp(&b.0))
        };
        let moved = attr(tag, name).and_then(nearest).map(|(_, after)| with_attr(tag, name, *after));
        out.push_str(moved.as_deref().unwrap_or(tag));
        done = end;
    }
    out.push_str(&svg[done..]);
    out
}

/// A small count as `f32`, exact below 2²⁴.
#[expect(clippy::cast_precision_loss, reason = "slot and series counts stay far below 2^24")]
fn as_f32(n: usize) -> f32 {
    n as f32
}

/// A length in points as `f32`, the engine's precision.
#[expect(clippy::cast_possible_truncation, reason = "charts-rs draws in f32: rounding is intended")]
fn as_f32_lossy(x: f64) -> f32 {
    x as f32
}

#[cfg(test)]
#[path = "bar_gap_tests.rs"]
mod bar_gap_tests;
