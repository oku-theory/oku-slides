//! t-746 : A fait passer un élément d'une slide à une autre pendant que B le
//! met en gras ; après fusion, le gras de B était perdu (retrait puis copie
//! dans l'autre slide). Le deck est désormais un seul `LoroTree` : le passage
//! d'une slide à l'autre est un `mov`, l'édition concurrente reste.

use oku_slides_core::kinds::essentiel::CONTENT;
use oku_slides_core::{Deck, DeckReplica, Element, Op, ParamValue, Slide, apply_all, apply_all_moving};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common;

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("fixture : {e}"))
}

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
}

fn with_ops(deck: &Deck, ops: Vec<Op>) -> Deck {
    let mut d = deck.clone();
    apply_all(&mut d, ops, &common::registry()).unwrap_or_else(|e| panic!("ops refusées : {e:?}"));
    d
}

/// Les éléments d'id `id` de la slide `slide`.
fn found(deck: &Deck, slide: &str, id: &str) -> Vec<Element> {
    let s = deck.slide(slide).unwrap_or_else(|| panic!("{slide} absente"));
    s.walk().filter(|l| l.element.id == id).map(|l| l.element.clone()).collect()
}

/// Les ids de `blocks` de la slide `slide`.
fn blocks(deck: &Deck, slide: &str) -> Vec<String> {
    let s = deck.slide(slide).unwrap_or_else(|| panic!("{slide} absente"));
    s.params.slot("blocks").unwrap_or_default().iter().map(|e| e.id.clone()).collect()
}

/// Le premier run du `body` de `e` est-il gras ?
fn first_run_bold(e: &Element) -> bool {
    match e.params.get("body") {
        Some(ParamValue::Rich(body)) => body.first().and_then(|p| p.runs.first()).is_some_and(|r| r.bold),
        _ => false,
    }
}

#[test]
fn move_between_slides_keeps_the_concurrent_bold() {
    let deck = roadmap();
    let registry = common::shared();
    let snap = DeckReplica::new(&deck, registry.clone())
        .unwrap_or_else(|e| panic!("replica : {e}"))
        .snapshot()
        .unwrap_or_else(|e| panic!("snapshot : {e}"));
    let open = || DeckReplica::from_snapshot(&snap, 1 << 24, registry.clone()).unwrap_or_else(|e| panic!("{e}")).0;
    let (mut a, mut b) = (open(), open());

    // A : `b-points` quitte `s-constat` pour la tête de `blocks` de `s-priorites`.
    let Some(original) = found(&deck, "s-constat", "b-points").pop() else { panic!("b-points absent") };
    assert_that!(first_run_bold(&original)).is_false();
    let remove = op(json!({ "op": "remove_element", "slide": "s-constat", "element": "b-points" }));
    let add = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-priorites" }, "param": "blocks" },
        "index": 0, "element": original.clone() }));
    a.apply(&with_ops(&deck, vec![remove, add]), "A", None).unwrap_or_else(|e| panic!("apply : {e}"));

    // B : met en gras le premier run du `body` de `b-points`, visé par id.
    let Some(ParamValue::Rich(mut body)) = original.params.get("body").cloned() else { panic!("body absent") };
    body[0].runs[0].bold = true;
    let bold = op(json!({ "op": "set_param", "target": { "slide": "s-constat", "element": "b-points" },
        "param": "body", "value": body }));
    let bold = with_ops(&deck, vec![bold]);
    assert_that!(found(&bold, "s-constat", "b-points").iter().all(first_run_bold)).is_true();
    b.apply(&bold, "B", None).unwrap_or_else(|e| panic!("apply : {e}"));

    a.merge(&b).unwrap_or_else(|e| panic!("merge : {e}"));
    b.merge(&a).unwrap_or_else(|e| panic!("merge : {e}"));
    let (merged, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(b.deck()).is_equal_to((merged.clone(), Vec::new()));
    assert_that!(blocks(&merged, "s-priorites"))
        .is_equal_to(["b-points", "c1", "c2", "c3"].map(str::to_owned).to_vec());
    assert_that!(found(&merged, "s-constat", "b-points")).is_empty();
    let moved = found(&merged, "s-priorites", "b-points");
    assert_that!(moved.len()).is_equal_to(1);
    assert_that!(first_run_bold(&moved[0])).is_true();
}

/// Le même scénario, avec un id présent dans trois slides : seul
/// `move_element` (déplacement annoncé) garde le nœud et le gras de B (t-777).
#[test]
fn move_element_with_a_repeated_id_keeps_the_concurrent_bold() {
    let dup = |slide: &str| {
        op(json!({ "op": "add_element", "slot": { "owner": { "slide": slide }, "param": "blocks" },
            "element": { "id": "dup", "kind": "base/text", "params": { "body": [{ "runs": [{ "t": slide }] }] } } }))
    };
    let mut deck = with_ops(&roadmap(), vec![dup("s-constat"), dup("s-priorites"), dup("s-libre")]);
    deck.slides.push(Slide::new("s-neuve", CONTENT));
    let registry = common::shared();
    let snap = DeckReplica::new(&deck, registry.clone())
        .unwrap_or_else(|e| panic!("replica : {e}"))
        .snapshot()
        .unwrap_or_else(|e| panic!("snapshot : {e}"));
    let open = || DeckReplica::from_snapshot(&snap, 1 << 24, registry.clone()).unwrap_or_else(|e| panic!("{e}")).0;
    let (mut a, mut b) = (open(), open());

    // A : `dup` quitte `s-constat` pour `s-neuve`, par `move_element`.
    let mut moved = deck.clone();
    let mv = op(json!({ "op": "move_element", "slide": "s-constat", "element": "dup",
        "to": { "owner": { "slide": "s-neuve" }, "param": "blocks" } }));
    let moves = apply_all_moving(&mut moved, vec![mv], &common::registry()).unwrap_or_else(|e| panic!("{e:?}"));
    a.apply_moving(&moved, "A", None, &moves).unwrap_or_else(|e| panic!("apply : {e}"));

    // B : met en gras le `dup` de `s-constat`.
    let Some(original) = found(&deck, "s-constat", "dup").pop() else { panic!("dup absent") };
    let Some(ParamValue::Rich(mut body)) = original.params.get("body").cloned() else { panic!("body absent") };
    body[0].runs[0].bold = true;
    let bold = op(json!({ "op": "set_param", "target": { "slide": "s-constat", "element": "dup" },
        "param": "body", "value": body }));
    b.apply(&with_ops(&deck, vec![bold]), "B", None).unwrap_or_else(|e| panic!("apply : {e}"));

    a.merge(&b).unwrap_or_else(|e| panic!("merge : {e}"));
    b.merge(&a).unwrap_or_else(|e| panic!("merge : {e}"));
    let (merged, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(b.deck()).is_equal_to((merged.clone(), Vec::new()));
    assert_that!(found(&merged, "s-constat", "dup")).is_empty();
    let moved = found(&merged, "s-neuve", "dup");
    assert_that!(moved.len()).is_equal_to(1);
    assert_that!(first_run_bold(&moved[0])).is_true();
    for other in ["s-priorites", "s-libre"] {
        assert_that!(found(&merged, other, "dup").iter().any(first_run_bold)).named(other).is_false();
    }
}
