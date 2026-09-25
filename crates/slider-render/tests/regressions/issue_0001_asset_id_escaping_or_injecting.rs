//! Un identifiant d'asset venu d'un fichier (zip des sources) ne doit ni faire
//! paniquer le rendu (chemin qui sort de la racine virtuelle), ni injecter du
//! code dans la source Typst générée (guillemet dans l'identifiant).

use std::collections::BTreeMap;

use slider_core::Deck;
use slider_render::{Fonts, Renderer};

fn fixture() -> Deck {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("fixture {path} illisible : {e}"));
    Deck::from_json(&json).unwrap_or_else(|e| panic!("fixture {path} invalide : {e}"))
}

fn render_with_asset(id: &str) -> Result<usize, String> {
    let mut r = Renderer::new(Fonts::default()).map_err(|e| e.to_string())?;
    let assets = BTreeMap::from([(id.to_owned(), vec![0u8; 8])]);
    r.set_deck(&fixture(), &assets).map_err(|e| e.to_string())?;
    r.compile().map(|c| c.page_count()).map_err(|e| e.to_string())
}

#[test]
fn render_asset_id_escaping_root_is_ignored() {
    assert_eq!(render_with_asset("sha256:../../outside"), Ok(8));
}

#[test]
fn render_asset_id_with_backslash_is_ignored() {
    assert_eq!(render_with_asset("sha256:a\\b"), Ok(8));
}

#[test]
fn render_asset_id_with_quote_cannot_inject_typst() {
    assert_eq!(render_with_asset(r#"sha256:ab",panic("injected"),""#), Ok(8));
}
