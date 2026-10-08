//! Widget `table` : rendu Typst natif, mise à l'échelle, garde des données, jetons.

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use oku_slides_core::Deck;
use oku_slides_render::{Compiled, Fonts, Renderer};
use serde_json::{Value, json};
use speculoos::prelude::*;

/// Hauteur de page, en pt : `page.height` de `default-tokens.json`.
const PAGE_H: f64 = 1080.0;

fn deck_of(slides: Value, tokens: Option<Value>) -> Deck {
    let mut d = json!({ "slides": slides });
    if let Some(t) = tokens {
        d["tokens"] = t;
    }
    Deck::from_json(&d.to_string()).unwrap_or_else(|e| panic!("{e}"))
}

/// Une slide de contenu dont l'unique bloc est le tableau `t1`.
fn in_flow(data: Value) -> Value {
    json!([{ "id": "s", "kind": "content", "params": { "title": "T",
        "blocks": [{ "id": "t1", "kind": "table", "params": { "data": data } }] } }])
}

fn rows(n: usize) -> Value {
    let mut source = vec![json!(["Pays", "Ventes"])];
    source.extend((0..n).map(|i| json!([format!("P{i}"), i])));
    json!({ "dataset": { "source": source } })
}

/// La slide de `in_flow`, avec un tableau sans `data`.
fn placeholder() -> Deck {
    deck_of(
        json!([{ "id": "s", "kind": "content", "params": { "title": "T",
        "blocks": [{ "id": "t1", "kind": "table" }] } }]),
        None,
    )
}

fn with(mut data: Value, key: &str, v: Value) -> Value {
    data[key] = v;
    data
}

fn compile(d: &Deck) -> Compiled {
    let mut r = Renderer::new(Fonts::default()).unwrap_or_else(|e| panic!("{e}"));
    r.render(d, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"))
}

/// Empreinte des pixels de la page 1, à 1 px par pt.
///
/// Pas `page_hash` : celui-ci hache aussi les balises d'introspection du
/// cadre, qui portent le contenu Typst (paramètres, fermetures). Deux pages
/// identiques à l'œil y diffèrent, et une page « changée » peut ne l'être
/// que par ses balises.
fn hash(d: &Deck) -> u64 {
    let png = compile(d).png(0, 1.0).unwrap_or_else(|e| panic!("{e}"));
    let mut h = DefaultHasher::new();
    png.hash(&mut h);
    h.finish()
}

fn flow_hash(data: Value) -> u64 {
    hash(&deck_of(in_flow(data), None))
}

/// Slide vide : un tableau au cadre 300×120 en (10, 10), puis un rectangle
/// opaque qui déborde le cadre de 4 pt de chaque côté (dessiné par-dessus).
fn covered(table: Option<Value>) -> Deck {
    let mut elements = Vec::new();
    if let Some(data) = table {
        elements.push(json!({ "id": "t1", "kind": "table", "params": { "data": data },
            "frame": { "x": 10, "y": 10, "w": 300, "h": 120 } }));
    }
    elements.push(json!({ "id": "cache", "kind": "shape", "params": { "shape": "rect" }, "style": { "fill": "ink" },
        "frame": { "x": 6, "y": 6, "w": 308, "h": 128 } }));
    deck_of(json!([{ "id": "s", "kind": "blank", "params": { "elements": elements } }]), None)
}

#[test]
fn page_height_is_the_default_tokens_one() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../typst/oku-slides/src/default-tokens.json");
    let tokens: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_that!(tokens["page"]["height"].as_f64()).is_equal_to(Some(PAGE_H));
}

#[test]
fn table_larger_than_its_frame_draws_nothing_outside_it() {
    assert_that!(hash(&covered(Some(rows(30))))).is_equal_to(hash(&covered(None)));
}

#[test]
fn table_with_a_long_cell_draws_nothing_outside_its_frame() {
    let data = json!({ "dataset": { "source": [["Note"], ["mot ".repeat(200)]] } });
    assert_that!(hash(&covered(Some(data)))).is_equal_to(hash(&covered(None)));
}

#[test]
fn table_with_unbreakable_columns_draws_nothing_outside_its_frame() {
    let row = vec![json!("ABCDEFGHIJKL"); 20];
    let data = json!({ "dataset": { "source": [row.clone(), row] } });
    assert_that!(hash(&covered(Some(data)))).is_equal_to(hash(&covered(None)));
}

#[test]
fn table_with_a_long_cjk_cell_draws_nothing_outside_its_frame() {
    let data =
        json!({ "dataset": { "source": [["说明", "Note"], ["这是一个没有空格的很长的中文句子。".repeat(20), "x"]] } });
    assert_that!(hash(&covered(Some(data)))).is_equal_to(hash(&covered(None)));
}

#[test]
fn tall_table_in_flow_keeps_its_box_on_the_page() {
    let c = compile(&deck_of(in_flow(rows(60)), None));
    let b = c.layout().boxes.iter().find(|b| b.id == "t1").cloned().unwrap_or_else(|| panic!("boîte t1 absente"));
    assert_that!(c.page_count()).is_equal_to(1);
    assert_that!(b.y + b.h).is_less_than_or_equal_to(PAGE_H);
}

/// Comme `covered`, mais le tableau est en flux dans une pile au cadre 300×120 :
/// il n'a pas de cadre à lui, seulement la hauteur que la pile lui laisse.
fn covered_in_stack(table: Option<Value>) -> Deck {
    let children: Vec<Value> =
        table.into_iter().map(|data| json!({ "id": "t1", "kind": "table", "params": { "data": data } })).collect();
    let elements = json!([
        { "id": "pile", "kind": "stack", "params": { "children": children }, "frame": { "x": 10, "y": 10, "w": 300, "h": 120 } },
        { "id": "cache", "kind": "shape", "params": { "shape": "rect" }, "style": { "fill": "ink" },
            "frame": { "x": 6, "y": 6, "w": 308, "h": 128 } },
    ]);
    deck_of(json!([{ "id": "s", "kind": "blank", "params": { "elements": elements } }]), None)
}

#[test]
fn tall_table_in_flow_draws_nothing_below_the_height_left_to_it() {
    assert_that!(hash(&covered_in_stack(Some(rows(30))))).is_equal_to(hash(&covered_in_stack(None)));
}

#[test]
fn table_in_flow_grows_with_its_rows() {
    let height = |n| {
        let c = compile(&deck_of(in_flow(rows(n)), None));
        c.layout().boxes.iter().find(|b| b.id == "t1").map(|b| b.h).unwrap_or_else(|| panic!("boîte t1 absente"))
    };
    assert_that!(height(6)).is_greater_than(height(2));
}

#[test]
fn table_without_data_differs_from_an_unknown_element() {
    let unknown = deck_of(
        json!([{ "id": "s", "kind": "content", "params": { "title": "T",
        "blocks": [{ "id": "t1", "kind": "inconnu" }] } }]),
        None,
    );
    assert_that!(hash(&placeholder())).is_not_equal_to(hash(&unknown));
}

#[test]
fn malformed_table_data_renders_the_placeholder() {
    for bad in [
        json!({ "dataset": { "source": [5] } }),
        json!({ "dataset": "x" }),
        json!({ "dataset": { "source": [["A"]] }, "header-row": "oui" }),
    ] {
        assert_that!(flow_hash(bad)).is_equal_to(hash(&placeholder()));
    }
}

#[test]
fn oversized_table_data_renders_the_placeholder() {
    assert_that!(flow_hash(rows(200))).is_equal_to(hash(&placeholder()));
}

#[test]
fn markup_in_cells_stays_literal() {
    let data = json!({ "dataset": { "source": [["#panic(\"x\")", "$x$"], ["*b* @ref <l>", "] \\"]] } });
    assert_that!(compile(&deck_of(in_flow(data), None)).page_count()).is_equal_to(1);
}

#[test]
fn header_column_changes_the_page() {
    assert_that!(flow_hash(with(rows(3), "header-column", json!(true)))).is_not_equal_to(flow_hash(rows(3)));
}

#[test]
fn header_row_off_changes_the_page() {
    assert_that!(flow_hash(with(rows(3), "header-row", json!(false)))).is_not_equal_to(flow_hash(rows(3)));
}

#[test]
fn striped_changes_the_page() {
    assert_that!(flow_hash(with(rows(4), "striped", json!(true)))).is_not_equal_to(flow_hash(rows(4)));
}

#[test]
fn auto_alignment_of_a_number_column_equals_end() {
    assert_that!(flow_hash(with(rows(3), "align", json!(["auto", "end"])))).is_equal_to(flow_hash(rows(3)));
}

#[test]
fn start_alignment_of_a_number_column_changes_the_page() {
    assert_that!(flow_hash(with(rows(3), "align", json!(["auto", "start"])))).is_not_equal_to(flow_hash(rows(3)));
}

#[test]
fn header_tokens_change_the_page() {
    let themed = deck_of(in_flow(rows(2)), Some(json!({ "component": { "table": { "header-bg": "#ff0000" } } })));
    assert_that!(hash(&themed)).is_not_equal_to(flow_hash(rows(2)));
}

#[test]
fn zero_padding_single_empty_cell_compiles() {
    let tokens = json!({ "component": { "table": { "padding-x": 0, "padding-y": 0 } } });
    let d = deck_of(in_flow(json!({ "dataset": { "source": [[null]] } })), Some(tokens));
    assert_that!(compile(&d).page_count()).is_equal_to(1);
    assert_that!(hash(&d)).is_not_equal_to(hash(&placeholder()));
}
