//! Rendu figé : les pages des decks de référence restent identiques au
//! pixel près pendant le passage aux widgets Typst (t-270).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use sha2::{Digest, Sha256};
use speculoos::prelude::*;

fn deck(path: &str) -> Deck {
    let path = format!("{}/{path}", env!("CARGO_MANIFEST_DIR"));
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path} illisible : {e}"));
    Deck::from_json(&json).unwrap_or_else(|e| panic!("{path} invalide : {e}"))
}

/// Les decks de référence : nom de la ligne figée et chemin du deck.
/// `widgets-pro` rend les widgets avec le thème Pro.
const REFERENCES: [(&str, &str); 3] = [
    ("roadmap", "../../fixtures/roadmap.json"),
    ("widgets", "tests/golden/widgets.json"),
    ("widgets-pro", "tests/golden/widgets.json"),
];

/// Le thème d'un deck de référence : le sien ; `roadmap.json` n'en porte
/// pas, l'app lui donne le thème par défaut (Papier) ; `widgets-pro`, Pro.
fn reference_theme(name: &str, deck: &Deck) -> Option<&'static str> {
    match name {
        "widgets-pro" => Some("pro"),
        _ => common::theme_name(deck.theme.as_ref()).or((name == "roadmap").then_some("papier")),
    }
}

#[test]
fn render_reference_decks_keeps_every_page_pixel_identical() {
    let mut r = common::renderer();
    let mut lines = Vec::new();
    for (name, path) in REFERENCES {
        let deck = deck(path);
        r.set_theme(Some(common::theme_tokens(reference_theme(name, &deck)))).unwrap();
        let compiled = r.render(&deck, &BTreeMap::new()).unwrap();
        for page in 0..compiled.page_count() {
            let png = compiled.png(page, 0.5).unwrap();
            lines.push(format!("{name} p{} {}", page + 1, hex::encode(Sha256::digest(&png))));
        }
    }
    insta::assert_snapshot!("reference_pages", lines.join("\n"));
}

/// Écrit les PNG des pages de référence dans `$GOLDEN_DUMP_DIR` (mesure d'un
/// écart d'antialiasing ; voir le plan 2, contraintes globales).
#[test]
#[ignore = "outil : lancé à la main avec GOLDEN_DUMP_DIR"]
fn dump_reference_pages() {
    let dir = std::env::var("GOLDEN_DUMP_DIR").unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    let mut r = common::renderer();
    for (name, path) in REFERENCES {
        let deck = deck(path);
        r.set_theme(Some(common::theme_tokens(reference_theme(name, &deck)))).unwrap();
        let compiled = r.render(&deck, &BTreeMap::new()).unwrap();
        for page in 0..compiled.page_count() {
            std::fs::write(format!("{dir}/{name}-p{}.png", page + 1), compiled.png(page, 0.5).unwrap()).unwrap();
        }
    }
}

/// Chaque boîte d'élément des decks de référence : deck, page, id, x, y, w, h au dixième de pt.
fn reference_boxes() -> Vec<String> {
    let mut r = common::renderer();
    let mut lines = Vec::new();
    for (name, path) in REFERENCES {
        let deck = deck(path);
        r.set_theme(Some(common::theme_tokens(reference_theme(name, &deck)))).unwrap_or_else(|e| panic!("{e}"));
        let compiled = r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"));
        for page in 1..=compiled.page_count() {
            for b in compiled.layout().boxes_on(page) {
                lines.push(format!("{name} p{page} {} {:.1} {:.1} {:.1} {:.1}", b.id, b.x, b.y, b.w, b.h));
            }
        }
    }
    lines
}

#[test]
fn reference_decks_element_boxes_are_unchanged() {
    insta::assert_snapshot!("reference_boxes", reference_boxes().join("\n"));
}

/// The `option` or `data` param of element `id` on slide 4 of a resolved deck.
fn resolved_param(deck: &Deck, id: &str, param: &str) -> serde_json::Value {
    let json = serde_json::to_value(deck).unwrap_or_else(|e| panic!("{e}"));
    json["slides"][3]["params"]["blocks"]
        .as_array()
        .and_then(|blocks| blocks.iter().find(|b| b["id"] == id))
        .map_or_else(|| panic!("{id} absent"), |b| b["params"][param].clone())
}

#[test]
fn golden_widgets_renders_shared_data_in_both_views() {
    let r = common::renderer();
    let deck = deck("tests/golden/widgets.json");
    let resolved = oku_slides_render::resolve_views(&deck, r.registry(), None);
    let table = resolved_param(&resolved, "tb1", "data");
    let chart = resolved_param(&resolved, "g1", "option");
    let rows = serde_json::json!([["Mois", "Ventes", "Marge"], ["Jan", 3.0, 0.2], ["Fév", 5.0, 0.25]]);
    assert_that!(table["dataset"]["source"]).is_equal_to(&rows);
    // The chart reads only the columns its one series uses: categories and the first values.
    let used = serde_json::json!([["Mois", "Ventes"], ["Jan", 3.0], ["Fév", 5.0]]);
    assert_that!(chart["dataset"]["source"]).is_equal_to(&used);
    assert_that!(resolved.data.is_empty()).is_true();
}
