//! Design token colors: the accepted hex form, and the color names a deck uses.

use std::collections::BTreeSet;

use crate::model::{Deck, Paragraph};
use crate::params::{ParamValue, Params};

/// True for `#rgb`, `#rrggbb` or `#rrggbbaa` (ASCII hex digits).
#[must_use]
pub fn is_hex_color(s: &str) -> bool {
    s.strip_prefix('#').is_some_and(|h| matches!(h.len(), 3 | 6 | 8) && h.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// The theme color names `deck` uses, hex values left out.
///
/// Read in element styles (`color`, `fill`, `stroke`) and in the text runs
/// (`color`) of slide and element params, list items included: exactly the
/// values Typst resolves with `color-of(…, lenient: true)`, which draws a
/// name the theme lacks in `ink-muted`; the session warns about it. Keep
/// both lists in step. Values starting with `#` are left out.
#[must_use]
pub fn color_names(deck: &Deck) -> BTreeSet<&str> {
    let mut out = BTreeSet::new();
    for slide in &deck.slides {
        run_colors(&slide.params, &mut out);
        for l in slide.walk() {
            let style = &l.element.style;
            out.extend([&style.color, &style.fill, &style.stroke].into_iter().flatten().map(String::as_str));
            run_colors(&l.element.params, &mut out);
        }
    }
    out.retain(|c| !c.starts_with('#'));
    out
}

/// Adds the run colors of the rich values of `params`, list items included, to `out`.
fn run_colors<'a>(params: &'a Params, out: &mut BTreeSet<&'a str>) {
    for (_, value) in params.iter() {
        match value {
            ParamValue::Rich(paragraphs) => out.extend(paragraphs.iter().flat_map(paragraph_colors)),
            ParamValue::List(items) => items.iter().for_each(|i| run_colors(&i.values, out)),
            ParamValue::Text(_) | ParamValue::Slot(_) | ParamValue::Value(_) => {}
        }
    }
}

/// The run colors of `paragraph`.
fn paragraph_colors(paragraph: &Paragraph) -> impl Iterator<Item = &str> {
    paragraph.runs.iter().filter_map(|r| r.color.as_deref())
}

#[cfg(test)]
#[path = "color_tests.rs"]
mod color_tests;
