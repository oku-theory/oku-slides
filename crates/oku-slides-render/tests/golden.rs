//! Rendu figé : les pages des decks de référence restent identiques au
//! pixel près pendant le passage aux widgets Typst (t-270).

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_render::{Fonts, Renderer};
use sha2::{Digest, Sha256};

fn deck(path: &str) -> Deck {
    let path = format!("{}/{path}", env!("CARGO_MANIFEST_DIR"));
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path} illisible : {e}"));
    Deck::from_json(&json).unwrap_or_else(|e| panic!("{path} invalide : {e}"))
}

#[test]
fn render_reference_decks_keeps_every_page_pixel_identical() {
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let mut lines = Vec::new();
    for (name, path) in [("roadmap", "../../fixtures/roadmap.json"), ("widgets", "tests/golden/widgets.json")] {
        let compiled = r.render(&deck(path), &BTreeMap::new()).unwrap();
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
    let mut r = Renderer::new(Fonts::default()).unwrap();
    for (name, path) in [("roadmap", "../../fixtures/roadmap.json"), ("widgets", "tests/golden/widgets.json")] {
        let compiled = r.render(&deck(path), &BTreeMap::new()).unwrap();
        for page in 0..compiled.page_count() {
            std::fs::write(format!("{dir}/{name}-p{}.png", page + 1), compiled.png(page, 0.5).unwrap()).unwrap();
        }
    }
}
