//! Un tableau qui tenait dans son cadre gardait sa largeur naturelle : deux
//! colonnes étroites collées à gauche, le reste du cadre vide. Le tableau
//! occupe toute la largeur qu'on lui donne.

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use oku_slides_core::Deck;
use oku_slides_render::{Fonts, Renderer};
use serde_json::json;
use speculoos::prelude::*;

/// Un tableau étroit au cadre 300×120 en (10, 10), sous un cache opaque qui
/// couvre tout le cadre sauf ses 40 pt de droite.
fn left_covered(with_table: bool) -> Deck {
    let mut elements = Vec::new();
    if with_table {
        elements.push(json!({ "id": "t", "kind": "table",
            "params": { "data": { "dataset": { "source": [["A", "B"], [1, 2]] } } },
            "frame": { "x": 10, "y": 10, "w": 300, "h": 120 } }));
    }
    elements.push(json!({ "id": "cache", "kind": "shape", "params": { "shape": "rect" }, "style": { "fill": "ink" },
        "frame": { "x": 6, "y": 6, "w": 264, "h": 128 } }));
    let deck = json!({ "slides": [{ "id": "s", "kind": "blank", "params": { "elements": elements } }] });
    Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("{e}"))
}

fn hash(d: &Deck) -> u64 {
    let mut r = Renderer::new(Fonts::default()).unwrap_or_else(|e| panic!("{e}"));
    let png = r.render(d, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}")).png(0, 1.0);
    let mut h = DefaultHasher::new();
    png.unwrap_or_else(|e| panic!("{e}")).hash(&mut h);
    h.finish()
}

#[test]
fn narrow_table_draws_up_to_the_right_of_its_frame() {
    assert_that!(hash(&left_covered(true))).is_not_equal_to(hash(&left_covered(false)));
}
