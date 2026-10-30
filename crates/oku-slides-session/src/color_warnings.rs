//! Deck warnings for the color names the rendered theme lacks.
//!
//! For deck data (element styles, text runs) Typst's `color-of` draws such
//! a name in `ink-muted` rather than failing the render (t-1293); each one
//! becomes a warning of the deck, beside Typst's own. A name in the theme's
//! own tokens still fails the render, so it needs no warning here.

use oku_slides_core::{Deck, color_names};
use oku_slides_i18n::Localizer;
use oku_slides_render::resolve_color;
use serde_json::Value;

use crate::agent::bounds::label;

/// One localized warning per color name of `deck` that `tokens` does not resolve.
///
/// `tokens` are the effective tokens of the last render (`Layout::tokens`);
/// without them there is nothing to compare, so no warning. Until a change
/// is rendered, the deck is compared with the previous render's tokens.
/// `resolve_color` follows eight references at most, where Typst has no
/// limit: a deeper chain would be reported though drawn as is. The name is
/// deck text, shown through `bounds::label`.
pub(crate) fn unknown_color_warnings(deck: &Deck, tokens: Option<&Value>, l: &Localizer) -> Vec<String> {
    let Some(colors) = tokens.and_then(|t| t.get("color")).and_then(Value::as_object) else {
        return Vec::new();
    };
    let level = l.tr("diagnostic-warning");
    color_names(deck)
        .into_iter()
        .filter(|name| resolve_color(colors, &Value::from(*name)).is_none())
        .map(|name| {
            let message = l.tr_with("diagnostic-unknown-color", &[("name", label(name).into())]);
            l.tr_with("diagnostic-unplaced", &[("level", level.as_str().into()), ("message", message.into())])
        })
        .collect()
}

#[cfg(test)]
#[path = "color_warnings_tests.rs"]
mod color_warnings_tests;
