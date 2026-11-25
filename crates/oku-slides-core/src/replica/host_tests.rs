//! `host` params in the replica: an opaque option naming deck data; a stored `dataset` is dropped.

use loro::{Container, LoroMap, LoroValue, ValueOrContainer};
use oku_slides_i18n::Localize;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::*;
use crate::{Deck, Op, apply_all};

fn ok<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e:?}"))
}

fn registry() -> Arc<WidgetRegistry> {
    crate::test_registry::shared()
}

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("invalid op: {e}"))
}

fn with_ops(deck: &Deck, ops: Vec<Op>) -> Deck {
    let mut d = deck.clone();
    apply_all(&mut d, ops, &registry()).unwrap_or_else(|e| panic!("ops refused: {e:?}"));
    d
}

/// A deck with the data `d` (columns `c1` Country, `c2` Sales) and slide `s`.
fn data_deck() -> Deck {
    let deck = ok(Deck::from_json(r#"{"slides":[{"id":"s","kind":"essentiel/content"}]}"#));
    let add = op(json!({ "op": "add_data", "id": "d", "name": "sales", "table": {
        "generation": "g1",
        "columns": [{ "id": "c1", "name": "Country" }, { "id": "c2", "name": "Sales" }],
        "rows": [{ "id": "r1", "cells": ["FR", 1.0] }, { "id": "r2", "cells": ["DE", 2.5] }]
    } }));
    with_ops(&deck, vec![add])
}

/// `deck` plus the element `id` of `kind` whose `param` is `option`.
fn with_view(deck: &Deck, id: &str, kind: &str, param: &str, option: &Value) -> Deck {
    let add = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s" }, "param": "blocks" },
        "element": { "id": id, "kind": kind, "params": { param: option } } }));
    with_ops(deck, vec![add])
}

/// The deck with table `t` showing `d`, aligned.
fn table_deck() -> Deck {
    with_view(&data_deck(), "t", "base/table", "data", &json!({ "data": "d", "align": ["start", "end"] }))
}

/// The deck with chart `g` showing `d`.
fn chart_deck() -> Deck {
    let option = json!({ "data": "d", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] });
    with_view(&data_deck(), "g", "base/chart", "option", &option)
}

/// The param `param` of element `id`.
fn stored(deck: &Deck, id: &str, param: &str) -> Value {
    let e = deck.slides[0].locate(id).unwrap_or_else(|| panic!("no element {id}"));
    e.element.params.value(param).unwrap_or_else(|| panic!("no {param}")).clone()
}

/// Reconciles `deck` into a fresh replica, then reads its snapshot back.
fn round_trip(deck: &Deck) -> Deck {
    let r = ok(DeckReplica::new(deck, registry()));
    let (back, issues) = ok(DeckReplica::from_snapshot(&ok(r.snapshot()), 1 << 24, registry())).1;
    assert_that!(issues).is_empty();
    back
}

fn as_map(v: Option<ValueOrContainer>) -> LoroMap {
    match v {
        Some(ValueOrContainer::Container(Container::Map(m))) => m,
        other => panic!("map expected: {other:?}"),
    }
}

/// The option map of param `param` of element `id` on slide `s`.
fn host_map(r: &DeckReplica, id: &str, param: &str) -> LoroMap {
    let (tree, n) = tree::node_of(&r.doc, "s", id);
    as_map(as_map(ok(tree.get_meta(n)).get("params")).get(param))
}

#[test]
fn host_table_round_trips_through_the_replica() {
    let deck = table_deck();
    let back = round_trip(&deck);
    assert_that!(back).is_equal_to(&deck);
    let option = stored(&back, "t", "data");
    assert_that!(option["data"]).is_equal_to(json!("d"));
    assert_that!(option["align"]).is_equal_to(json!(["start", "end"]));
}

#[test]
fn host_chart_round_trips_through_the_replica() {
    let deck = chart_deck();
    assert_that!(round_trip(&deck)).is_equal_to(&deck);
    let r = ok(DeckReplica::new(&deck, registry()));
    assert_that!(host_map(&r, "g", "option").get("data").and_then(|v| v.into_value().ok()))
        .is_equal_to(Some(LoroValue::from("d")));
}

#[test]
fn rewriting_an_unchanged_deck_writes_nothing() {
    let deck = table_deck();
    let mut r = ok(DeckReplica::new(&deck, registry()));
    let before = r.len_ops();
    ok(r.apply(&deck, "Nothing", None));
    ok(r.apply(&r.deck().0, "Nothing", None));
    assert_that!(r.len_ops()).is_equal_to(before);
}

#[test]
fn forged_nested_maps_in_a_host_option_read_as_opaque_values() {
    let r = ok(DeckReplica::new(&table_deck(), registry()));
    let nested = ok(host_map(&r, "t", "data").insert_container("chart", LoroMap::new()));
    ok(nested.insert("title", "x"));
    r.doc.commit();
    let (deck, issues) = r.deck();
    assert_that!(issues).is_empty();
    assert_that!(stored(&deck, "t", "data")["chart"]).is_equal_to(json!({ "title": "x" }));
}

#[test]
fn replica_read_drops_stored_dataset_and_reports() {
    let r = ok(DeckReplica::new(&chart_deck(), registry()));
    let host = host_map(&r, "g", "option");
    let dataset = ok(host.insert_container(schema::DATASET, LoroMap::new()));
    ok(dataset.insert("source", LoroValue::from(vec![LoroValue::from(vec![LoroValue::from("A")])])));
    r.doc.commit();
    let (deck, issues) = r.deck();
    assert_that!(stored(&deck, "g", "option").get("dataset")).is_none();
    assert_that!(stored(&deck, "g", "option")["data"]).is_equal_to(json!("d"));
    assert_that!(issues.len()).is_equal_to(1);
    assert_that!(issues[0].element.as_deref()).is_equal_to(Some("g"));
    assert_that!(issues[0].slide.as_deref()).is_equal_to(Some("s"));
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(issues[0].message.localize(&fr))
        .is_equal_to(fr.tr_with("issue-data-inline-dropped", &[("element", "g".into())]));
}

#[test]
fn stored_deck_and_snapshot_have_no_dataset_key() {
    let mut deck = data_deck();
    let option = json!({
        "dataset": { "source": [["Month", "Sales"], ["Jan", 3], ["Feb", 5]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }]
    });
    let element = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s" }, "param": "blocks" },
        "element": { "id": "g", "kind": "base/chart", "params": { "option": option } } }));
    let registry = registry();
    let labels = oku_slides_chart::ChartLabels::default();
    let mut n = 0;
    let mut ids = |taken: &dyn Fn(&str) -> bool| loop {
        n += 1;
        let id = format!("x{n}");
        if !taken(&id) {
            return id;
        }
    };
    let mut ctx = crate::PrepareCtx {
        deck: &deck,
        registry: &registry,
        labels: &labels,
        ids: &mut ids,
        origin: crate::Origin::External,
    };
    let prepared = ok(crate::prepare_ops(vec![element], &mut ctx));
    ok(apply_all(&mut deck, prepared.ops, &registry));
    assert_that!(deck.data.len()).is_equal_to(2);
    assert_that!(ok(deck.to_json()).contains("\"dataset\"")).is_false();
    let r = ok(DeckReplica::new(&deck, registry.clone()));
    let snapshot = ok(DeckReplica::from_snapshot(&ok(r.snapshot()), 1 << 24, registry)).0;
    // A shallow document built here: `get_deep_value` is safe on it.
    let doc = format!("{:?}", snapshot.doc.get_deep_value());
    assert_that!(doc.contains("data")).is_true();
    assert_that!(doc.contains("\"dataset\"")).is_false();
}
