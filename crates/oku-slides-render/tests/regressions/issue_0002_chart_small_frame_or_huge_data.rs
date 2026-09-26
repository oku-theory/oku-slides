//! Un graphique dans un petit cadre faisait paniquer charts-rs (`clamp`
//! min > max, reste par zéro) et tuait le rendu ; des centaines de milliers
//! de points donnaient un SVG de dizaines de Mo. Un graphique qui ne se
//! dessine pas doit laisser un cadre de repli, sans arrêter le rendu.

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_render::{Fonts, Renderer};
use serde_json::json;
use speculoos::prelude::*;

fn deck_with(frame: serde_json::Value, rows: usize) -> Deck {
    let mut source = vec![json!(["Mois", "Ventes"])];
    source.extend((0..rows).map(|i| json!([format!("m{i}"), i])));
    serde_json::from_value(json!({ "slides": [{ "id": "s", "kind": "blank", "elements": [{
        "id": "g", "kind": "chart", "frame": frame,
        "option": { "dataset": { "source": source }, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] }
    }] }] }))
    .unwrap_or_else(|e| panic!("{e}"))
}

fn compiles(deck: &Deck) -> bool {
    let mut r = Renderer::new(Fonts::default()).unwrap_or_else(|e| panic!("{e}"));
    r.set_deck(deck, &BTreeMap::new()).is_ok() && r.compile().is_ok()
}

#[test]
fn chart_in_tiny_frames_renders_without_panic() {
    for (w, h) in [(300.0, 40.0), (100.0, 60.0), (10.0, 10.0), (1e30, 1e30)] {
        let deck = deck_with(json!({ "x": 0, "y": 0, "w": w, "h": h }), 3);
        asserting(&format!("cadre {w}×{h}")).that(&compiles(&deck)).is_true();
    }
}

#[test]
fn chart_with_huge_dataset_falls_back_instead_of_rendering() {
    let deck = deck_with(json!({ "x": 0, "y": 0, "w": 800, "h": 450 }), 200_000);
    let mut r = Renderer::new(Fonts::default()).unwrap();
    r.set_deck(&deck, &BTreeMap::new()).unwrap();
    let page = r.compile().unwrap().svg(0).unwrap();
    // Le cadre de repli, pas un graphique de dizaines de Mo embarqué.
    assert_that!(page.len()).is_less_than(1_000_000);
}
