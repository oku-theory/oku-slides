//! Un graphique sans cadre (dans le flux ou sur une cellule de la grille)
//! était dessiné en 800×450 puis mis à l'échelle de sa boîte par Typst : ses
//! textes grossissaient ou rapetissaient avec elle. Il doit être dessiné à
//! la taille de sa boîte, pour que ses textes gardent la taille du thème.

use std::collections::BTreeMap;

use base64::Engine;
use oku_slides_core::Deck;
use oku_slides_core::kinds::{base, essentiel};
use serde_json::{Value, json};
use speculoos::prelude::*;

fn chart(placement: Value) -> Value {
    let mut c = json!({ "id": "g", "kind": base::CHART, "params": { "option": {
        "dataset": { "source": [["Mois", "Ventes"], ["Janvier", 3], ["Février", 5]] },
        "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } });
    if let (Value::Object(c), Value::Object(p)) = (&mut c, placement) {
        c.extend(p);
    }
    c
}

fn deck(slide: &Value) -> Deck {
    serde_json::from_value(json!({ "slides": [slide] })).unwrap_or_else(|e| panic!("{e}"))
}

/// Taille (largeur, hauteur) du SVG du graphique embarqué dans la page.
fn drawn_size(page: &str) -> (f64, f64) {
    const PREFIX: &str = "data:image/svg+xml;base64,";
    let start = page.find(PREFIX).unwrap_or_else(|| panic!("graphique absent de la page")) + PREFIX.len();
    let end = start + page[start..].find('"').unwrap_or_else(|| panic!("data URI non fermée"));
    let bytes = base64::engine::general_purpose::STANDARD.decode(&page[start..end]).unwrap_or_else(|e| panic!("{e}"));
    let svg = String::from_utf8(bytes).unwrap_or_else(|e| panic!("{e}"));
    let attr = |name: &str| -> f64 {
        let key = format!(" {name}=\"");
        let s = svg.find(&key).unwrap_or_else(|| panic!("{name} absent")) + key.len();
        svg[s..s + svg[s..].find('"').unwrap_or(0)].parse().unwrap_or_else(|e| panic!("{name} : {e}"))
    };
    (attr("width"), attr("height"))
}

fn assert_drawn_at_box_size(deck: &Deck) {
    let mut r = crate::common::renderer();
    let doc = r.render(deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"));
    let b = doc.layout().boxes.iter().find(|b| b.id == "g").cloned().unwrap_or_else(|| panic!("boîte absente"));
    let (w, h) = drawn_size(&doc.svg(0).unwrap_or_else(|e| panic!("{e}")));
    asserting(&format!("largeur dessinée {w} pour une boîte de {}", b.w)).that(&w).is_close_to(b.w, 1.0);
    asserting(&format!("hauteur dessinée {h} pour une boîte de {}", b.h)).that(&h).is_close_to(b.h, 1.0);
}

#[test]
fn chart_on_a_wide_grid_cell_is_drawn_at_the_cell_size() {
    let cell = chart(json!({ "cell": { "col": 0, "row": 4, "cols": 9, "rows": 2 } }));
    assert_drawn_at_box_size(&deck(&json!({ "id": "s", "kind": essentiel::BLANK, "params": { "elements": [cell] } })));
}

#[test]
fn chart_in_the_flow_is_drawn_at_its_box_size() {
    let blocks = [json!({ "id": "t", "kind": base::TEXT, "params": { "body": "Un point" } }), chart(json!({}))];
    assert_drawn_at_box_size(&deck(&json!({ "id": "s", "kind": essentiel::CONTENT, "params": { "blocks": blocks } })));
}

#[test]
fn chart_in_a_frame_is_drawn_at_the_frame_size() {
    let framed = chart(json!({ "frame": { "x": 0, "y": 0, "w": 500, "h": 200 } }));
    assert_drawn_at_box_size(&deck(
        &json!({ "id": "s", "kind": essentiel::BLANK, "params": { "elements": [framed] } }),
    ));
}
