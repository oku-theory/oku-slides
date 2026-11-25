//! A removed data against a peer's concurrent edits: merge, then undo.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::sync::Arc;

use oku_slides_core::{Deck, DeckReplica, Op, PlanOutcome, apply_all, plan_views};
use oku_slides_data::{DataCell, DataId};
use oku_slides_i18n::Message;
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("invalid op: {e}"))
}

fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn with_ops(deck: &Deck, ops: Vec<Op>) -> Deck {
    let mut next = deck.clone();
    ok(apply_all(&mut next, ops, &common::registry()));
    next
}

/// A table element `id` in the blocks of slide `s`, bound to `data`.
fn table(id: &str, data: &str) -> Op {
    op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s" }, "param": "blocks" },
        "element": { "id": id, "kind": "base/table", "params": { "data": { "data": data } } } }))
}

/// `common::data_deck` (data `d`, no view) and its sibling `e`, shown by the table `t`.
fn deck() -> Deck {
    let add = op(json!({ "op": "add_data", "id": "e", "name": "costs", "table": { "generation": "g2",
        "columns": [{ "id": "k1", "name": "Cost" }], "rows": [{ "id": "q1", "cells": [3] }] } }));
    with_ops(&common::data_deck(), vec![add, table("t", "e")])
}

/// Two peers from one snapshot of `deck`.
fn peers(deck: &Deck) -> (DeckReplica, DeckReplica) {
    let registry = Arc::new(common::registry());
    let snap = ok(ok(DeckReplica::new(deck, registry.clone())).snapshot());
    let open = || ok(DeckReplica::from_snapshot(&snap, 1 << 24, registry.clone())).0;
    (open(), open())
}

fn remove_d() -> Op {
    op(json!({ "op": "remove_data", "data": "d" }))
}

/// A removes `d` while B applies `ops_b`; both merge, `a_first` deciding who merges first.
fn concurrent(ops_b: Vec<Op>, a_first: bool) -> (Deck, DeckReplica, DeckReplica) {
    let deck = deck();
    let (mut a, mut b) = peers(&deck);
    ok(a.apply(&with_ops(&deck, vec![remove_d()]), "A", None));
    ok(b.apply(&with_ops(&deck, ops_b), "B", None));
    if a_first {
        ok(a.merge(&b));
        ok(b.merge(&a));
    } else {
        ok(b.merge(&a));
        ok(a.merge(&b));
    }
    let (merged, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(b.deck().0).is_equal_to(&merged);
    (merged, a, b)
}

fn missing(element: &str) -> Message {
    Message::new("issue-data-missing").arg("element", element).arg("data", "d")
}

#[rstest]
#[case::a_merges_first(true)]
#[case::b_merges_first(false)]
fn concurrent_remove_data_and_bind_renders_failure_frame(#[case] a_first: bool) {
    let (merged, _, _) = concurrent(vec![table("u", "d")], a_first);
    assert_that!(merged.data.contains_key(&did("d"))).is_false();
    let (plans, issues) = plan_views(&merged, &common::registry());
    let messages: Vec<(Option<String>, Message)> = issues.into_iter().map(|i| (i.element, i.message)).collect();
    assert_that!(messages).is_equal_to(vec![(Some("u".to_owned()), missing("u"))]);
    let sibling = plans.iter().find(|p| p.view.element == "t").map(|p| p.outcome.clone());
    assert_that!(sibling).is_equal_to(Some(PlanOutcome::Resolve { cut: false }));
}

fn did(id: &str) -> DataId {
    ok(DataId::try_from(id.to_owned()))
}

#[test]
fn undo_remove_data_after_peer_set_cell_keeps_peer_edit() {
    let set = op(json!({ "op": "edit_data", "data": "d",
        "change": { "op": "set_cell", "row": "r1", "column": "c2", "value": 99 } }));
    let (_, mut a, mut b) = concurrent(vec![set], true);
    assert_that!(ok(a.undo())).is_true();
    ok(b.merge(&a));
    let (restored, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(b.deck().0).is_equal_to(&restored);
    let data = restored.data.get(&did("d")).unwrap_or_else(|| panic!("d not restored"));
    assert_that!(data.table.cell("r1", "c2")).is_equal_to(oku_slides_data::CellRead::Overlay(&DataCell::Number(99.0)));
}

#[test]
fn undo_remove_data_after_peer_bound_view_renders_without_issue() {
    let (_, mut a, mut b) = concurrent(vec![table("u", "d")], true);
    assert_that!(ok(a.undo())).is_true();
    ok(b.merge(&a));
    let (restored, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(b.deck().0).is_equal_to(&restored);
    let (plans, issues) = plan_views(&restored, &common::registry());
    assert_that!(issues).is_empty();
    let outcomes: Vec<(String, PlanOutcome)> = plans.into_iter().map(|p| (p.view.element, p.outcome)).collect();
    assert_that!(outcomes).contains((String::from("u"), PlanOutcome::Resolve { cut: false }));
}
