use rstest::rstest;
use speculoos::prelude::*;

use serde_json::{Value, json};

use super::{color_names, is_hex_color};
use crate::Deck;

#[rstest]
#[case("#abc", true)]
#[case("#A1B2C3", true)]
#[case("#a1b2c3d4", true)]
#[case("#zzz", false)]
#[case("#1234", false)]
#[case("#12345", false)]
#[case("#", false)]
#[case("abc", false)]
#[case(" #abc", false)]
#[case("#abc ", false)]
#[case("#ｆｆｆ", false)]
fn is_hex_color_accepts_only_short_long_and_alpha_hex(#[case] s: &str, #[case] ok: bool) {
    assert_that!(is_hex_color(s)).is_equal_to(ok);
}

/// A deck of one `essentiel/blank` slide of params `slide_params`.
fn deck_with(slide_params: &Value) -> Deck {
    let deck = json!({ "slides": [{ "id": "s", "kind": "essentiel/blank", "params": slide_params }] });
    Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("{e}"))
}

/// The names `color_names` finds in a slide of params `slide_params`.
fn names_in(slide_params: &Value) -> Vec<String> {
    color_names(&deck_with(slide_params)).into_iter().map(str::to_owned).collect()
}

#[rstest]
#[case::style_color(json!({ "elements": [{ "id": "e", "kind": "base/text", "style": { "color": "a" } }] }))]
#[case::style_fill(json!({ "elements": [{ "id": "e", "kind": "base/shape", "style": { "fill": "a" } }] }))]
#[case::style_stroke(json!({ "elements": [{ "id": "e", "kind": "base/shape", "style": { "stroke": "a" } }] }))]
#[case::element_run(json!({ "elements": [{ "id": "e", "kind": "base/text",
    "params": { "body": [{ "runs": [{ "t": "x", "color": "a" }] }] } }] }))]
#[case::slide_run(json!({ "title": [{ "runs": [{ "t": "x", "color": "a" }] }] }))]
#[case::nested_element(json!({ "elements": [{ "id": "p", "kind": "base/stack",
    "params": { "items": [{ "id": "e", "kind": "base/text", "style": { "color": "a" } }] } }] }))]
#[case::list_item_run(json!({ "steps": [{ "id": "i", "label": [{ "runs": [{ "t": "x", "color": "a" }] }] }] }))]
fn color_names_finds_name_in_every_color_place(#[case] slide_params: Value) {
    assert_that!(names_in(&slide_params)).is_equal_to(vec!["a".to_owned()]);
}

#[test]
fn color_names_hex_values_are_skipped() {
    let params = json!({ "elements": [{ "id": "e", "kind": "base/text", "style": { "color": "#ff0000" },
        "params": { "body": [{ "runs": [{ "t": "x", "color": "#00ff00" }] }] } }] });
    assert_that!(names_in(&params)).is_empty();
}
