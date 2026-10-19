//! Un tableau haut dans les blocs d'une slide de contenu prenait toute la
//! hauteur restante (il est réduit pour tenir dans ce qu'on lui laisse) : un
//! graphique après lui n'avait plus de place, un texte débordait de la page.
//! Le tableau partage la hauteur avec les blocs qui grandissent.

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_core::kinds::base;
use oku_slides_render::{ElementBox, Layout};
use serde_json::{Value, json};
use speculoos::prelude::*;

fn table(n: usize) -> Value {
    let mut source = vec![json!(["Pays", "Ventes"])];
    source.extend((0..n).map(|i| json!([format!("P{i}"), i])));
    json!({ "id": "t", "kind": base::TABLE, "params": { "data": { "dataset": { "source": source } } } })
}

fn chart() -> Value {
    json!({ "id": "g", "kind": base::CHART, "params": { "option": {
        "dataset": { "source": [["Mois", "Ventes"], ["Janvier", 3], ["Février", 5]] },
        "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } })
}

fn text() -> Value {
    json!({ "id": "x", "kind": base::TEXT, "params": { "body": "Un point à retenir" } })
}

fn layout(blocks: Value) -> Layout {
    let deck = json!({ "slides": [{ "id": "s", "kind": "content", "params": { "title": "T", "blocks": blocks } }] });
    let deck = Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("deck invalide : {e}"));
    let mut r = crate::common::renderer();
    r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}")).layout().clone()
}

fn boxed<'a>(l: &'a Layout, id: &str) -> &'a ElementBox {
    l.boxes.iter().find(|b| b.id == id).unwrap_or_else(|| panic!("boîte {id} absente"))
}

/// Bas de la zone de corps, en pt : `page.height - page.margin-bottom`.
fn body_bottom(l: &Layout) -> f64 {
    let page = &l.tokens.as_ref().unwrap_or_else(|| panic!("jetons absents"))["page"];
    let v = |k: &str| page[k].as_f64().unwrap_or_else(|| panic!("page.{k} absent"));
    v("height") - v("margin-bottom")
}

#[test]
fn tall_table_leaves_room_for_a_chart_and_a_text_after_it() {
    let l = layout(json!([table(40), chart(), text()]));
    let (t, g, x) = (boxed(&l, "t"), boxed(&l, "g"), boxed(&l, "x"));
    let bottom = body_bottom(&l);
    asserting(&format!("texte de {} à {} sous le bas du corps {bottom}", x.y, x.y + x.h))
        .that(&(x.y + x.h))
        .is_less_than_or_equal_to(bottom + 0.5);
    asserting(&format!("graphique de {} pt, tableau de {} pt", g.h, t.h)).that(&g.h).is_greater_than(t.h * 0.5);
}

#[test]
fn tall_table_leaves_room_for_a_text_after_it() {
    let l = layout(json!([table(40), text()]));
    let x = boxed(&l, "x");
    let bottom = body_bottom(&l);
    asserting(&format!("texte de {} à {} sous le bas du corps {bottom}", x.y, x.y + x.h))
        .that(&(x.y + x.h))
        .is_less_than_or_equal_to(bottom + 0.5);
}

#[test]
fn short_table_keeps_its_natural_height_before_a_text() {
    // Un tableau qui tient garde sa hauteur : le texte suit juste dessous.
    let l = layout(json!([table(2), text()]));
    let (t, x) = (boxed(&l, "t"), boxed(&l, "x"));
    assert_that!(x.y - (t.y + t.h)).is_less_than(50.0);
}
