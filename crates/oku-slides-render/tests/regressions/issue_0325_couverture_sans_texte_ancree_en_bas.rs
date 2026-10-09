//! Une couverture `plain` sans barre ni texte (surtitre, titre, sous-titre
//! vides) posait ses champs vides en haut de la page : seuls des marqueurs
//! hors flux, sans hauteur, ne se laissaient pas ancrer par `align(bottom)`.
//! Leurs boîtes doivent rester en bas, là où le texte tapé apparaîtra.

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_render::{Fonts, Renderer};
use serde_json::json;
use speculoos::prelude::*;

#[test]
fn plain_cover_without_text_anchors_its_empty_fields_at_the_bottom() {
    let deck = json!({
        "version": 4,
        "params": { "title": "", "author": "Matthieu" },
        "tokens": { "component": { "title-slide": { "bar": false } } },
        "slides": [{ "id": "s", "kind": "title" }]
    });
    let deck = Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("deck invalide : {e}"));
    let mut r = Renderer::new(Fonts::default()).unwrap_or_else(|e| panic!("{e}"));
    let doc = r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"));
    let tokens = doc.layout().tokens.as_ref().unwrap_or_else(|| panic!("jetons absents"));
    let page_h = tokens["page"]["height"].as_f64().unwrap_or_else(|| panic!("hauteur de page absente"));
    let fields: Vec<_> = doc.layout().fields.iter().filter(|f| f.owner == "s").collect();
    for name in ["kicker", "title", "subtitle"] {
        let f = fields.iter().find(|f| f.field == name).unwrap_or_else(|| panic!("pas de champ {name}"));
        asserting(&format!("{name} vide")).that(&f.empty).is_true();
        asserting(&format!("{name} à y = {} sur une page de {page_h}", f.y)).that(&f.y).is_greater_than(page_h / 2.0);
    }
}
