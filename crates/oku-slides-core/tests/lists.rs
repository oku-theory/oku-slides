//! `list` parameters on the `roadmap` fixture: `edit_list` ops, defaults, bounds and validation.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
mod common;

use common::list_registry;
use oku_slides_core::*;
use serde_json::{Value, json};
use speculoos::prelude::*;

fn fixture() -> Deck {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("fixture {path} illisible : {e}"));
    Deck::from_json(&json).unwrap_or_else(|e| panic!("fixture {path} invalide : {e}"))
}

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
}

fn add(element: &Value) -> Op {
    op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": element }))
}

/// Comme [`add`], pour un objet nouveau : les listes sont amorcées.
fn add_new(element: &Value) -> Op {
    op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "defaults": true, "element": element }))
}

fn list_of(deck: &Deck, slide: &str, element: &str, param: &str) -> Option<Vec<String>> {
    let Some(s) = deck.slide(slide) else { panic!("slide {slide} absente") };
    let Some(found) = s.locate(element) else { panic!("élément {element} absent : {:?}", s.params) };
    let e = found.element;
    e.params.list(param).map(|l| l.iter().map(|i| i.id.clone()).collect())
}

fn at(slide: &str, element: &str) -> Target {
    Target { slide: slide.into(), element: Some(element.into()) }
}

fn set_param(target: Target, param: &str, value: Value) -> Op {
    Op::SetParam { target, param: param.into(), value }
}

fn apply(deck: &mut Deck, op: Op) -> Result<(), TransactionError> {
    apply_all(deck, vec![op], &list_registry())
}

fn apply_err(deck: &mut Deck, op: Op) -> OpErrorKind {
    let Err(err) = apply(deck, op) else { panic!("l'op devait être refusée") };
    err.op_error().kind()
}

#[test]
fn add_element_seeds_the_list_default_with_d_ids() {
    let mut deck = fixture();
    apply(&mut deck, add_new(&json!({ "id": "f", "kind": "test-el/liste" }))).unwrap();
    assert_that!(list_of(&deck, "s-constat", "f", "etapes")).is_equal_to(Some(vec!["d0".to_owned()]));
}

#[test]
fn edit_list_insert_then_set_field_edits_the_deck() {
    let mut deck = fixture();
    apply(&mut deck, add_new(&json!({ "id": "f", "kind": "test-el/liste" }))).unwrap();
    apply(
        &mut deck,
        op(json!({ "op": "edit_list", "target": { "slide": "s-constat", "element": "f" },
        "param": "etapes", "change": { "op": "insert", "id": "i-1" } })),
    )
    .unwrap();
    apply(
        &mut deck,
        op(json!({ "op": "edit_list", "target": { "slide": "s-constat", "element": "f" },
        "param": "etapes", "change": { "op": "set_field", "item": "i-1", "field": "date", "value": "2027" } })),
    )
    .unwrap();
    let f = deck.slide("s-constat").unwrap().locate("f").unwrap().element;
    let l = f.params.list("etapes").unwrap();
    assert_that!(l.iter().map(|i| i.id.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["d0", "i-1"]);
    assert_that!(l[1].values.text("date")).is_equal_to(Some("2027"));
}

#[test]
fn edit_list_on_a_non_list_param_is_param_type() {
    let mut deck = fixture();
    let kind = apply_err(
        &mut deck,
        op(json!({ "op": "edit_list", "target": { "slide": "s-priorites", "element": "c1" },
        "param": "title", "change": { "op": "remove", "item": "x" } })),
    );
    assert_that!(kind).is_equal_to(OpErrorKind::ParamType);
}

#[test]
fn set_param_null_empties_the_list() {
    let mut deck = fixture();
    apply(&mut deck, add_new(&json!({ "id": "f", "kind": "test-el/liste" }))).unwrap();
    apply(&mut deck, set_param(at("s-constat", "f"), "etapes", json!(null))).unwrap();
    let f = deck.slide("s-constat").unwrap().locate("f").unwrap().element;
    assert_that!(f.params.list("etapes").map_or(0, <[_]>::len)).is_equal_to(0);
}

fn edit(element: &str, param: &str, change: &Value) -> Op {
    op(json!({ "op": "edit_list", "target": { "slide": "s-constat", "element": element },
        "param": param, "change": change }))
}

#[test]
fn add_element_without_defaults_keeps_an_absent_list_absent() {
    let mut deck = fixture();
    apply(&mut deck, add(&json!({ "id": "f", "kind": "test-el/liste" }))).unwrap();
    assert_that!(list_of(&deck, "s-constat", "f", "etapes")).is_none();
}

#[test]
fn add_element_with_defaults_seeds_a_nested_slot_element() {
    let mut deck = fixture();
    let stack =
        json!({ "id": "p", "kind": "base/stack", "params": { "children": [{ "id": "f", "kind": "test-el/liste" }] } });
    apply(&mut deck, add_new(&stack)).unwrap();
    assert_that!(list_of(&deck, "s-constat", "f", "etapes")).is_equal_to(Some(vec!["d0".to_owned()]));
}

#[test]
fn set_kind_to_another_kind_seeds_but_to_the_same_kind_does_not() {
    let mut deck = fixture();
    let set_kind = |kind: &str| {
        op(json!({ "op": "set_kind", "target": { "slide": "s-priorites", "element": "c1" },
        "kind": kind }))
    };
    apply(&mut deck, set_kind("test-el/liste")).unwrap();
    assert_that!(list_of(&deck, "s-priorites", "c1", "etapes")).is_equal_to(Some(vec!["d0".to_owned()]));
    apply(&mut deck, set_param(at("s-priorites", "c1"), "etapes", json!(null))).unwrap();
    apply(&mut deck, set_kind("test-el/liste")).unwrap();
    assert_that!(list_of(&deck, "s-priorites", "c1", "etapes")).is_none();
}

#[test]
fn replace_element_does_not_seed() {
    let mut deck = fixture();
    apply(&mut deck, add(&json!({ "id": "f", "kind": "test-el/liste" }))).unwrap();
    let replacement: Element = serde_json::from_value(json!({ "id": "f", "kind": "test-el/liste" })).unwrap();
    apply(&mut deck, Op::ReplaceElement { slide: "s-constat".into(), element: replacement }).unwrap();
    assert_that!(list_of(&deck, "s-constat", "f", "etapes")).is_none();
}

#[test]
fn set_param_with_a_valid_array_converts_the_values() {
    let mut deck = fixture();
    apply(&mut deck, add(&json!({ "id": "f", "kind": "test-el/liste" }))).unwrap();
    let rows = json!([{ "id": "a", "date": " 2020 " }, { "id": "b" }]);
    apply(&mut deck, set_param(at("s-constat", "f"), "etapes", rows)).unwrap();
    let f = deck.slide("s-constat").unwrap().locate("f").unwrap().element;
    let l = f.params.list("etapes").unwrap();
    assert_that!(l.iter().map(|i| i.id.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["a", "b"]);
    assert_that!(l[0].values.text("date")).is_equal_to(Some("2020"));
    assert_that!(l[1].values.text("texte")).is_equal_to(Some("Nouvelle étape"));
}

#[test]
fn set_param_outside_the_bounds_is_refused() {
    let mut deck = fixture();
    apply(&mut deck, add(&json!({ "id": "f", "kind": "test-el/bornes" }))).unwrap();
    let too_many = json!([{ "id": "a" }, { "id": "b" }, { "id": "c" }]);
    let kind = apply_err(&mut deck, set_param(at("s-constat", "f"), "bornee", too_many));
    assert_that!(kind).is_equal_to(OpErrorKind::ListBound);
    let kind = apply_err(&mut deck, set_param(at("s-constat", "f"), "bornee", json!([])));
    assert_that!(kind).is_equal_to(OpErrorKind::ListBound);
}

#[test]
fn edit_list_removing_the_last_item_removes_the_param() {
    let mut deck = fixture();
    apply(&mut deck, add_new(&json!({ "id": "f", "kind": "test-el/liste" }))).unwrap();
    apply(&mut deck, edit("f", "etapes", &json!({ "op": "remove", "item": "d0" }))).unwrap();
    let f = deck.slide("s-constat").unwrap().locate("f").unwrap().element;
    assert_that!(f.params.get("etapes").is_none()).is_true();
}

#[test]
fn edit_list_move_out_of_range_is_refused() {
    let mut deck = fixture();
    apply(&mut deck, add_new(&json!({ "id": "f", "kind": "test-el/liste" }))).unwrap();
    let kind = apply_err(&mut deck, edit("f", "etapes", &json!({ "op": "move", "item": "d0", "to": 5 })));
    assert_that!(kind).is_equal_to(OpErrorKind::IndexOutOfRange);
}

fn liste_with(steps: &Value) -> Value {
    json!({ "id": "f", "kind": "test-el/liste", "params": { "etapes": steps } })
}

fn good_steps() -> Value {
    json!([{ "id": "a", "date": "2019" }, { "id": "b", "texte": "x" }])
}

fn bad_steps() -> Value {
    json!([{ "id": "a", "date": 3 }])
}

fn replace(element: Value) -> Op {
    let element = serde_json::from_value(element).unwrap_or_else(|e| panic!("élément invalide : {e}"));
    Op::ReplaceElement { slide: "s-constat".into(), element }
}

fn add_slide(steps: &Value) -> Op {
    let slide = json!({ "id": "s-liste", "kind": "essentiel/blank", "params": { "elements": [
        { "id": "f", "kind": "test-el/liste", "frame": { "x": 0, "y": 0, "w": 1, "h": 1 }, "params": { "etapes": steps } }
    ] } });
    Op::AddSlide {
        index: None,
        defaults: false,
        slide: serde_json::from_value(slide).unwrap_or_else(|e| panic!("slide invalide : {e}")),
    }
}

#[test]
fn add_element_accepts_a_well_typed_list() {
    let mut deck = fixture();
    apply(&mut deck, add(&liste_with(&good_steps()))).unwrap();
    assert_that!(list_of(&deck, "s-constat", "f", "etapes")).is_equal_to(Some(vec!["a".to_owned(), "b".to_owned()]));
}

#[test]
fn add_element_rejects_a_list_column_of_the_wrong_type() {
    let mut deck = fixture();
    let kind = apply_err(&mut deck, add(&liste_with(&bad_steps())));
    assert_that!(kind).is_equal_to(OpErrorKind::ParamType);
}

#[test]
fn replace_element_accepts_a_well_typed_list() {
    let mut deck = fixture();
    apply(&mut deck, add(&json!({ "id": "f", "kind": "test-el/liste" }))).unwrap();
    apply(&mut deck, replace(liste_with(&good_steps()))).unwrap();
    assert_that!(list_of(&deck, "s-constat", "f", "etapes")).is_equal_to(Some(vec!["a".to_owned(), "b".to_owned()]));
}

#[test]
fn replace_element_rejects_a_list_column_of_the_wrong_type() {
    let mut deck = fixture();
    apply(&mut deck, add(&json!({ "id": "f", "kind": "test-el/liste" }))).unwrap();
    let kind = apply_err(&mut deck, replace(liste_with(&bad_steps())));
    assert_that!(kind).is_equal_to(OpErrorKind::ParamType);
}

#[test]
fn add_slide_accepts_an_element_with_a_well_typed_list() {
    let mut deck = fixture();
    apply(&mut deck, add_slide(&good_steps())).unwrap();
    assert_that!(list_of(&deck, "s-liste", "f", "etapes")).is_equal_to(Some(vec!["a".to_owned(), "b".to_owned()]));
}

#[test]
fn add_slide_rejects_an_element_with_a_list_column_of_the_wrong_type() {
    let mut deck = fixture();
    let kind = apply_err(&mut deck, add_slide(&bad_steps()));
    assert_that!(kind).is_equal_to(OpErrorKind::ParamType);
}

/// Des étapes dont la date dépasse la borne d'une cellule `plain` (4096 octets).
fn long_steps() -> Value {
    json!([{ "id": "a", "date": "x".repeat(4097) }])
}

#[test]
fn set_param_with_a_cell_too_long_is_refused_and_keeps_the_deck() {
    let mut deck = fixture();
    apply(&mut deck, add_new(&json!({ "id": "f", "kind": "test-el/liste" }))).unwrap();
    let before = deck.clone();
    let kind = apply_err(&mut deck, set_param(at("s-constat", "f"), "etapes", long_steps()));
    assert_that!(kind).is_equal_to(OpErrorKind::ListBound);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn edit_list_set_field_too_long_is_refused_and_keeps_the_deck() {
    let mut deck = fixture();
    apply(&mut deck, add_new(&json!({ "id": "f", "kind": "test-el/liste" }))).unwrap();
    let before = deck.clone();
    let change = json!({ "op": "set_field", "item": "d0", "field": "date", "value": "x".repeat(4097) });
    let kind = apply_err(&mut deck, edit("f", "etapes", &change));
    assert_that!(kind).is_equal_to(OpErrorKind::ListBound);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn edit_list_insert_with_an_id_too_long_is_refused_and_keeps_the_deck() {
    let mut deck = fixture();
    apply(&mut deck, add_new(&json!({ "id": "f", "kind": "test-el/liste" }))).unwrap();
    let before = deck.clone();
    let kind = apply_err(&mut deck, edit("f", "etapes", &json!({ "op": "insert", "id": "i".repeat(65) })));
    assert_that!(kind).is_equal_to(OpErrorKind::ListBound);
    assert_that!(deck).is_equal_to(before);
}

#[rstest::rstest]
#[case::add_element(add(&liste_with(&long_steps())))]
#[case::add_slide(add_slide(&long_steps()))]
#[case::too_many_items(
    add(&liste_with(&json!((0..257).map(|i| json!({ "id": format!("i{i}") })).collect::<Vec<_>>())))
)]
fn inserting_a_list_beyond_its_size_bounds_is_refused_and_keeps_the_deck(#[case] op: Op) {
    let mut deck = fixture();
    let before = deck.clone();
    assert_that!(apply_err(&mut deck, op)).is_equal_to(OpErrorKind::ListBound);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn replace_element_with_a_list_beyond_its_size_bounds_is_refused() {
    let mut deck = fixture();
    apply(&mut deck, add(&json!({ "id": "f", "kind": "test-el/liste" }))).unwrap();
    let before = deck.clone();
    assert_that!(apply_err(&mut deck, replace(liste_with(&long_steps())))).is_equal_to(OpErrorKind::ListBound);
    assert_that!(deck).is_equal_to(before);
}

fn bounded_deck() -> Deck {
    let mut deck = fixture();
    apply(&mut deck, add_new(&json!({ "id": "f", "kind": "test-el/liste" }))).unwrap_or_else(|e| panic!("{e}"));
    let rows = json!([{ "id": "a" }, { "id": "b" }]);
    let element = json!({ "id": "b", "kind": "test-el/bornes", "params": { "bornee": rows } });
    apply(&mut deck, add(&element)).unwrap_or_else(|e| panic!("{e}"));
    let one = json!({ "id": "m", "kind": "test-el/bornes", "params": { "bornee": [{ "id": "a" }] } });
    apply(&mut deck, add(&one)).unwrap_or_else(|e| panic!("{e}"));
    deck
}

#[rstest::rstest]
#[case::full(
    edit("b", "bornee", &json!({ "op": "insert", "id": "c" })),
    "La liste bornee est pleine (2 items au plus)",
    "The list bornee is full (2 items at most)",
    ListBound::Full { max: 2 }
)]
#[case::minimum(
    edit("m", "bornee", &json!({ "op": "remove", "item": "a" })),
    "La liste bornee doit garder au moins 1 item",
    "The list bornee needs at least 1 item",
    ListBound::Minimum { min: 1 }
)]
#[case::minimum_by_set_param(
    set_param(at("s-constat", "b"), "bornee", json!([])),
    "La liste bornee doit garder au moins 1 item",
    "The list bornee needs at least 1 item",
    ListBound::Minimum { min: 1 }
)]
#[case::cell_too_long(
    edit("f", "etapes", &json!({ "op": "set_field", "item": "d0", "field": "date", "value": "x".repeat(4097) })),
    "Une cellule de la liste etapes est trop longue (colonne date)",
    "A cell of the list etapes is too long (column date)",
    ListBound::CellTooLong { column: "date".to_owned() }
)]
#[case::id_too_long(
    edit("f", "etapes", &json!({ "op": "insert", "id": "i".repeat(65) })),
    "Un identifiant d'item de la liste etapes est trop long",
    "An item id of the list etapes is too long",
    ListBound::IdTooLong
)]
#[case::too_many_items(
    set_param(at("s-constat", "f"), "etapes", json!((0..257).map(|i| json!({ "id": format!("i{i}") })).collect::<Vec<_>>())),
    "La liste etapes a trop d'items (256 au plus)",
    "The list etapes has too many items (256 at most)",
    ListBound::TooManyItems { max: 256 }
)]
fn list_bound_refusals_say_which_bound_in_french_and_english(
    #[case] op: Op,
    #[case] fr: &str,
    #[case] en: &str,
    #[case] bound: ListBound,
) {
    use oku_slides_i18n::{Localize, Localizer};
    let mut deck = bounded_deck();
    let Err(err) = apply(&mut deck, op) else { panic!("l'op devait être refusée") };
    let err = err.op_error();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ListBound);
    assert_that!(err.list_bound()).is_equal_to(Some(&bound));
    // Les espaces insécables que Fluent isole autour des arguments sont retirés.
    let say = |l: &str| err.localize(&Localizer::new(Some(l))).replace(['\u{2068}', '\u{2069}'], "");
    assert_that!(say("fr")).is_equal_to(fr.to_owned());
    assert_that!(say("en")).is_equal_to(en.to_owned());
}

#[test]
fn list_bound_of_an_error_other_than_a_list_bound_is_none() {
    let mut deck = bounded_deck();
    let Err(err) = apply(&mut deck, edit("f", "etapes", &json!({ "op": "remove", "item": "absent" }))) else {
        panic!("l'op devait être refusée")
    };
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::ItemNotFound);
    assert_that!(err.op_error().list_bound()).is_none();
}

// Listes imbriquées (t-495) : `edit_list` adresse la liste d'un item par `param/item/colonne`.

fn edit_plan(param: &str, change: &Value) -> Op {
    op(
        json!({ "op": "edit_list", "target": { "slide": "s-constat", "element": "p" }, "param": param, "change": change }),
    )
}

fn tasks_of(deck: &Deck, step: &str) -> Vec<(String, Option<String>)> {
    let Some(p) = deck.slide("s-constat").and_then(|s| s.locate("p")).map(|l| l.element) else { panic!("plan absent") };
    let steps = p.params.list("etapes").unwrap_or_default();
    let it = steps.iter().find(|i| i.id == step).unwrap_or_else(|| panic!("étape {step} absente"));
    it.values
        .list("taches")
        .unwrap_or_default()
        .iter()
        .map(|t| (t.id.clone(), t.values.text("texte").map(str::to_owned)))
        .collect()
}

#[test]
fn add_element_with_defaults_seeds_nested_list_defaults() {
    let mut deck = fixture();
    apply(&mut deck, add_new(&json!({ "id": "p", "kind": "test-el/plan" }))).unwrap();
    assert_that!(tasks_of(&deck, "d0")).is_equal_to(vec![("d0".to_owned(), Some("Cadrer".to_owned()))]);
}

#[test]
fn edit_list_nested_path_inserts_moves_sets_and_removes_a_nested_item() {
    let mut deck = fixture();
    apply(&mut deck, add_new(&json!({ "id": "p", "kind": "test-el/plan" }))).unwrap();
    apply(&mut deck, edit_plan("etapes/d0/taches", &json!({ "op": "insert", "index": 0, "id": "n" }))).unwrap();
    apply(
        &mut deck,
        edit_plan("etapes/d0/taches", &json!({ "op": "set_field", "item": "n", "field": "texte", "value": "Neuf" })),
    )
    .unwrap();
    apply(&mut deck, edit_plan("etapes/d0/taches", &json!({ "op": "move", "item": "n", "to": 1 }))).unwrap();
    assert_that!(tasks_of(&deck, "d0"))
        .is_equal_to(vec![("d0".to_owned(), Some("Cadrer".to_owned())), ("n".to_owned(), Some("Neuf".to_owned()))]);
    apply(&mut deck, edit_plan("etapes/d0/taches", &json!({ "op": "remove", "item": "d0" }))).unwrap();
    assert_that!(tasks_of(&deck, "d0")).is_equal_to(vec![("n".to_owned(), Some("Neuf".to_owned()))]);
}

#[test]
fn edit_list_inserting_a_step_seeds_its_nested_default() {
    let mut deck = fixture();
    apply(&mut deck, add_new(&json!({ "id": "p", "kind": "test-el/plan" }))).unwrap();
    apply(&mut deck, edit_plan("etapes", &json!({ "op": "insert", "id": "e2" }))).unwrap();
    assert_that!(tasks_of(&deck, "e2")).is_equal_to(vec![("d0".to_owned(), Some("Première".to_owned()))]);
}

#[rstest::rstest]
#[case("etapes/zz/taches", OpErrorKind::ItemNotFound)]
#[case("etapes/d0/titre", OpErrorKind::ParamType)]
#[case("etapes/d0", OpErrorKind::UnknownParam)]
#[case("inconnue/d0/taches", OpErrorKind::UnknownParam)]
fn edit_list_bad_nested_path_is_refused(#[case] param: &str, #[case] kind: OpErrorKind) {
    let mut deck = fixture();
    apply(&mut deck, add_new(&json!({ "id": "p", "kind": "test-el/plan" }))).unwrap();
    assert_that!(apply_err(&mut deck, edit_plan(param, &json!({ "op": "insert", "id": "n" })))).is_equal_to(kind);
}

#[test]
fn edit_list_nested_insert_beyond_its_max_is_refused() {
    let mut deck = fixture();
    apply(&mut deck, add_new(&json!({ "id": "p", "kind": "test-el/plan" }))).unwrap();
    for id in ["a", "b"] {
        apply(&mut deck, edit_plan("etapes/d0/taches", &json!({ "op": "insert", "id": id }))).unwrap();
    }
    let kind = apply_err(&mut deck, edit_plan("etapes/d0/taches", &json!({ "op": "insert", "id": "c" })));
    assert_that!(kind).is_equal_to(OpErrorKind::ListBound);
}

#[rstest::rstest]
#[case::top(json!([{ "id": "a/b" }]))]
#[case::nested(json!([{ "id": "a", "taches": [{ "id": "x/y" }] }]))]
fn add_element_with_a_slash_in_an_item_id_is_refused(#[case] steps: Value) {
    let mut deck = fixture();
    let kind = apply_err(&mut deck, add(&json!({ "id": "p", "kind": "test-el/plan", "params": { "etapes": steps } })));
    assert_that!(kind).is_equal_to(OpErrorKind::InvalidValue);
}
