//! Une image du flux plus haute que sa rangée annonçait une boîte plus haute que l'image dessinée.

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use speculoos::prelude::*;

use crate::common::{IMAGE_KIND, TEXT_KIND, renderer};

/// Une image SVG de 100 × 2000 : bien plus haute que la place qui lui reste.
const TALL_SVG: &[u8] =
    br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="2000"><rect width="100" height="2000"/></svg>"#;

#[test]
fn issue_1036_flow_image_box_is_the_drawn_height() {
    let deck = Deck::from_json(
        &serde_json::json!({
            "version": oku_slides_core::DECK_VERSION, "params": {"title": "T"},
            "slides": [{"id": "s", "kind": "content", "params": {"title": "Titre", "blocks": [
                {"id": "t", "kind": TEXT_KIND, "params": {"body": [{"runs": [{"t": "Avant"}]}]}},
                {"id": "i", "kind": IMAGE_KIND, "params": {"asset": "sha256:1036", "fit": "contain"}},
            ]}}]
        })
        .to_string(),
    )
    .unwrap();
    let assets = BTreeMap::from([("sha256:1036".to_owned(), TALL_SVG.to_vec())]);
    let compiled = renderer().render(&deck, &assets).unwrap();
    let layout = compiled.layout();
    let zone = layout.zone_on(1).unwrap();
    let image = layout.boxes_on(1).iter().find(|b| b.id == "i").unwrap();
    assert_that!(image.h).is_greater_than(0.0);
    assert_that!(image.y + image.h).is_less_than_or_equal_to(zone.y + zone.h + 0.5);
    // L'image prend la rangée `1fr` : elle descend jusqu'au bas de la zone.
    assert_that!((zone.y + zone.h) - (image.y + image.h)).is_less_than(1.0);
}
