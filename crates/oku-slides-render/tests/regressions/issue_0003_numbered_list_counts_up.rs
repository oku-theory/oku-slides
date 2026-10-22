//! t-173 : chaque paragraphe numéroté devenait sa propre liste Typst
//! (`enum(body)`), si bien qu'une liste de trois éléments s'affichait
//! « 1. 1. 1. ». Les éléments voisins doivent former une seule liste.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use oku_slides_core::Deck;
use oku_slides_core::kinds::{base, essentiel};
use serde_json::json;
use speculoos::prelude::*;

/// Une slide avec `n` paragraphes numérotés, tous « a ».
fn numbered(n: usize) -> Deck {
    let text: Vec<_> = (0..n).map(|_| json!({ "list": "number", "runs": [{ "t": "a" }] })).collect();
    serde_json::from_value(json!({ "slides": [{ "id": "s", "kind": essentiel::BLANK, "params": { "elements": [{
        "id": "e", "kind": base::TEXT, "frame": { "x": 0, "y": 0, "w": 800, "h": 400 }, "params": { "body": text }
    }] } }] }))
    .unwrap_or_else(|e| panic!("{e}"))
}

/// Glyphes distincts dessinés sur la page : un chiffre de plus en ajoute un.
fn distinct_glyphs(deck: &Deck) -> usize {
    let mut r = crate::common::renderer();
    r.set_deck(deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"));
    let svg = r.compile().unwrap_or_else(|e| panic!("{e}")).svg(0).unwrap_or_else(|e| panic!("{e}"));
    svg.split("xlink:href=\"#g").skip(1).filter_map(|s| s.split('"').next()).collect::<BTreeSet<_>>().len()
}

#[test]
fn numbered_paragraphs_in_a_row_draw_1_2_3() {
    // « 1. a » seul, puis « 1. a 2. a 3. a » : deux chiffres de plus.
    assert_that!(distinct_glyphs(&numbered(3))).is_equal_to(distinct_glyphs(&numbered(1)) + 2);
}
