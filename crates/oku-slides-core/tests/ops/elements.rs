//! Ops sur les paramètres, les éléments et les sous-arbres insérés.

use oku_slides_core::kinds::base;
use oku_slides_core::kinds::essentiel::CONTENT;
use oku_slides_core::*;
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::*;

// ---- set_param ------------------------------------------------------------------------

#[test]
fn set_param_shape_path_keeps_a_well_formed_path() {
    let mut deck = fixture();
    let ring = json!({ "d": "M 0 0 L 1 0 L 1 1 Z M 0.25 0.25 L 0.25 0.75 L 0.75 0.75 Z" });
    apply(&mut deck, set_param(at("s-libre", "e-fond"), "path", ring.clone())).unwrap();
    let e = deck.slide("s-libre").and_then(|s| s.locate("e-fond")).unwrap().element;
    assert_that!(e.params.value("path")).is_equal_to(Some(&ring));
}

#[rstest]
#[case::malformed(json!({ "d": "L 0 0" }), OpErrorKind::InvalidValue)]
#[case::empty(json!({ "d": "" }), OpErrorKind::InvalidValue)]
#[case::unknown_key(json!({ "d": "M 0 0", "z": 1 }), OpErrorKind::InvalidValue)]
#[case::bare_text(json!("M 0 0 L 1 1"), OpErrorKind::ParamType)]
fn set_param_shape_path_refused_returns_its_kind(#[case] value: Value, #[case] kind: OpErrorKind) {
    let mut deck = fixture();
    let err = apply_err(&mut deck, set_param(at("s-libre", "e-fond"), "path", value));
    assert_that!(err.kind()).is_equal_to(kind);
}

#[test]
fn add_element_shape_with_a_malformed_path_is_refused() {
    let mut deck = fixture();
    let shape = element(json!({ "id": "e-chemin", "kind": "base/shape", "frame": { "x": 0, "y": 0, "w": 10, "h": 10 },
        "params": { "shape": "path", "path": { "d": "M 0 0 L" } } }));
    let err = apply_err(&mut deck, add(slot(slide("s-libre"), "elements"), None, shape));
    assert_that!(err.value_reason()).is_equal_to(Some(ValueReason::Path));
}

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
    let probe = json!({ "packs": [{ "namespace": "t", "kind": "layouts", "widgets": [{ "name": "cover", "category": "slide",
        "params": [{ "name": "client", "type": "plain", "bind": "client" }] }] }] });
    let registry = WidgetRegistry::from_probe(probe).unwrap();
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "t/cover" }] }));
    set_param(slide("s"), "client", json!("Globex")).apply(&mut deck, &registry).unwrap();
    assert_that!(deck.params.extra.get("client")).is_equal_to(Some(&json!("Globex")));
}

#[test]
fn set_param_unknown_param_returns_unknown_param() {
    let mut deck = fixture();
    let err = apply_err(&mut deck, set_param(slide("s-constat"), "inexistant", json!("x")));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::UnknownParam);
    assert_that!(err.subject()).is_equal_to(Some("inexistant"));
    assert_that!(err.to_string()).is_equal_to("essentiel/content has no parameter inexistant".to_owned());
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
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "essentiel/content",
        "params": { "blocks": [{ "id": "p", "kind": "base/stack" }] } }] }));
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

// ---- add_element ------------------------------------------------------------------------

fn with_stack() -> Deck {
    deck(json!({ "slides": [{ "id": "s", "kind": "essentiel/content", "params": { "blocks": [
        { "id": "p", "kind": "base/stack", "params": { "children": [
            { "id": "a", "kind": "base/text" }, { "id": "b", "kind": "base/text" }
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
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "essentiel/blank" }] }));
    let mut e = Element::text("t", vec![]);
    e.frame = Some(Frame { x: 0.0, y: 0.0, w: 10.0, h: 10.0 });
    apply(&mut deck, add(slot(slide("s"), "elements"), None, e)).unwrap();
    assert_that!(deck.slides[0].params.slot("elements").map(<[Element]>::len)).is_equal_to(Some(1));
}

/// Chaîne de `n` `base/stack` imbriqués (`c0` ⊃ `c1` ⊃ …), `c0` dans `elements`.
fn stack_chain(n: usize) -> Deck {
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "essentiel/blank" }] }));
    let stack = |i: usize| {
        let mut e = Element::new(format!("c{i}"), base::STACK);
        e.frame = Some(Frame { x: 0.0, y: 0.0, w: 10.0, h: 10.0 });
        e
    };
    apply(&mut deck, add(slot(slide("s"), "elements"), None, stack(0))).unwrap_or_else(|e| panic!("stack 0 : {e}"));
    for i in 1..n {
        let op = add(slot(at("s", &format!("c{}", i - 1)), "children"), None, stack(i));
        apply(&mut deck, op).unwrap_or_else(|e| panic!("stack {i} : {e}"));
    }
    deck
}

#[test]
fn add_element_beyond_max_slot_depth_returns_invalid_value() {
    let mut deck = stack_chain(MAX_SLOT_DEPTH);
    let last = format!("c{}", MAX_SLOT_DEPTH - 1);
    let err = apply_err(&mut deck, add(slot(at("s", &last), "children"), None, Element::new("trop", base::STACK)));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidValue);
    assert_that!(err.to_string()).contains("nest too deeply");
}

#[test]
fn add_element_whose_subtree_exceeds_max_slot_depth_returns_invalid_value() {
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "essentiel/blank" }] }));
    let mut tree = json!({ "id": format!("c{MAX_SLOT_DEPTH}"), "kind": "base/stack" });
    for i in (0..MAX_SLOT_DEPTH).rev() {
        tree = json!({ "id": format!("c{i}"), "kind": "base/stack", "params": { "children": [tree] } });
    }
    let err = apply_err(&mut deck, add(slot(slide("s"), "elements"), None, element(tree)));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidValue);
}

#[test]
fn replace_element_with_too_deep_tree_returns_invalid_value() {
    let mut deck = stack_chain(2);
    let mut tree = json!({ "id": "x", "kind": "base/stack" });
    for i in 0..MAX_SLOT_DEPTH {
        tree = json!({ "id": format!("x{i}"), "kind": "base/stack", "params": { "children": [tree] } });
    }
    let mut e = element(tree);
    e.id = "c1".into();
    let err = apply_err(&mut deck, Op::ReplaceElement { slide: "s".into(), element: e });
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidValue);
}

#[test]
fn add_slide_with_too_deep_tree_returns_invalid_value() {
    let mut deck = fixture();
    let mut tree = json!({ "id": "x", "kind": "base/stack" });
    for i in 0..=MAX_SLOT_DEPTH {
        tree = json!({ "id": format!("x{i}"), "kind": "base/stack", "params": { "children": [tree] } });
    }
    let s: Slide =
        serde_json::from_value(json!({ "id": "s-neuve", "kind": "essentiel/blank", "params": { "elements": [tree] } }))
            .unwrap();
    let err = apply_err(&mut deck, Op::AddSlide { index: None, defaults: false, slide: s });
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
    let probe = json!({ "packs": [{ "namespace": "t", "kind": "layouts", "widgets": [
        { "name": "only-text", "category": "slide",
          "params": [{ "name": "items", "type": "slot", "mode": "flow", "accepts": ["base/text"] }] },
    ] }, { "namespace": "base", "kind": "widgets", "widgets": [
        { "name": "text", "category": "element", "params": [{ "name": "body", "type": "rich" }] },
        { "name": "shape", "category": "element", "params": [] }
    ] }] });
    let registry = WidgetRegistry::from_probe(probe).unwrap();
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "t/only-text" }] }));
    let items = || slot(slide("s"), "items");
    add(items(), None, Element::text("t", vec![])).apply(&mut deck, &registry).unwrap();
    let before = deck.clone();
    let err = add(items(), None, Element::new("f", base::SHAPE)).apply(&mut deck, &registry).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::SlotRejects);
    assert_that!(err.subject()).is_equal_to(Some(base::SHAPE));
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
    let err = apply_err(&mut deck, add(slot(slide("s-constat"), "blocks"), None, Element::new("n", CONTENT)));
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
    let err =
        apply_err(&mut deck, add(slot(at("s-constat", "absent"), "children"), None, Element::new("n", base::TEXT)));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ElementNotFound);
}

// ---- remove_element ---------------------------------------------------------------------

#[test]
fn remove_element_with_duplicate_owner_ids_removes_the_located_element() {
    // Deux stacks `g` : l'élément `t` est dans le second, à l'index 1, alors
    // que le premier `g` n'a qu'un enfant.
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "essentiel/content", "params": {
        "blocks": [{ "id": "g", "kind": "base/stack", "params": { "children": [{ "id": "z", "kind": "base/text" }] } }],
        "elements": [{ "id": "g", "kind": "base/stack", "params": { "children": [
            { "id": "y", "kind": "base/text" }, { "id": "t", "kind": "base/text" }
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

// ---- Sous-arbres insérés : paramètres typés, genres connus, slots qui acceptent ---------

/// Slide `free` (slot `items` libre de tout filtre), élément `box` dont le
/// slot `children` n'accepte que `text`.
fn accepts_registry() -> WidgetRegistry {
    WidgetRegistry::from_probe(json!({ "packs": [{ "namespace": "t", "kind": "layouts", "widgets": [
        { "name": "free", "category": "slide",
          "params": [{ "name": "items", "type": "slot", "mode": "flow" }] },
    ] }, { "namespace": "t", "kind": "widgets", "widgets": [
        { "name": "box", "category": "element",
          "params": [{ "name": "children", "type": "slot", "mode": "flow", "accepts": ["base/text"] }] },
    ] }, { "namespace": "base", "kind": "widgets", "widgets": [
        { "name": "text", "category": "element", "params": [{ "name": "body", "type": "rich" }] },
        { "name": "shape", "category": "element", "params": [] }
    ] }] }))
    .unwrap_or_else(|e| panic!("registre de test : {e}"))
}

#[test]
fn add_element_with_wrong_typed_param_returns_param_type() {
    let mut deck = fixture();
    let e = element(json!({ "id": "n", "kind": "base/text", "params": { "body": "hello" } }));
    let err = apply_err(&mut deck, add(slot(slide("s-constat"), "blocks"), None, e));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ParamType);
    assert_that!(err.subject()).is_equal_to(Some("body"));
}

#[test]
fn add_element_with_undeclared_nested_param_returns_unknown_param() {
    let mut deck = fixture();
    let e = element(json!({ "id": "n", "kind": "base/stack", "params": { "children": [
        { "id": "n1", "kind": "base/text", "params": { "titre": "x" } }
    ] } }));
    let err = apply_err(&mut deck, add(slot(slide("s-constat"), "blocks"), None, e));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::UnknownParam);
}

#[test]
fn add_element_with_nested_unknown_kind_returns_unknown_widget() {
    let mut deck = fixture();
    let e = element(json!({ "id": "n", "kind": "base/stack", "params": { "children": [
        { "id": "n1", "kind": "mystere" }
    ] } }));
    let err = apply_err(&mut deck, add(slot(slide("s-constat"), "blocks"), None, e));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::UnknownWidget);
    assert_that!(err.subject()).is_equal_to(Some("mystere"));
}

#[test]
fn add_element_with_nested_slide_kind_returns_wrong_category() {
    let mut deck = fixture();
    let e = element(json!({ "id": "n", "kind": "base/stack", "params": { "children": [
        { "id": "n1", "kind": "essentiel/content" }
    ] } }));
    let err = apply_err(&mut deck, add(slot(slide("s-constat"), "blocks"), None, e));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::WrongCategory);
}

#[test]
fn add_element_with_nested_child_refused_by_accepts_returns_slot_rejects() {
    let registry = accepts_registry();
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "t/free" }] }));
    let before = deck.clone();
    let e = element(json!({ "id": "b", "kind": "t/box", "params": { "children": [
        { "id": "t", "kind": "base/text" }, { "id": "f", "kind": "base/shape" }
    ] } }));
    let err = add(slot(slide("s"), "items"), None, e).apply(&mut deck, &registry).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::SlotRejects);
    assert_that!(err.subject()).is_equal_to(Some(base::SHAPE));
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn add_slide_with_choice_out_of_values_returns_choice_out_of_range() {
    let mut deck = fixture();
    let s: Slide = serde_json::from_value(
        json!({ "id": "s-neuve", "kind": "essentiel/content", "params": { "layout": "bogus" } }),
    )
    .unwrap();
    let err = apply_err(&mut deck, Op::AddSlide { index: None, defaults: false, slide: s });
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ChoiceOutOfRange);
    assert_that!(err.subject()).is_equal_to(Some("layout"));
}

#[test]
fn add_slide_with_wrong_typed_nested_param_returns_param_type() {
    let mut deck = fixture();
    let s: Slide =
        serde_json::from_value(json!({ "id": "s-neuve", "kind": "essentiel/content", "params": { "blocks": [
        { "id": "n", "kind": "base/card", "params": { "highlight": "oui" } }
    ] } }))
        .unwrap();
    let err = apply_err(&mut deck, Op::AddSlide { index: None, defaults: false, slide: s });
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ParamType);
}

#[test]
fn replace_element_with_wrong_typed_nested_param_returns_param_type() {
    let mut deck = with_stack();
    let e = element(json!({ "id": "p", "kind": "base/stack", "params": { "children": [
        { "id": "a", "kind": "base/text", "params": { "body": 3 } }
    ] } }));
    let err = apply_err(&mut deck, Op::ReplaceElement { slide: "s".into(), element: e });
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ParamType);
    assert_that!(err.subject()).is_equal_to(Some("body"));
}

#[test]
fn replace_element_with_valid_subtree_replaces_it() {
    let mut deck = with_stack();
    let e = element(json!({ "id": "p", "kind": "base/stack", "params": { "layout": "columns", "children": [
        { "id": "a", "kind": "base/text", "params": { "body": [{ "runs": [{ "t": "x" }] }] } }
    ] } }));
    apply(&mut deck, Op::ReplaceElement { slide: "s".into(), element: e }).unwrap();
    assert_that!(children(&deck)).is_equal_to(vec!["a".to_owned()]);
}

#[test]
fn set_kind_of_child_to_kind_refused_by_its_slot_returns_slot_rejects() {
    let registry = accepts_registry();
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "t/free", "params": { "items": [
        { "id": "b", "kind": "t/box", "params": { "children": [{ "id": "t", "kind": "base/text" }] } }
    ] } }] }));
    let before = deck.clone();
    let err = Op::SetKind { target: at("s", "t"), kind: base::SHAPE.into() }.apply(&mut deck, &registry).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::SlotRejects);
    assert_that!(deck).is_equal_to(before);
}
