//! t-693 : `Ids::fresh` repartait de `-2` à chaque doublon, soit un coût
//! quadratique sur une slide aux ids très répétés. Il reprend désormais au
//! dernier suffixe essayé. Les noms rendus ne changent pas.
//!
//! Écart à G16 : cette régression est une garde de résultat. La linéarité est
//! prouvée par le compteur d'essais du test unitaire
//! `ids_fresh_with_two_thousand_suffixes_taken_is_linear`, qu'une régression
//! externe ne peut pas lire.

use oku_slides_core::{Deck, ParamValue, canonicalize};
use serde_json::json;
use speculoos::prelude::*;

#[test]
fn canonicalize_of_five_thousand_copies_names_them_in_order() {
    let blocks: Vec<_> = (0..5_000)
        .map(|n| json!({ "id": "x", "kind": "base/text", "params": { "body": [{ "runs": [{ "t": format!("texte {n}") }] }] } }))
        .collect();
    let mut deck = Deck::from_json(
        &json!({ "slides": [{ "id": "s", "kind": "essentiel/content", "params": { "blocks": blocks } }] }).to_string(),
    )
    .unwrap_or_else(|e| panic!("deck : {e}"));
    canonicalize(&mut deck);
    let ids: Vec<String> = match deck.slides[0].params.get("blocks") {
        Some(ParamValue::Slot(elements)) => elements.iter().map(|e| e.id.clone()).collect(),
        other => panic!("slot attendu : {other:?}"),
    };
    let expected: Vec<String> = std::iter::once("x".to_owned()).chain((2..=5_000).map(|n| format!("x-{n}"))).collect();
    assert_that!(ids).is_equal_to(expected);
}
