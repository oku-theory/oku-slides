use oku_slides_core::*;
use serde_json::{Value, json};
use speculoos::prelude::*;

fn fixture() -> Deck {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("fixture {path} illisible : {e}"));
    Deck::from_json(&json).unwrap_or_else(|e| panic!("fixture {path} invalide : {e}"))
}

/// Le registre de base, plus un widget d'élément `liste` à une liste `etapes`.
fn list_registry() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    let mut probe: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} : {e}"));
    let liste = json!({
        "name": "liste",
        "category": "element",
        "params": [{
            "name": "etapes",
            "type": "list",
            "item": [
                { "name": "date", "type": "plain" },
                { "name": "texte", "type": "plain", "default": "Nouvelle étape" }
            ],
            "default": [{ "date": "2019" }]
        }, {
            "name": "bornee",
            "type": "list",
            "min": 1,
            "max": 2,
            "item": [{ "name": "t", "type": "plain" }]
        }]
    });
    probe["base"].as_array_mut().unwrap_or_else(|| panic!("sonde : base attendu")).push(liste);
    let probe = serde_json::from_value(probe).unwrap_or_else(|e| panic!("sonde : {e}"));
    WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("sonde : {e}"))
}

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
}

fn add(element: Value) -> Op {
    op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": element }))
}

/// Comme [`add`], pour un objet nouveau : les listes sont amorcées.
fn add_new(element: Value) -> Op {
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
    apply(&mut deck, add_new(json!({ "id": "f", "kind": "liste" }))).unwrap();
    assert_that!(list_of(&deck, "s-constat", "f", "etapes")).is_equal_to(Some(vec!["d0".to_owned()]));
}

#[test]
fn edit_list_insert_then_set_field_edits_the_deck() {
    let mut deck = fixture();
    apply(&mut deck, add_new(json!({ "id": "f", "kind": "liste" }))).unwrap();
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
    apply(&mut deck, add_new(json!({ "id": "f", "kind": "liste" }))).unwrap();
    apply(&mut deck, set_param(at("s-constat", "f"), "etapes", json!(null))).unwrap();
    let f = deck.slide("s-constat").unwrap().locate("f").unwrap().element;
    assert_that!(f.params.list("etapes").map(<[_]>::len).unwrap_or(0)).is_equal_to(0);
}

fn edit(element: &str, param: &str, change: Value) -> Op {
    op(json!({ "op": "edit_list", "target": { "slide": "s-constat", "element": element },
        "param": param, "change": change }))
}

#[test]
fn add_element_without_defaults_keeps_an_absent_list_absent() {
    let mut deck = fixture();
    apply(&mut deck, add(json!({ "id": "f", "kind": "liste" }))).unwrap();
    assert_that!(list_of(&deck, "s-constat", "f", "etapes")).is_none();
}

#[test]
fn add_element_with_defaults_seeds_a_nested_slot_element() {
    let mut deck = fixture();
    let stack = json!({ "id": "p", "kind": "stack", "params": { "children": [{ "id": "f", "kind": "liste" }] } });
    apply(&mut deck, add_new(stack)).unwrap();
    assert_that!(list_of(&deck, "s-constat", "f", "etapes")).is_equal_to(Some(vec!["d0".to_owned()]));
}

#[test]
fn set_kind_to_another_kind_seeds_but_to_the_same_kind_does_not() {
    let mut deck = fixture();
    let set_kind = |kind: &str| {
        op(json!({ "op": "set_kind", "target": { "slide": "s-priorites", "element": "c1" },
        "kind": kind }))
    };
    apply(&mut deck, set_kind("liste")).unwrap();
    assert_that!(list_of(&deck, "s-priorites", "c1", "etapes")).is_equal_to(Some(vec!["d0".to_owned()]));
    apply(&mut deck, set_param(at("s-priorites", "c1"), "etapes", json!(null))).unwrap();
    apply(&mut deck, set_kind("liste")).unwrap();
    assert_that!(list_of(&deck, "s-priorites", "c1", "etapes")).is_none();
}

#[test]
fn replace_element_does_not_seed() {
    let mut deck = fixture();
    apply(&mut deck, add(json!({ "id": "f", "kind": "liste" }))).unwrap();
    let replacement: Element = serde_json::from_value(json!({ "id": "f", "kind": "liste" })).unwrap();
    apply(&mut deck, Op::ReplaceElement { slide: "s-constat".into(), element: replacement }).unwrap();
    assert_that!(list_of(&deck, "s-constat", "f", "etapes")).is_none();
}

#[test]
fn set_param_with_a_valid_array_converts_the_values() {
    let mut deck = fixture();
    apply(&mut deck, add(json!({ "id": "f", "kind": "liste" }))).unwrap();
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
    apply(&mut deck, add(json!({ "id": "f", "kind": "liste" }))).unwrap();
    let too_many = json!([{ "id": "a" }, { "id": "b" }, { "id": "c" }]);
    let kind = apply_err(&mut deck, set_param(at("s-constat", "f"), "bornee", too_many));
    assert_that!(kind).is_equal_to(OpErrorKind::InvalidValue);
    let kind = apply_err(&mut deck, set_param(at("s-constat", "f"), "bornee", json!([])));
    assert_that!(kind).is_equal_to(OpErrorKind::InvalidValue);
}

#[test]
fn edit_list_removing_the_last_item_removes_the_param() {
    let mut deck = fixture();
    apply(&mut deck, add_new(json!({ "id": "f", "kind": "liste" }))).unwrap();
    apply(&mut deck, edit("f", "etapes", json!({ "op": "remove", "item": "d0" }))).unwrap();
    let f = deck.slide("s-constat").unwrap().locate("f").unwrap().element;
    assert_that!(f.params.get("etapes").is_none()).is_true();
}

#[test]
fn edit_list_move_out_of_range_is_refused() {
    let mut deck = fixture();
    apply(&mut deck, add_new(json!({ "id": "f", "kind": "liste" }))).unwrap();
    let kind = apply_err(&mut deck, edit("f", "etapes", json!({ "op": "move", "item": "d0", "to": 5 })));
    assert_that!(kind).is_equal_to(OpErrorKind::IndexOutOfRange);
}

fn liste_with(steps: Value) -> Value {
    json!({ "id": "f", "kind": "liste", "params": { "etapes": steps } })
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

fn add_slide(steps: Value) -> Op {
    let slide = json!({ "id": "s-liste", "kind": "blank", "params": { "elements": [
        { "id": "f", "kind": "liste", "frame": { "x": 0, "y": 0, "w": 1, "h": 1 }, "params": { "etapes": steps } }
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
    apply(&mut deck, add(liste_with(good_steps()))).unwrap();
    assert_that!(list_of(&deck, "s-constat", "f", "etapes")).is_equal_to(Some(vec!["a".to_owned(), "b".to_owned()]));
}

#[test]
fn add_element_rejects_a_list_column_of_the_wrong_type() {
    let mut deck = fixture();
    let kind = apply_err(&mut deck, add(liste_with(bad_steps())));
    assert_that!(kind).is_equal_to(OpErrorKind::ParamType);
}

#[test]
fn replace_element_accepts_a_well_typed_list() {
    let mut deck = fixture();
    apply(&mut deck, add(json!({ "id": "f", "kind": "liste" }))).unwrap();
    apply(&mut deck, replace(liste_with(good_steps()))).unwrap();
    assert_that!(list_of(&deck, "s-constat", "f", "etapes")).is_equal_to(Some(vec!["a".to_owned(), "b".to_owned()]));
}

#[test]
fn replace_element_rejects_a_list_column_of_the_wrong_type() {
    let mut deck = fixture();
    apply(&mut deck, add(json!({ "id": "f", "kind": "liste" }))).unwrap();
    let kind = apply_err(&mut deck, replace(liste_with(bad_steps())));
    assert_that!(kind).is_equal_to(OpErrorKind::ParamType);
}

#[test]
fn add_slide_accepts_an_element_with_a_well_typed_list() {
    let mut deck = fixture();
    apply(&mut deck, add_slide(good_steps())).unwrap();
    assert_that!(list_of(&deck, "s-liste", "f", "etapes")).is_equal_to(Some(vec!["a".to_owned(), "b".to_owned()]));
}

#[test]
fn add_slide_rejects_an_element_with_a_list_column_of_the_wrong_type() {
    let mut deck = fixture();
    let kind = apply_err(&mut deck, add_slide(bad_steps()));
    assert_that!(kind).is_equal_to(OpErrorKind::ParamType);
}
