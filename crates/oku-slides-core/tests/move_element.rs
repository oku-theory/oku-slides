//! L'op `move_element` : un élément change de slot, de slide ou de place (t-777).

mod common;

use common::registry;
use oku_slides_core::*;
use oku_slides_i18n::{Localize, Localizer};
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

fn deck_of(v: Value) -> Deck {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("deck invalide : {e}"))
}

fn text(id: &str) -> Value {
    json!({ "id": id, "kind": "text", "params": { "body": [{ "runs": [{ "t": id }] }] } })
}

/// `move_element` de `element` (slide `slide`) vers le slot `param` de `owner` (`None` : la slide `to`).
fn mv(slide: &str, element: &str, to: &str, owner: Option<&str>, param: &str, index: Option<usize>) -> Op {
    let owner = match owner {
        Some(e) => json!({ "slide": to, "element": e }),
        None => json!({ "slide": to }),
    };
    op(json!({ "op": "move_element", "slide": slide, "element": element,
        "to": { "owner": owner, "param": param }, "index": index }))
}

fn add_to(slide: &str, param: &str, element: Value) -> Op {
    op(json!({ "op": "add_element", "slot": { "owner": { "slide": slide }, "param": param }, "element": element }))
}

fn with_ops(deck: &Deck, ops: Vec<Op>) -> Deck {
    let mut d = deck.clone();
    apply_all(&mut d, ops, &registry()).unwrap_or_else(|e| panic!("ops refusées : {e:?}"));
    d
}

/// La fixture, avec une pile `pile` (enfants `k1`, `k2`) à la fin de `blocks` de `s-constat`.
fn with_pile() -> Deck {
    let pile = json!({ "id": "pile", "kind": "stack", "params": { "children": [text("k1"), text("k2")] } });
    with_ops(&fixture(), vec![add_to("s-constat", "blocks", pile)])
}

/// Les ids du slot `param` de la slide `slide`.
fn ids(deck: &Deck, slide: &str, param: &str) -> Vec<String> {
    let s = deck.slide(slide).unwrap_or_else(|| panic!("{slide} absente"));
    s.params.slot(param).unwrap_or_default().iter().map(|e| e.id.clone()).collect()
}

fn owned(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

/// Applique `ops` avec `registry` ; le refus doit laisser le deck intact et
/// porter un message localisé.
fn refused(deck: &Deck, ops: Vec<Op>, registry: &WidgetRegistry) -> OpErrorKind {
    let mut d = deck.clone();
    let Err(err) = apply_all_moving(&mut d, ops, registry) else { panic!("les ops devaient être refusées") };
    assert_that!(d).named("deck après refus").is_equal_to(deck.clone());
    let err = err.op_error();
    let fr = err.localize(&Localizer::new(Some("fr")));
    assert_that!(fr.is_empty()).named("message localisé vide").is_false();
    assert_that!(fr).named("message localisé").is_not_equal_to(err.to_string());
    err.kind()
}

#[test]
fn move_element_between_slides_keeps_the_id_and_the_subtree() {
    let deck = with_pile();
    let moved = with_ops(&deck, vec![mv("s-constat", "pile", "s-priorites", None, "blocks", Some(0))]);
    assert_that!(ids(&moved, "s-priorites", "blocks")).is_equal_to(owned(&["pile", "c1", "c2", "c3"]));
    let pile = moved.slide("s-priorites").and_then(|s| s.locate("pile")).map(|l| l.element.clone());
    let children: Vec<String> = pile
        .as_ref()
        .and_then(|p| p.params.slot("children"))
        .unwrap_or_default()
        .iter()
        .map(|e| e.id.clone())
        .collect();
    assert_that!(children).is_equal_to(owned(&["k1", "k2"]));
    let constat = moved.slide("s-constat").unwrap_or_else(|| panic!("s-constat absente"));
    for id in ["pile", "k1", "k2"] {
        assert_that!(constat.locate(id).is_none()).named(id).is_true();
    }
    assert_that!(ids(&moved, "s-constat", "blocks")).is_equal_to(owned(&["b-points"]));
}

#[test]
fn move_element_within_a_slide_reorders() {
    let moved = with_ops(&fixture(), vec![mv("s-priorites", "c3", "s-priorites", None, "blocks", Some(0))]);
    assert_that!(ids(&moved, "s-priorites", "blocks")).is_equal_to(owned(&["c3", "c1", "c2"]));
}

#[test]
fn move_element_into_a_flow_slot_drops_frame_and_cell() {
    let mut deck = fixture();
    let mut json = serde_json::to_value(&deck).unwrap_or_else(|e| panic!("{e}"));
    let Some(slides) = json["slides"].as_array_mut() else { panic!("slides attendues") };
    let Some(libre) = slides.iter_mut().find(|s| s["id"] == "s-libre") else { panic!("s-libre absente") };
    let Some(elements) = libre["params"]["elements"].as_array_mut() else { panic!("elements attendus") };
    let Some(texte) = elements.iter_mut().find(|e| e["id"] == "e-texte") else { panic!("e-texte absent") };
    texte["cell"] = json!({ "col": 1, "row": 1 });
    deck = deck_of(json);
    let before = deck.slide("s-libre").and_then(|s| s.locate("e-texte")).map(|l| l.element.clone());
    assert_that!(before.as_ref().is_some_and(|e| e.frame.is_some() && e.cell.is_some())).is_true();
    let moved = with_ops(&deck, vec![mv("s-libre", "e-texte", "s-constat", None, "blocks", None)]);
    let after = moved.slide("s-constat").and_then(|s| s.locate("e-texte")).map(|l| l.element.clone());
    let Some(after) = after else { panic!("e-texte absent de s-constat") };
    assert_that!(after.frame).is_none();
    assert_that!(after.cell).is_none();
    assert_that!(ids(&moved, "s-constat", "blocks")).is_equal_to(owned(&["b-points", "e-texte"]));
}

#[test]
fn move_element_into_its_own_subtree_is_refused() {
    let deck = with_pile();
    let kind = refused(&deck, vec![mv("s-constat", "pile", "s-constat", Some("pile"), "children", None)], &registry());
    assert_that!(kind).is_equal_to(OpErrorKind::ElementNotFound);
}

#[test]
fn move_element_onto_a_taken_id_is_refused() {
    let deck = with_ops(&fixture(), vec![add_to("s-constat", "blocks", text("c1"))]);
    let kind = refused(&deck, vec![mv("s-priorites", "c1", "s-constat", None, "blocks", None)], &registry());
    assert_that!(kind).is_equal_to(OpErrorKind::DuplicateId);
}

/// Slide `free` (slot `items` libre de tout filtre), élément `box` dont le
/// slot `children` n'accepte que `text`.
fn accepts_registry() -> WidgetRegistry {
    WidgetRegistry::from_probe(json!({ "base": [
        { "name": "free", "category": "slide",
          "params": [{ "name": "items", "type": "slot", "mode": "flow" }] },
        { "name": "box", "category": "element",
          "params": [{ "name": "children", "type": "slot", "mode": "flow", "accepts": ["text"] }] },
        { "name": "text", "category": "element", "params": [] },
        { "name": "shape", "category": "element", "params": [] }
    ], "pack": null }))
    .unwrap_or_else(|e| panic!("registre de test : {e}"))
}

#[test]
fn move_element_to_an_unknown_slot_is_refused() {
    let deck = with_pile();
    let kind = refused(&deck, vec![mv("s-constat", "pile", "s-priorites", None, "zone", None)], &registry());
    assert_that!(kind).is_equal_to(OpErrorKind::UnknownParam);

    let deck = deck_of(json!({ "slides": [{ "id": "s", "kind": "free", "params": { "items": [
        { "id": "b", "kind": "box" }, { "id": "f", "kind": "shape" }
    ] } }] }));
    let kind = refused(&deck, vec![mv("s", "f", "s", Some("b"), "children", None)], &accepts_registry());
    assert_that!(kind).is_equal_to(OpErrorKind::SlotRejects);
}

#[test]
fn apply_all_moving_composes_moves_of_one_element() {
    let run = |ops: Vec<Op>| {
        let mut d = fixture();
        apply_all_moving(&mut d, ops, &registry()).unwrap_or_else(|e| panic!("ops refusées : {e:?}"))
    };
    let a_b = || mv("s-constat", "b-points", "s-priorites", None, "blocks", None);
    let a_c = run(vec![a_b(), mv("s-priorites", "b-points", "s-libre", None, "blocks", None)]);
    let expected = Moved { element: "b-points".into(), from: "s-constat".into(), to: "s-libre".into() };
    assert_that!(a_c).is_equal_to(vec![expected]);

    let back = run(vec![a_b(), mv("s-priorites", "b-points", "s-constat", None, "blocks", None)]);
    assert_that!(back).is_empty();

    let remove = op(json!({ "op": "remove_element", "slide": "s-priorites", "element": "b-points" }));
    assert_that!(run(vec![a_b(), remove])).is_empty();

    let within = run(vec![mv("s-priorites", "c3", "s-priorites", None, "blocks", Some(0))]);
    assert_that!(within).is_empty();
}

#[test]
fn move_element_from_or_to_an_unknown_slide_is_refused() {
    let deck = fixture();
    let kind = refused(&deck, vec![mv("s-absente", "b-points", "s-priorites", None, "blocks", None)], &registry());
    assert_that!(kind).is_equal_to(OpErrorKind::SlideNotFound);
    let kind = refused(&deck, vec![mv("s-constat", "b-points", "s-absente", None, "blocks", None)], &registry());
    assert_that!(kind).is_equal_to(OpErrorKind::SlideNotFound);
}

/// Une chaîne de `n` piles imbriquées, d'ids `{prefix}0` (en haut) à `{prefix}{n-1}`.
fn pile_chain(prefix: &str, n: usize) -> Value {
    let mut tree = json!({ "id": format!("{prefix}{}", n - 1), "kind": "stack" });
    for i in (0..n - 1).rev() {
        tree = json!({ "id": format!("{prefix}{i}"), "kind": "stack", "params": { "children": [tree] } });
    }
    tree
}

#[test]
fn move_element_too_deep_is_refused() {
    // `h1` est déjà imbriqué (dans `h0`) ; la chaîne a MAX_SLOT_DEPTH - 1 piles.
    let deck = deck_of(json!({ "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
        pile_chain("c", MAX_SLOT_DEPTH - 1), pile_chain("h", 2)
    ] } }] }));
    let kind = refused(&deck, vec![mv("s", "c0", "s", Some("h1"), "children", None)], &registry());
    assert_that!(kind).is_equal_to(OpErrorKind::InvalidValue);
    // Un cran plus haut, sous `h0`, la chaîne tient tout juste.
    let moved = with_ops(&deck, vec![mv("s", "c0", "s", Some("h0"), "children", None)]);
    assert_that!(ids(&moved, "s", "blocks")).is_equal_to(owned(&["h0"]));
}

#[test]
fn move_element_to_an_index_out_of_range_is_refused() {
    let deck = fixture();
    let kind = refused(&deck, vec![mv("s-priorites", "c1", "s-constat", None, "blocks", Some(2))], &registry());
    assert_that!(kind).is_equal_to(OpErrorKind::IndexOutOfRange);
    // Dans la même slide, la longueur est celle du slot sans l'élément.
    let kind = refused(&deck, vec![mv("s-priorites", "c1", "s-priorites", None, "blocks", Some(3))], &registry());
    assert_that!(kind).is_equal_to(OpErrorKind::IndexOutOfRange);
}

#[test]
fn ops_schema_lists_move_element() {
    let schema = ops_schema().unwrap_or_else(|e| panic!("schéma : {e}")).to_string();
    assert!(schema.contains("\"move_element\""), "move_element absent du schéma des ops");
}
