use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use oku_slides_core::Deck;

use super::unknown_color_warnings;

/// A deck of one element per color of `colors`, each its text color.
fn deck_with_colors(colors: &[&str]) -> Deck {
    let elements: Vec<Value> = colors
        .iter()
        .enumerate()
        .map(|(i, c)| json!({ "id": format!("e{i}"), "kind": "base/text", "style": { "color": c } }))
        .collect();
    let deck = json!({ "slides": [{ "id": "s", "kind": "essentiel/blank", "params": { "elements": elements } }] });
    Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("{e}"))
}

/// A deck whose one element has text color `color`.
fn deck_with_color(color: &str) -> Deck {
    deck_with_colors(&[color])
}

/// Tokens whose colors are `ink` (hex) and `soft` (a name pointing at `ink`).
fn tokens() -> Value {
    json!({ "color": { "ink": "#000000", "soft": "ink" } })
}

#[rstest]
#[case::known_name("ink", 0)]
#[case::name_through_a_reference("soft", 0)]
#[case::hex("#123456", 0)]
#[case::unknown_name("accent-soft", 1)]
fn unknown_color_warnings_counts_only_unresolved_names(#[case] color: &str, #[case] count: usize) {
    let warnings = unknown_color_warnings(&deck_with_color(color), Some(&tokens()), oku_slides_i18n::english());
    assert_that!(warnings).has_length(count);
}

#[rstest]
#[case::english("en", "accent-soft")]
#[case::french("fr", "« accent-soft »")]
fn unknown_color_warnings_names_the_color_in_the_user_language(#[case] lang: &str, #[case] expected: &str) {
    let l = oku_slides_i18n::Localizer::new(Some(lang));
    let warnings = unknown_color_warnings(&deck_with_color("accent-soft"), Some(&tokens()), &l);
    assert_that!(warnings.join("\n")).contains(expected);
}

#[test]
fn unknown_color_warnings_without_tokens_is_empty() {
    assert_that!(unknown_color_warnings(&deck_with_color("accent-soft"), None, oku_slides_i18n::english())).is_empty();
}

#[test]
fn unknown_color_warnings_one_per_distinct_name() {
    let deck = deck_with_colors(&["x", "x", "y", "ink"]);
    assert_that!(unknown_color_warnings(&deck, Some(&tokens()), oku_slides_i18n::english())).has_length(2);
}

#[test]
fn unknown_color_warnings_quotes_name_without_its_quotation_marks() {
    let warnings = unknown_color_warnings(&deck_with_color("a\"b"), Some(&tokens()), oku_slides_i18n::english());
    assert_that!(warnings.join("")).does_not_contain("a\"b");
}
