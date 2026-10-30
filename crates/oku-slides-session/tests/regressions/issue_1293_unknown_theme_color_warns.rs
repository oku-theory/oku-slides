//! t-1293: a deck colored with Papier's `accent-soft` failed to render once
//! switched to Pro, which has no such color. It now renders, and the deck
//! warnings name the color drawn in `ink-muted` instead.

use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::{Isolated, fixture_path, req, session, set_theme, snapshot_json};

/// The demo deck colored `accent-soft` under Papier, then moved to `theme` if another.
fn colored_under(theme: &str) -> Isolated {
    let mut s = session();
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("{e:?}"));
    s.handle(set_theme("papier")).unwrap_or_else(|e| panic!("{e:?}"));
    let style = json!({"cmd": "apply", "label": "Couleur", "ops": [
        {"op": "set_style", "slide": "s-libre", "element": "e-texte", "style": {"color": "accent-soft"}}]});
    s.handle(req(style)).unwrap_or_else(|e| panic!("{e:?}"));
    if theme != "papier" {
        s.handle(set_theme(theme)).unwrap_or_else(|e| panic!("{e:?}"));
    }
    s
}

/// The deck warnings naming `accent-soft`.
fn accent_soft_warnings(s: &mut Isolated) -> Vec<String> {
    let warnings = snapshot_json(s)["warnings"].as_array().cloned().unwrap_or_default();
    warnings.iter().filter_map(Value::as_str).filter(|w| w.contains("accent-soft")).map(str::to_owned).collect()
}

#[test]
fn snapshot_under_theme_without_the_color_warns_once() {
    let mut s = colored_under("pro");
    assert_that!(accent_soft_warnings(&mut s)).has_length(1);
}

#[test]
fn snapshot_under_theme_with_the_color_does_not_warn() {
    let mut s = colored_under("papier");
    assert_that!(accent_soft_warnings(&mut s)).is_empty();
}
