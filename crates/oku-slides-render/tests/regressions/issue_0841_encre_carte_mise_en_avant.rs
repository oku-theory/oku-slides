//! La carte mise en avant posait sa couleur d'encre par `if ink != none { set
//! text(fill: ink) }` : en Typst, un `set` dans le bloc d'un `if` ne vaut que
//! dans ce bloc, la règle ne touchait donc aucun texte (seul un second
//! `text(fill: ink, body)` colorait le corps, sans défaut visible). Elle est
//! devenue `set … if` et l'enveloppe a disparu : ce test garde ce remaniement,
//! le corps prend `card.highlight-ink` seulement si la carte est mise en avant.

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_core::kinds::{base, essentiel};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use crate::common::{theme_pins, theme_tokens};

/// Encre de la carte mise en avant du thème Pro, lue dans ses jetons.
fn ink() -> String {
    let tokens = theme_tokens(Some("pro"));
    let name = tokens["component"]["base"]["card"]["highlight-ink"].as_str().unwrap_or_else(|| panic!("highlight-ink"));
    let hex = tokens["color"][name].as_str().unwrap_or_else(|| panic!("couleur {name}"));
    hex.to_lowercase()
}

/// SVG d'une slide nue du thème Pro portant une carte au corps « Corps ».
fn card_svg(highlight: bool) -> String {
    let card = json!({ "id": "c", "kind": base::CARD, "frame": { "x": 40, "y": 40, "w": 400, "h": 200 },
        "params": { "highlight": highlight, "body": [{ "runs": [{ "t": "Corps" }] }] } });
    let deck = json!({ "theme": "pro", "themes": theme_pins("pro"),
        "slides": [{ "id": "s", "kind": essentiel::BLANK, "params": { "elements": [card] } }] });
    let deck = Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("{e}"));
    let mut r = crate::common::renderer();
    r.set_theme(Some(theme_tokens(Some("pro")))).unwrap_or_else(|e| panic!("{e}"));
    let doc = r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"));
    doc.svg(0).unwrap_or_else(|e| panic!("{e}")).to_lowercase()
}

#[rstest]
#[case(true, true)]
#[case(false, false)]
fn card_body_uses_highlight_ink_only_when_highlighted(#[case] highlight: bool, #[case] inked: bool) {
    assert_that!(card_svg(highlight).contains(&ink())).is_equal_to(inked);
}
