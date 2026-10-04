mod common;

use common::registry;
use oku_slides_core::*;
use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

fn fixture() -> Deck {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("fixture {path} illisible : {e}"));
    Deck::from_json(&json).unwrap_or_else(|e| panic!("fixture {path} invalide : {e}"))
}

fn deck(value: Value) -> Deck {
    serde_json::from_value(value).unwrap_or_else(|e| panic!("deck invalide : {e}"))
}

fn element(value: Value) -> Element {
    serde_json::from_value(value).unwrap_or_else(|e| panic!("élément invalide : {e}"))
}

fn para(t: &str) -> Vec<Paragraph> {
    vec![Paragraph::plain(t)]
}

fn slide(id: &str) -> Target {
    Target { slide: id.into(), element: None }
}

fn at(slide: &str, element: &str) -> Target {
    Target { slide: slide.into(), element: Some(element.into()) }
}

fn slot(owner: Target, param: &str) -> SlotRef {
    SlotRef { owner, param: param.into() }
}

fn set_param(target: Target, param: &str, value: Value) -> Op {
    Op::SetParam { target, param: param.into(), value }
}

fn add(slot: SlotRef, index: Option<usize>, element: Element) -> Op {
    Op::AddElement { slot, index, element }
}

/// Applique `op` au deck avec le registre de base.
fn apply(deck: &mut Deck, op: Op) -> Result<(), OpError> {
    op.apply(deck, &registry())
}

fn apply_err(deck: &mut Deck, op: Op) -> OpError {
    let before = deck.clone();
    let Err(err) = apply(deck, op) else { panic!("l'op devait être refusée") };
    assert_that!(*deck).named("deck après refus").is_equal_to(before);
    err
}

fn sample_ops() -> Vec<Op> {
    vec![
        set_param(at("s-constat", "b-points"), "body", json!(para("Nouveau texte"))),
        set_param(at("s-priorites", "c2"), "body", json!(para("Carte réécrite"))),
        Op::SetStyle {
            slide: "s-libre".into(),
            element: "e-texte".into(),
            style: Style { color: Some("accent".into()), size: Some(TokenValue::Number(48.0)), ..Default::default() },
        },
        Op::SetFrame {
            slide: "s-libre".into(),
            element: "e-image".into(),
            frame: Some(Frame { x: 900.0, y: 20.0, w: 700.0, h: 400.0 }),
        },
        add(slot(slide("s-constat"), "blocks"), Some(0), Element::text("b-intro", para("Introduction"))),
        Op::RemoveElement { slide: "s-priorites".into(), element: "c1".into() },
        set_param(slide("s-constat"), "title", json!("Titre dicté")),
        set_param(slide("s-constat"), "layout", json!("columns")),
        set_param(slide("s-ch1"), "subtitle", json!(null)),
        set_param(slide("s-libre"), "grid", json!({ "columns": 12, "rows": 6 })),
        set_param(slide("s-title"), "title", json!("Titre lié")),
        Op::SetKind { target: slide("s-ch1"), kind: "content".into() },
        Op::AddSlide { index: Some(3), slide: Slide::new("s-nouvelle", "content") },
        Op::MoveSlide { slide: "s-fin".into(), index: 1 },
        Op::RemoveSlide { slide: "s-ch2".into() },
        Op::SetNotes { slide: "s-title".into(), notes: "Bonjour à tous".into() },
        Op::SetTheme { theme: Some("pro".into()) },
        Op::SetToken { path: vec!["color".into(), "accent".into()], value: json!("#2f5fb3") },
        Op::SetToken { path: vec!["component".into(), "card".into(), "height".into()], value: json!("fill") },
        Op::SetDeckParam { key: "author".into(), value: json!("Quelqu'un") },
        Op::SetDeckParam { key: "numbering".into(), value: json!("{n}") },
        Op::SetVar { key: "client".into(), value: json!("Globex") },
        Op::SetVar { key: "confidential".into(), value: json!(null) },
        Op::SetTemplates {
            templates: Some(PackRef {
                id: uuid::Uuid::from_u128(7),
                version: PackVersion::from_hex(&"c".repeat(64)).unwrap_or_else(|| panic!("hex invalide")),
            }),
        },
        Op::SetSlideField { slide: "s-constat".into(), field: SlideField::Name, value: json!("Constat") },
        Op::SetCell {
            slide: "s-libre".into(),
            element: "e-texte".into(),
            cell: Some(Cell { col: 0, row: 0, cols: 8, rows: 4 }),
        },
    ]
}

// ---- Ensemble ---------------------------------------------------------------------

#[test]
fn apply_each_op_changes_the_deck() {
    for op in sample_ops() {
        let mut deck = fixture();
        let before = deck.clone();
        apply(&mut deck, op.clone()).unwrap_or_else(|e| panic!("{op:?} : {e}"));
        asserting(&format!("l'op {op:?} n'a rien changé")).that(&deck).is_not_equal_to(&before);
    }
}

#[test]
fn apply_all_sample_ops_returns_a_valid_deck() {
    let mut deck = fixture();
    apply_all(&mut deck, sample_ops(), &registry()).unwrap();
    assert_that!(validate(&deck, &registry(), None)).is_empty();
}

#[test]
fn apply_all_with_a_failing_op_rolls_back_whole_transaction() {
    let mut deck = fixture();
    let before = deck.clone();
    let mut ops = sample_ops();
    ops.push(set_param(at("s-constat", "inexistant"), "body", json!([])));
    let ops_len = ops.len();
    let err = apply_all(&mut deck, ops, &registry()).unwrap_err();
    assert_that!(err.index()).is_equal_to(ops_len - 1);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::ElementNotFound);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn apply_all_set_grid_and_set_cell_places_the_element() {
    let mut deck = fixture();
    let cell = Cell { col: 0, row: 0, cols: 8, rows: 4 };
    let ops = vec![
        set_param(slide("s-libre"), "grid", json!({ "columns": 12, "rows": 6 })),
        Op::SetCell { slide: "s-libre".into(), element: "e-texte".into(), cell: Some(cell) },
    ];
    apply_all(&mut deck, ops, &registry()).unwrap();
    let placed = deck.slide("s-libre").unwrap().locate("e-texte").unwrap().element.cell;
    assert_that!(placed).is_equal_to(Some(cell));
}

#[test]
fn apply_all_cell_outside_new_grid_returns_error_and_keeps_deck() {
    let mut deck = fixture();
    let before = deck.clone();
    let bad = vec![
        set_param(slide("s-libre"), "grid", json!({ "columns": 4, "rows": 3 })),
        Op::SetCell {
            slide: "s-libre".into(),
            element: "e-texte".into(),
            cell: Some(Cell { col: 3, row: 0, cols: 2, rows: 1 }),
        },
    ];
    assert_that!(apply_all(&mut deck, bad, &registry())).is_err();
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_cell_without_explicit_grid_uses_default_grid() {
    let mut deck = fixture();
    let cell = |col, cols| Op::SetCell {
        slide: "s-libre".into(),
        element: "e-texte".into(),
        cell: Some(Cell { col, row: 0, cols, rows: 1 }),
    };
    assert_that!(apply(&mut deck, cell(0, 8))).is_ok();
    assert_that!(apply(&mut deck, cell(10, 4))).is_err();
}

#[test]
fn set_cell_in_flow_slot_returns_invalid_value() {
    let mut deck = fixture();
    let op = Op::SetCell {
        slide: "s-priorites".into(),
        element: "c1".into(),
        cell: Some(Cell { col: 0, row: 0, cols: 1, rows: 1 }),
    };
    assert_that!(apply_err(&mut deck, op).kind()).is_equal_to(OpErrorKind::InvalidValue);
}

// ---- set_param ------------------------------------------------------------------------

#[test]
fn set_param_plain_on_card_title_sets_the_text() {
    let mut deck = fixture();
    apply(&mut deck, set_param(at("s-priorites", "c1"), "title", json!("  Nouveau titre "))).unwrap();
    let c1 = deck.slide("s-priorites").unwrap().locate("c1").unwrap().element;
    assert_that!(c1.params.text("title")).is_equal_to(Some("Nouveau titre"));
}

#[rstest]
#[case::null(json!(null))]
#[case::empty(json!(""))]
#[case::blank(json!("   "))]
fn set_param_null_removes_the_param(#[case] value: Value) {
    let mut deck = fixture();
    apply(&mut deck, set_param(slide("s-constat"), "title", value)).unwrap();
    assert_that!(deck.slide("s-constat").unwrap().params.get("title")).is_none();
}

#[test]
fn set_param_rich_sets_paragraphs() {
    let mut deck = fixture();
    apply(&mut deck, set_param(at("s-constat", "b-points"), "body", json!(para("Un point")))).unwrap();
    let e = deck.slide("s-constat").unwrap().locate("b-points").unwrap().element;
    assert_that!(e.params.rich("body")).is_equal_to(Some(para("Un point").as_slice()));
}

#[test]
fn set_param_rich_empty_list_removes_the_param() {
    let mut deck = fixture();
    apply(&mut deck, set_param(at("s-constat", "b-points"), "body", json!([]))).unwrap();
    let e = deck.slide("s-constat").unwrap().locate("b-points").unwrap().element;
    assert_that!(e.params.get("body")).is_none();
}

#[rstest]
#[case::title("title", "title")]
#[case::organization("kicker", "organization")]
fn set_param_on_bound_title_writes_deck_params(#[case] param: &str, #[case] field: &str) {
    let mut deck = fixture();
    apply(&mut deck, set_param(slide("s-title"), param, json!(" Lié "))).unwrap();
    let params = serde_json::to_value(&deck.params).unwrap();
    assert_that!(params[field]).is_equal_to(json!("Lié"));
    assert_that!(deck.slide("s-title").unwrap().params.is_empty()).is_true();
}

#[test]
fn set_param_on_bound_title_null_clears_deck_params() {
    let mut deck = fixture();
    apply(&mut deck, set_param(slide("s-title"), "title", json!(null))).unwrap();
    assert_that!(deck.params.title).is_none();
}

#[test]
fn set_param_on_bound_param_to_extra_key_writes_the_extra() {
    let probe = json!({ "base": [{ "name": "cover", "category": "slide",
        "params": [{ "name": "client", "type": "plain", "bind": "client" }] }], "pack": null });
    let registry = WidgetRegistry::from_probe(probe).unwrap();
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "cover" }] }));
    set_param(slide("s"), "client", json!("Globex")).apply(&mut deck, &registry).unwrap();
    assert_that!(deck.params.extra.get("client")).is_equal_to(Some(&json!("Globex")));
}

#[test]
fn set_param_unknown_param_returns_unknown_param() {
    let mut deck = fixture();
    let err = apply_err(&mut deck, set_param(slide("s-constat"), "inexistant", json!("x")));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::UnknownParam);
    assert_that!(err.subject()).is_equal_to(Some("inexistant"));
    assert_that!(err.to_string()).is_equal_to("content has no parameter inexistant".to_owned());
}

#[test]
fn set_param_on_unknown_widget_returns_unknown_widget() {
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "mystere" }] }));
    let err = apply_err(&mut deck, set_param(slide("s"), "title", json!("x")));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::UnknownWidget);
    assert_that!(err.subject()).is_equal_to(Some("mystere"));
}

#[test]
fn set_param_choice_out_of_values_returns_choice_out_of_range() {
    let mut deck = fixture();
    let err = apply_err(&mut deck, set_param(slide("s-constat"), "layout", json!("diagonal")));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ChoiceOutOfRange);
    assert_that!(err.subject()).is_equal_to(Some("layout"));
}

#[rstest]
#[case::zero(json!(0))]
#[case::negative(json!(-3))]
fn set_param_number_out_of_bounds_returns_invalid_value(#[case] count: Value) {
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "content",
        "params": { "blocks": [{ "id": "p", "kind": "stack" }] } }] }));
    let err = apply_err(&mut deck, set_param(at("s", "p"), "count", count));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidValue);
}

#[rstest]
#[case::plain_number("title", json!(3))]
#[case::rich_string("body", json!("texte"))]
#[case::flag_string("highlight", json!("oui"))]
fn set_param_value_of_wrong_type_returns_param_type(#[case] param: &str, #[case] value: Value) {
    let mut deck = fixture();
    let err = apply_err(&mut deck, set_param(at("s-priorites", "c1"), param, value));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ParamType);
    assert_that!(err.subject()).is_equal_to(Some(param));
}

#[test]
fn set_param_on_slot_returns_param_type() {
    let mut deck = fixture();
    let err = apply_err(&mut deck, set_param(slide("s-constat"), "blocks", json!([])));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ParamType);
    assert_that!(err.to_string()).contains("add_element");
}

#[test]
fn set_param_grid_with_zero_columns_returns_invalid_value() {
    let mut deck = fixture();
    let err = apply_err(&mut deck, set_param(slide("s-libre"), "grid", json!({ "columns": 0, "rows": 6 })));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidValue);
}

// ---- set_deck_param, set_slide_field, set_kind -----------------------------------------

#[test]
fn set_deck_param_sets_title() {
    let mut deck = fixture();
    apply(&mut deck, Op::SetDeckParam { key: "title".into(), value: json!(" Nouveau ") }).unwrap();
    assert_that!(deck.params.title).is_equal_to(Some("Nouveau".to_owned()));
}

#[rstest]
#[case("")]
#[case("   ")]
fn set_deck_param_blank_author_stores_none(#[case] blank: &str) {
    let mut deck = fixture();
    apply(&mut deck, Op::SetDeckParam { key: "author".into(), value: json!(blank) }).unwrap();
    assert_that!(deck.params.author).is_none();
}

#[test]
fn set_slide_field_name_sets_the_name() {
    let mut deck = fixture();
    let op = Op::SetSlideField { slide: "s-constat".into(), field: SlideField::Name, value: json!(" Constat ") };
    apply(&mut deck, op).unwrap();
    assert_that!(deck.slide("s-constat").unwrap().name).is_equal_to(Some("Constat".to_owned()));
}

#[test]
fn set_kind_keeps_params() {
    let mut deck = fixture();
    let before = deck.slide("s-ch1").unwrap().params.clone();
    apply(&mut deck, Op::SetKind { target: slide("s-ch1"), kind: "content".into() }).unwrap();
    let s = deck.slide("s-ch1").unwrap();
    assert_that!(s.kind.as_str()).is_equal_to("content");
    assert_that!(s.params).is_equal_to(before);
}

#[test]
fn set_kind_on_element_sets_the_kind() {
    let mut deck = fixture();
    apply(&mut deck, Op::SetKind { target: at("s-constat", "b-points"), kind: "card".into() }).unwrap();
    let e = deck.slide("s-constat").unwrap().locate("b-points").unwrap().element;
    assert_that!(e.kind.as_str()).is_equal_to("card");
}

#[test]
fn set_kind_to_slide_widget_on_element_returns_wrong_category() {
    let mut deck = fixture();
    let err = apply_err(&mut deck, Op::SetKind { target: at("s-constat", "b-points"), kind: "content".into() });
    assert_that!(err.kind()).is_equal_to(OpErrorKind::WrongCategory);
    assert_that!(err.subject()).is_equal_to(Some("content"));
}

#[test]
fn set_kind_to_unknown_widget_returns_unknown_widget() {
    let mut deck = fixture();
    let err = apply_err(&mut deck, Op::SetKind { target: slide("s-ch1"), kind: "mystere".into() });
    assert_that!(err.kind()).is_equal_to(OpErrorKind::UnknownWidget);
}

// ---- add_element ------------------------------------------------------------------------

fn with_stack() -> Deck {
    deck(json!({ "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
        { "id": "p", "kind": "stack", "params": { "children": [
            { "id": "a", "kind": "text" }, { "id": "b", "kind": "text" }
        ] } }
    ] } }] }))
}

fn children(deck: &Deck) -> Vec<String> {
    let p = deck.slides[0].locate("p").unwrap_or_else(|| panic!("p absent")).element;
    p.params.slot("children").unwrap_or_default().iter().map(|e| e.id.clone()).collect()
}

#[test]
fn add_element_into_stack_children_inserts_at_index() {
    let mut deck = with_stack();
    apply(&mut deck, add(slot(at("s", "p"), "children"), Some(1), Element::text("n", vec![]))).unwrap();
    assert_that!(children(&deck)).is_equal_to(vec!["a".to_owned(), "n".to_owned(), "b".to_owned()]);
}

#[test]
fn add_element_at_index_equal_to_len_appends() {
    let mut deck = with_stack();
    apply(&mut deck, add(slot(at("s", "p"), "children"), Some(2), Element::text("n", vec![]))).unwrap();
    assert_that!(children(&deck)).is_equal_to(vec!["a".to_owned(), "b".to_owned(), "n".to_owned()]);
}

#[test]
fn add_element_at_index_past_len_returns_index_out_of_range() {
    let mut deck = with_stack();
    let err = apply_err(&mut deck, add(slot(at("s", "p"), "children"), Some(3), Element::text("n", vec![])));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::IndexOutOfRange);
}

#[test]
fn add_element_into_slide_blocks_appends() {
    let mut deck = fixture();
    apply(&mut deck, add(slot(slide("s-priorites"), "blocks"), None, Element::text("c4", vec![]))).unwrap();
    let blocks = deck.slide("s-priorites").unwrap().params.slot("blocks").unwrap();
    assert_that!(blocks.last().map(|e| e.id.as_str())).is_equal_to(Some("c4"));
}

#[test]
fn add_element_into_empty_slot_creates_it() {
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "blank" }] }));
    let mut e = Element::text("t", vec![]);
    e.frame = Some(Frame { x: 0.0, y: 0.0, w: 10.0, h: 10.0 });
    apply(&mut deck, add(slot(slide("s"), "elements"), None, e)).unwrap();
    assert_that!(deck.slides[0].params.slot("elements").map(<[Element]>::len)).is_equal_to(Some(1));
}

/// Chaîne de `n` canvas imbriqués (`c0` ⊃ `c1` ⊃ …), `c0` dans `elements`.
fn canvas_chain(n: usize) -> Deck {
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "blank" }] }));
    let canvas = |i: usize| {
        let mut e = Element::new(format!("c{i}"), "canvas");
        e.frame = Some(Frame { x: 0.0, y: 0.0, w: 10.0, h: 10.0 });
        e
    };
    apply(&mut deck, add(slot(slide("s"), "elements"), None, canvas(0))).unwrap_or_else(|e| panic!("canvas 0 : {e}"));
    for i in 1..n {
        let op = add(slot(at("s", &format!("c{}", i - 1)), "children"), None, canvas(i));
        apply(&mut deck, op).unwrap_or_else(|e| panic!("canvas {i} : {e}"));
    }
    deck
}

#[test]
fn add_element_beyond_max_slot_depth_returns_invalid_value() {
    let mut deck = canvas_chain(MAX_SLOT_DEPTH);
    let last = format!("c{}", MAX_SLOT_DEPTH - 1);
    let err = apply_err(&mut deck, add(slot(at("s", &last), "children"), None, Element::new("trop", "canvas")));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidValue);
    assert_that!(err.to_string()).contains("nest too deeply");
}

#[test]
fn add_element_whose_subtree_exceeds_max_slot_depth_returns_invalid_value() {
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "blank" }] }));
    let mut tree = json!({ "id": format!("c{MAX_SLOT_DEPTH}"), "kind": "canvas" });
    for i in (0..MAX_SLOT_DEPTH).rev() {
        tree = json!({ "id": format!("c{i}"), "kind": "canvas", "params": { "children": [tree] } });
    }
    let err = apply_err(&mut deck, add(slot(slide("s"), "elements"), None, element(tree)));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidValue);
}

#[test]
fn replace_element_with_too_deep_tree_returns_invalid_value() {
    let mut deck = canvas_chain(2);
    let mut tree = json!({ "id": "x", "kind": "canvas" });
    for i in 0..MAX_SLOT_DEPTH {
        tree = json!({ "id": format!("x{i}"), "kind": "canvas", "params": { "children": [tree] } });
    }
    let mut e = element(tree);
    e.id = "c1".into();
    let err = apply_err(&mut deck, Op::ReplaceElement { slide: "s".into(), element: e });
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidValue);
}

#[test]
fn add_slide_with_too_deep_tree_returns_invalid_value() {
    let mut deck = fixture();
    let mut tree = json!({ "id": "x", "kind": "canvas" });
    for i in 0..=MAX_SLOT_DEPTH {
        tree = json!({ "id": format!("x{i}"), "kind": "canvas", "params": { "children": [tree] } });
    }
    let s: Slide =
        serde_json::from_value(json!({ "id": "s-neuve", "kind": "blank", "params": { "elements": [tree] } })).unwrap();
    let err = apply_err(&mut deck, Op::AddSlide { index: None, slide: s });
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidValue);
}

#[test]
fn add_element_into_non_slot_param_returns_param_type() {
    let mut deck = fixture();
    let err = apply_err(&mut deck, add(slot(at("s-priorites", "c1"), "body"), None, Element::text("n", vec![])));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ParamType);
    let c1 = deck.slide("s-priorites").unwrap().locate("c1").unwrap().element;
    assert_that!(c1.params.rich("body").map(<[Paragraph]>::is_empty)).is_equal_to(Some(false));
}

#[test]
fn add_element_into_undeclared_param_returns_unknown_param() {
    let mut deck = fixture();
    let err = apply_err(&mut deck, add(slot(slide("s-constat"), "zone"), None, Element::text("n", vec![])));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::UnknownParam);
}

#[test]
fn add_element_rejected_by_accepts_returns_slot_rejects() {
    let probe = json!({ "base": [
        { "name": "only-text", "category": "slide",
          "params": [{ "name": "items", "type": "slot", "mode": "flow", "accepts": ["text"] }] },
        { "name": "text", "category": "element", "params": [{ "name": "body", "type": "rich" }] },
        { "name": "shape", "category": "element", "params": [] }
    ], "pack": null });
    let registry = WidgetRegistry::from_probe(probe).unwrap();
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "only-text" }] }));
    let items = || slot(slide("s"), "items");
    add(items(), None, Element::text("t", vec![])).apply(&mut deck, &registry).unwrap();
    let before = deck.clone();
    let err = add(items(), None, Element::new("f", "shape")).apply(&mut deck, &registry).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::SlotRejects);
    assert_that!(err.subject()).is_equal_to(Some("shape"));
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn add_element_of_unknown_widget_returns_unknown_widget() {
    let mut deck = fixture();
    let err = apply_err(&mut deck, add(slot(slide("s-constat"), "blocks"), None, Element::new("n", "mystere")));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::UnknownWidget);
}

#[test]
fn add_element_of_slide_widget_returns_wrong_category() {
    let mut deck = fixture();
    let err = apply_err(&mut deck, add(slot(slide("s-constat"), "blocks"), None, Element::new("n", "content")));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::WrongCategory);
}

#[test]
fn add_element_duplicate_id_returns_duplicate_id() {
    let mut deck = fixture();
    let err = apply_err(&mut deck, add(slot(slide("s-priorites"), "blocks"), None, Element::text("c1", vec![])));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::DuplicateId);
    assert_that!(err.subject()).is_equal_to(Some("c1"));
}

#[test]
fn add_element_with_owner_missing_returns_element_not_found() {
    let mut deck = fixture();
    let err = apply_err(&mut deck, add(slot(at("s-constat", "absent"), "children"), None, Element::new("n", "text")));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ElementNotFound);
}

// ---- remove_element ---------------------------------------------------------------------

#[test]
fn remove_element_with_duplicate_owner_ids_removes_the_located_element() {
    // Deux canvas `g` : l'élément `t` est dans le second, à l'index 1, alors
    // que le premier `g` n'a qu'un enfant.
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "content", "params": {
        "blocks": [{ "id": "g", "kind": "canvas", "params": { "children": [{ "id": "z", "kind": "text" }] } }],
        "elements": [{ "id": "g", "kind": "canvas", "params": { "children": [
            { "id": "y", "kind": "text" }, { "id": "t", "kind": "text" }
        ] } }]
    } }] }));
    apply(&mut deck, Op::RemoveElement { slide: "s".into(), element: "t".into() }).unwrap();
    let ids: Vec<&str> = deck.slides[0].element_ids().collect();
    assert_that!(ids).is_equal_to(vec!["g", "z", "g", "y"]);
}

#[test]
fn remove_element_missing_returns_element_not_found() {
    let mut deck = fixture();
    let err = apply_err(&mut deck, Op::RemoveElement { slide: "s-constat".into(), element: "absent".into() });
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ElementNotFound);
}

// ---- Canonique, JSON, schéma ---------------------------------------------------------------

#[test]
fn apply_all_add_slide_with_blank_prose_returns_canonical_deck() {
    let mut deck = fixture();
    let mut slide = Slide::new("s-neuve", "content");
    slide.params.set("title", ParamValue::Text(" ".into()));
    slide.notes = "\n".into();
    apply_all(&mut deck, vec![Op::AddSlide { index: None, slide }], &registry()).unwrap();
    let added = deck.slide("s-neuve").unwrap();
    assert_that!(added.params.get("title")).is_none();
    assert_that!(added.notes).is_empty();
    let mut canonical = deck.clone();
    canonicalize(&mut canonical);
    assert_that!(deck).is_equal_to(canonical);
}

#[test]
fn deck_to_json_then_from_json_returns_equal_valid_deck() {
    let deck = fixture();
    let again = Deck::from_json(&deck.to_json().unwrap()).unwrap();
    assert_that!(deck).is_equal_to(&again);
    assert_that!(validate(&deck, &registry(), None)).is_empty();
}

#[test]
fn target_without_element_serializes_without_the_field() {
    assert_that!(serde_json::to_value(slide("s1")).unwrap()).is_equal_to(json!({ "slide": "s1" }));
    let back: Target = serde_json::from_value(json!({ "slide": "s1", "element": "e1" })).unwrap();
    assert_that!(back).is_equal_to(at("s1", "e1"));
}

#[test]
fn ops_schema_generates_schema_naming_ops() {
    let s = ops_schema().unwrap().to_string();
    for name in ["set_param", "set_deck_param", "set_kind", "add_element", "set_token"] {
        assert_that!(s.as_str()).contains(name);
    }
    assert_that!(s.as_str()).does_not_contain("\"set_text\"");
    // t-423 : `set_grid` redevient une op (grille et recadrage des éléments).
    assert_that!(s.as_str()).contains("\"set_grid\"");
}

#[test]
fn op_errors_localize_with_their_subject() {
    let fr = Localizer::new(Some("fr"));
    let cases: Vec<(Op, &str)> = vec![
        (Op::SetNotes { slide: "absente".into(), notes: String::new() }, "Slide introuvable : absente"),
        (set_param(at("s-constat", "x"), "body", json!([])), "Élément introuvable : x"),
        (Op::MoveSlide { slide: "s-fin".into(), index: 99 }, "Position hors limites"),
        (set_param(slide("s-fin"), "text", json!(3)), "Valeur invalide pour text"),
        (Op::SetToken { path: vec![], value: json!(1) }, "Valeur invalide pour path"),
        (set_param(slide("s-libre"), "grid", json!({ "columns": 0, "rows": 6 })), "Valeur invalide pour grid"),
        (
            Op::SetFrame {
                slide: "s-libre".into(),
                element: "e-texte".into(),
                frame: Some(Frame { x: 0.0, y: 0.0, w: 0.0, h: 1.0 }),
            },
            "Valeur invalide pour frame",
        ),
        (Op::SetKind { target: slide("s-fin"), kind: "inconnu".into() }, "Widget inconnu : inconnu"),
        (set_param(slide("s-fin"), "titre", json!("x")), "Paramètre inconnu : titre"),
        (set_param(slide("s-constat"), "layout", json!("x")), "Valeur non permise pour layout"),
        (Op::SetKind { target: slide("s-fin"), kind: "text".into() }, "text ne peut pas être utilisé ici"),
        (Op::AddSlide { index: None, slide: Slide::new("s-fin", "content") }, "Identifiant déjà utilisé : s-fin"),
    ];
    for (op, expected) in cases {
        let mut deck = fixture();
        let err = apply(&mut deck, op).unwrap_err();
        asserting(expected).that(&err.localize(&fr)).is_equal_to(expected.to_owned());
    }
}

#[test]
fn slot_rejects_localizes_with_the_kind() {
    let probe = json!({ "base": [
        { "name": "only-text", "category": "slide",
          "params": [{ "name": "items", "type": "slot", "mode": "flow", "accepts": ["text"] }] },
        { "name": "text", "category": "element", "params": [] },
        { "name": "shape", "category": "element", "params": [] }
    ], "pack": null });
    let registry = WidgetRegistry::from_probe(probe).unwrap();
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "only-text" }] }));
    let err = add(slot(slide("s"), "items"), None, Element::new("f", "shape")).apply(&mut deck, &registry).unwrap_err();
    assert_that!(err.localize(&Localizer::new(Some("fr")))).is_equal_to("shape n'est pas accepté ici".to_owned());
    assert_that!(err.to_string()).is_equal_to("items does not accept shape".to_owned());
}

// ---- Sous-arbres insérés : paramètres typés, genres connus, slots qui acceptent ---------

/// Slide `free` (slot `items` libre de tout filtre), élément `box` dont le
/// slot `children` n'accepte que `text`.
fn accepts_registry() -> WidgetRegistry {
    WidgetRegistry::from_probe(json!({ "base": [
        { "name": "free", "category": "slide",
          "params": [{ "name": "items", "type": "slot", "mode": "flow" }] },
        { "name": "box", "category": "element",
          "params": [{ "name": "children", "type": "slot", "mode": "flow", "accepts": ["text"] }] },
        { "name": "text", "category": "element", "params": [{ "name": "body", "type": "rich" }] },
        { "name": "shape", "category": "element", "params": [] }
    ], "pack": null }))
    .unwrap_or_else(|e| panic!("registre de test : {e}"))
}

#[test]
fn add_element_with_wrong_typed_param_returns_param_type() {
    let mut deck = fixture();
    let e = element(json!({ "id": "n", "kind": "text", "params": { "body": "hello" } }));
    let err = apply_err(&mut deck, add(slot(slide("s-constat"), "blocks"), None, e));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ParamType);
    assert_that!(err.subject()).is_equal_to(Some("body"));
}

#[test]
fn add_element_with_undeclared_nested_param_returns_unknown_param() {
    let mut deck = fixture();
    let e = element(json!({ "id": "n", "kind": "stack", "params": { "children": [
        { "id": "n1", "kind": "text", "params": { "titre": "x" } }
    ] } }));
    let err = apply_err(&mut deck, add(slot(slide("s-constat"), "blocks"), None, e));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::UnknownParam);
}

#[test]
fn add_element_with_nested_unknown_kind_returns_unknown_widget() {
    let mut deck = fixture();
    let e = element(json!({ "id": "n", "kind": "stack", "params": { "children": [
        { "id": "n1", "kind": "mystere" }
    ] } }));
    let err = apply_err(&mut deck, add(slot(slide("s-constat"), "blocks"), None, e));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::UnknownWidget);
    assert_that!(err.subject()).is_equal_to(Some("mystere"));
}

#[test]
fn add_element_with_nested_slide_kind_returns_wrong_category() {
    let mut deck = fixture();
    let e = element(json!({ "id": "n", "kind": "stack", "params": { "children": [
        { "id": "n1", "kind": "content" }
    ] } }));
    let err = apply_err(&mut deck, add(slot(slide("s-constat"), "blocks"), None, e));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::WrongCategory);
}

#[test]
fn add_element_with_nested_child_refused_by_accepts_returns_slot_rejects() {
    let registry = accepts_registry();
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "free" }] }));
    let before = deck.clone();
    let e = element(json!({ "id": "b", "kind": "box", "params": { "children": [
        { "id": "t", "kind": "text" }, { "id": "f", "kind": "shape" }
    ] } }));
    let err = add(slot(slide("s"), "items"), None, e).apply(&mut deck, &registry).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::SlotRejects);
    assert_that!(err.subject()).is_equal_to(Some("shape"));
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn add_slide_with_choice_out_of_values_returns_choice_out_of_range() {
    let mut deck = fixture();
    let s: Slide =
        serde_json::from_value(json!({ "id": "s-neuve", "kind": "content", "params": { "layout": "bogus" } })).unwrap();
    let err = apply_err(&mut deck, Op::AddSlide { index: None, slide: s });
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ChoiceOutOfRange);
    assert_that!(err.subject()).is_equal_to(Some("layout"));
}

#[test]
fn add_slide_with_wrong_typed_nested_param_returns_param_type() {
    let mut deck = fixture();
    let s: Slide = serde_json::from_value(json!({ "id": "s-neuve", "kind": "content", "params": { "blocks": [
        { "id": "n", "kind": "card", "params": { "highlight": "oui" } }
    ] } }))
    .unwrap();
    let err = apply_err(&mut deck, Op::AddSlide { index: None, slide: s });
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ParamType);
}

#[test]
fn replace_element_with_wrong_typed_nested_param_returns_param_type() {
    let mut deck = with_stack();
    let e = element(json!({ "id": "p", "kind": "stack", "params": { "children": [
        { "id": "a", "kind": "text", "params": { "body": 3 } }
    ] } }));
    let err = apply_err(&mut deck, Op::ReplaceElement { slide: "s".into(), element: e });
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ParamType);
    assert_that!(err.subject()).is_equal_to(Some("body"));
}

#[test]
fn replace_element_with_valid_subtree_replaces_it() {
    let mut deck = with_stack();
    let e = element(json!({ "id": "p", "kind": "stack", "params": { "layout": "columns", "children": [
        { "id": "a", "kind": "text", "params": { "body": [{ "runs": [{ "t": "x" }] }] } }
    ] } }));
    apply(&mut deck, Op::ReplaceElement { slide: "s".into(), element: e }).unwrap();
    assert_that!(children(&deck)).is_equal_to(vec!["a".to_owned()]);
}

#[test]
fn set_kind_of_child_to_kind_refused_by_its_slot_returns_slot_rejects() {
    let registry = accepts_registry();
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "free", "params": { "items": [
        { "id": "b", "kind": "box", "params": { "children": [{ "id": "t", "kind": "text" }] } }
    ] } }] }));
    let before = deck.clone();
    let err = Op::SetKind { target: at("s", "t"), kind: "shape".into() }.apply(&mut deck, &registry).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::SlotRejects);
    assert_that!(deck).is_equal_to(before);
}
