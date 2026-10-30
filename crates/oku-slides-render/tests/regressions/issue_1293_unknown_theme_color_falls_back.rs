//! A deck imported under Papier names its colors after Papier's tokens
//! (`accent-soft`). Under a theme without that name (Pro), `color-of`
//! panicked and the whole deck failed to render. An unknown name in an
//! element style or a text run now falls back to `ink-muted` (the session
//! lists it in the deck warnings); one in the theme's own tokens still fails
//! the render, so a theme typo stays loud (t-1293).

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_core::kinds::{base, essentiel};
use oku_slides_render::{Compiled, RenderError};
use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use crate::common::{theme_pins, theme_tokens};

/// Render of a blank Pro slide holding `element`, over Pro's tokens changed by `tweak`.
fn pro_render(element: &Value, tweak: impl FnOnce(&mut Map<String, Value>)) -> Result<Compiled, RenderError> {
    let deck = json!({ "theme": "pro", "themes": theme_pins("pro"),
        "slides": [{ "id": "s", "kind": essentiel::BLANK, "params": { "elements": [element] } }] });
    let deck = Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("{e}"));
    let mut tokens = theme_tokens(Some("pro"));
    tweak(&mut tokens);
    let mut r = crate::common::renderer();
    r.set_theme(Some(tokens)).unwrap_or_else(|e| panic!("{e}"));
    r.render(&deck, &BTreeMap::new())
}

/// SVG of a blank Pro slide holding `element`.
fn pro_svg(element: &Value) -> String {
    let doc = pro_render(element, |_| {}).unwrap_or_else(|e| panic!("{e}"));
    doc.svg(0).unwrap_or_else(|e| panic!("{e}")).to_lowercase()
}

#[test]
fn pro_theme_lacks_accent_soft() {
    assert_that!(theme_tokens(Some("pro"))["color"].get("accent-soft")).is_none();
}

/// `element` with every `accent-soft` replaced by `ink-muted`.
fn explicit(element: &Value) -> Value {
    serde_json::from_str(&element.to_string().replace("\"accent-soft\"", "\"ink-muted\""))
        .unwrap_or_else(|e| panic!("{e}"))
}

#[rstest]
#[case::style_color(json!({ "id": "e", "kind": base::TEXT, "frame": { "x": 40, "y": 40, "w": 400, "h": 200 },
    "style": { "color": "accent-soft" }, "params": { "body": [{ "runs": [{ "t": "Corps" }] }] } }))]
#[case::run_color(json!({ "id": "e", "kind": base::TEXT, "frame": { "x": 40, "y": 40, "w": 400, "h": 200 },
    "params": { "body": [{ "runs": [{ "t": "Corps", "color": "accent-soft" }] }] } }))]
#[case::shape_stroke(json!({ "id": "e", "kind": base::SHAPE, "frame": { "x": 40, "y": 40, "w": 400, "h": 200 },
    "style": { "stroke": "accent-soft" }, "params": { "shape": "rect" } }))]
#[case::shape_fill(json!({ "id": "e", "kind": base::SHAPE, "frame": { "x": 40, "y": 40, "w": 400, "h": 200 },
    "style": { "fill": "accent-soft" }, "params": { "shape": "rect" } }))]
fn color_of_unknown_name_renders_as_ink_muted(#[case] element: Value) {
    assert_that!(pro_svg(&element)).is_equal_to(pro_svg(&explicit(&element)));
}

#[test]
fn color_of_unknown_name_in_theme_component_fails_render() {
    let card = json!({ "id": "e", "kind": base::CARD, "frame": { "x": 40, "y": 40, "w": 400, "h": 200 },
        "params": { "body": [{ "runs": [{ "t": "Corps" }] }] } });
    let result = pro_render(&card, |t| {
        t["component"]["base"]["card"]["border-color"] = json!("accent-soft");
    });
    assert_that!(result.map(|_| ())).is_err();
}
