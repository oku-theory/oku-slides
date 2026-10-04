//! L'op `set_grid` et la grille résolue par le registre.

mod common;

use common::registry;
use oku_slides_core::*;
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

fn fixture() -> Deck {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("fixture {path} illisible : {e}"));
    Deck::from_json(&json).unwrap_or_else(|e| panic!("fixture {path} : {e}"))
}

/// Le registre de base, plus un widget de slide `quatre` dont la grille par défaut est 4 × 3.
fn with_quatre() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    let mut probe: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} : {e}"));
    probe["pack"] = json!([{
        "name": "quatre", "label": null, "category": "slide", "params": [
            { "name": "grid", "label": null, "type": "grid", "default": { "columns": 4, "rows": 3, "gap": "sm" }, "bind": null },
            { "name": "elements", "label": null, "type": "slot", "mode": "free", "accepts": null, "default": null, "bind": null }
        ]
    }]);
    WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("{path} : {e}"))
}

/// Un deck d'une slide `quatre` dont l'élément `e` occupe `cell`.
fn quatre_deck(cell: Cell) -> Deck {
    let mut deck = fixture();
    let mut slide = Slide::new("s-4", "quatre");
    let mut e = Element::new("e", "text");
    e.cell = Some(cell);
    slide.params.set("elements", ParamValue::Slot(vec![e]));
    deck.slides.push(slide);
    deck
}

fn set_grid(slide: &str, columns: Option<u32>, rows: Option<u32>) -> Op {
    Op::SetGrid { slide: slide.into(), columns, rows }
}

fn cell_of(deck: &Deck, slide: &str) -> Option<Cell> {
    deck.slide(slide).and_then(|s| s.locate("e")).and_then(|l| l.element.cell)
}

#[test]
fn resolved_grid_without_param_reads_the_widget_default() {
    let deck = quatre_deck(Cell { col: 0, row: 0, cols: 1, rows: 1 });
    let g = deck.slide("s-4").unwrap().resolved_grid(&with_quatre());
    assert_that!((g.columns, g.rows)).is_equal_to((4, 3));
}

#[test]
fn set_grid_from_the_widget_default_rescales_cells() {
    let mut deck = quatre_deck(Cell { col: 2, row: 1, cols: 2, rows: 1 });
    apply_all(&mut deck, vec![set_grid("s-4", Some(8), Some(6))], &with_quatre()).unwrap();
    assert_that!(cell_of(&deck, "s-4")).is_equal_to(Some(Cell { col: 4, row: 2, cols: 4, rows: 2 }));
}

#[test]
fn set_grid_from_the_widget_default_keeps_its_gap() {
    let mut deck = quatre_deck(Cell { col: 0, row: 0, cols: 1, rows: 1 });
    apply_all(&mut deck, vec![set_grid("s-4", Some(8), None)], &with_quatre()).unwrap();
    let grid = deck.slide("s-4").unwrap().params.value("grid").cloned();
    assert_that!(grid).is_equal_to(Some(json!({ "columns": 8, "rows": 3, "gap": "sm" })));
}

#[test]
fn set_cell_beyond_the_widget_default_is_refused() {
    let mut deck = quatre_deck(Cell { col: 0, row: 0, cols: 1, rows: 1 });
    let op =
        Op::SetCell { slide: "s-4".into(), element: "e".into(), cell: Some(Cell { col: 4, row: 0, cols: 1, rows: 1 }) };
    assert_that!(apply_all(&mut deck, vec![op], &with_quatre())).is_err();
}

fn issue_keys(deck: &Deck) -> Vec<String> {
    validate(deck, &with_quatre(), None).iter().map(|i| format!("{:?}", i.message)).collect()
}

#[test]
fn validate_cell_inside_the_widget_default_reports_nothing() {
    let deck = quatre_deck(Cell { col: 3, row: 2, cols: 1, rows: 1 });
    assert_that!(issue_keys(&deck).iter().any(|k| k.contains("issue-cell-outside-grid"))).is_false();
}

#[test]
fn validate_cell_beyond_the_widget_default_is_reported() {
    // Colonne 4 : dans la grille 12 × 6 par défaut, hors de la 4 × 3 du widget.
    let deck = quatre_deck(Cell { col: 4, row: 0, cols: 1, rows: 1 });
    assert_that!(issue_keys(&deck).iter().any(|k| k.contains("issue-cell-outside-grid"))).is_true();
}

#[test]
fn set_grid_keeps_an_explicit_gap() {
    let mut deck = quatre_deck(Cell { col: 0, row: 0, cols: 1, rows: 1 });
    let reg = with_quatre();
    let grid = json!({ "columns": 4, "rows": 3, "gap": "lg" });
    let set = Op::SetParam { target: Target { slide: "s-4".into(), element: None }, param: "grid".into(), value: grid };
    apply_all(&mut deck, vec![set, set_grid("s-4", Some(6), None)], &reg).unwrap();
    let grid = deck.slide("s-4").unwrap().params.value("grid").cloned();
    assert_that!(grid).is_equal_to(Some(json!({ "columns": 6, "rows": 3, "gap": "lg" })));
}

#[rstest]
#[case::columns_zero(Some(0), None)]
#[case::columns_too_many(Some(25), None)]
#[case::rows_zero(None, Some(0))]
#[case::rows_too_many(None, Some(25))]
fn set_grid_out_of_range_is_refused(#[case] columns: Option<u32>, #[case] rows: Option<u32>) {
    let mut deck = quatre_deck(Cell { col: 0, row: 0, cols: 1, rows: 1 });
    let before = deck.clone();
    let err = apply_all(&mut deck, vec![set_grid("s-4", columns, rows)], &with_quatre()).unwrap_err();
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::InvalidValue);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_grid_leaves_a_framed_element_without_cell_untouched() {
    let mut deck = quatre_deck(Cell { col: 0, row: 0, cols: 1, rows: 1 });
    let frame = Frame { x: 10.0, y: 20.0, w: 300.0, h: 200.0 };
    let mut framed = Element::new("f", "text");
    framed.frame = Some(frame);
    let slide = deck.slides.last_mut().unwrap();
    let mut elements = match slide.params.get("elements") {
        Some(ParamValue::Slot(v)) => v.clone(),
        _ => panic!("slot elements absent"),
    };
    elements.push(framed);
    slide.params.set("elements", ParamValue::Slot(elements));
    apply_all(&mut deck, vec![set_grid("s-4", Some(8), Some(6))], &with_quatre()).unwrap();
    let found = deck.slide("s-4").and_then(|s| s.locate("f")).map(|l| (l.element.frame, l.element.cell));
    assert_that!(found).is_equal_to(Some((Some(frame), None)));
}

#[test]
fn set_grid_on_a_slide_with_a_filled_flow_slot_is_refused() {
    let mut deck = fixture();
    let before = deck.clone();
    let err = apply_all(&mut deck, vec![set_grid("s-constat", Some(6), None)], &registry()).unwrap_err();
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::InvalidValue);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_grid_on_an_unknown_slide_is_refused() {
    let mut deck = fixture();
    let err = apply_all(&mut deck, vec![set_grid("absente", Some(6), None)], &registry()).unwrap_err();
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::SlideNotFound);
}

#[test]
fn set_grid_to_the_same_grid_changes_nothing() {
    let mut deck = quatre_deck(Cell { col: 1, row: 1, cols: 1, rows: 1 });
    let before = deck.clone();
    apply_all(&mut deck, vec![set_grid("s-4", Some(4), Some(3))], &with_quatre()).unwrap();
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_grid_reads_from_json() {
    let op: Op = serde_json::from_value(json!({ "op": "set_grid", "slide": "s-4", "rows": 4 })).unwrap();
    assert_that!(op).is_equal_to(set_grid("s-4", None, Some(4)));
}
