use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use oku_slides_core::{Deck, Op};
use oku_slides_data::{Data, DataId};
use serde_json::json;
use speculoos::prelude::*;

use super::{Queries, copy_data_ops};

fn data(name: &str) -> Arc<Data> {
    Arc::new(
        serde_json::from_value(json!({
            "name": name,
            "label": "Sales",
            "kind": "manual",
            "table": {
                "generation": "g0",
                "columns": [{"id": "cat", "name": "Category", "type": "text"}, {"id": "v", "name": "Value", "type": "number"}],
                "rows": ["r1", "r2"],
                "cells": {"r1/cat": "A", "r1/v": 3, "r2/cat": "B", "r2/v": 5}
            }
        }))
        .unwrap(),
    )
}

fn id(s: &str) -> DataId {
    DataId::try_from(s.to_owned()).unwrap()
}

/// Draws `n1`, `n2`… skipping taken ones.
fn counter() -> impl FnMut(&dyn Fn(&str) -> bool) -> String {
    let mut n = 0;
    move |taken: &dyn Fn(&str) -> bool| loop {
        n += 1;
        let c = format!("n{n}");
        if !taken(&c) {
            return c;
        }
    }
}

#[test]
fn copy_data_ops_keeps_row_and_column_ids() {
    let source = BTreeMap::from([(id("ex"), data("sales"))]);
    let mut ids = counter();
    let (ops, map) =
        copy_data_ops(&source, &[id("ex")], &Deck::default(), &BTreeSet::new(), &mut ids, Queries::Refused).unwrap();
    let [Op::AddData { id: Some(new), name, label, table: Some(table), sql: None }] = ops.as_slice() else {
        panic!("one add_data: {ops:?}")
    };
    assert_that!(map).is_equal_to(BTreeMap::from([(id("ex"), new.clone())]));
    // The counter's first draw names the copy, the second its generation.
    assert_that!(new.as_str()).is_equal_to("n1");
    assert_that!(name.as_str()).is_equal_to("sales");
    assert_that!(label.as_deref()).is_equal_to(Some("Sales"));
    assert_that!(table.generation.as_deref()).is_equal_to(Some("n2"));
    let columns: Vec<Option<&str>> = table.columns.iter().map(|c| c.id.as_deref()).collect();
    assert_that!(columns).is_equal_to(vec![Some("cat"), Some("v")]);
    let rows: Vec<Option<&str>> = table.rows.iter().map(|r| r.id.as_deref()).collect();
    assert_that!(rows).is_equal_to(vec![Some("r1"), Some("r2")]);
    assert_that!(table.overlay.clone()).is_equal_to(source[&id("ex")].table().unwrap().cells.clone());
}

#[test]
fn copy_data_ops_refs_repeated_or_missing_copy_once() {
    let source = BTreeMap::from([(id("ex"), data("sales"))]);
    let mut ids = counter();
    let (ops, map) = copy_data_ops(
        &source,
        &[id("ex"), id("gone"), id("ex")],
        &Deck::default(),
        &BTreeSet::new(),
        &mut ids,
        Queries::Refused,
    )
    .unwrap();
    assert_that!(ops).has_length(1);
    assert_that!(map.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec![id("ex")]);
}

#[test]
fn copy_data_ops_name_taken_in_deck_gets_a_suffix() {
    let source = BTreeMap::from([(id("a"), data("sales")), (id("b"), data("Sales"))]);
    let deck = Deck { data: BTreeMap::from([(id("n1"), data("SALES"))]), ..Deck::default() };
    let mut ids = counter();
    let (ops, map) =
        copy_data_ops(&source, &[id("a"), id("b")], &deck, &BTreeSet::new(), &mut ids, Queries::Refused).unwrap();
    let names: Vec<&str> = ops
        .iter()
        .filter_map(|op| if let Op::AddData { name, .. } = op { Some(name.as_str()) } else { None })
        .collect();
    assert_that!(names).is_equal_to(vec!["sales_2", "sales_3"]);
    assert_that!(map.values().any(|v| v.as_str() == "n1")).is_false();
}

#[test]
fn copy_data_ops_skips_ids_the_batch_adds() {
    let source = BTreeMap::from([(id("ex"), data("sales"))]);
    let mut ids = counter();
    let reserved = BTreeSet::from(["n1".to_owned()]);
    let (_, map) =
        copy_data_ops(&source, &[id("ex")], &Deck::default(), &reserved, &mut ids, Queries::Refused).unwrap();
    assert_that!(map.get(&id("ex")).map(DataId::as_str)).is_equal_to(Some("n2"));
}

fn query() -> Arc<Data> {
    Arc::new(
        serde_json::from_value(json!({ "name": "top", "label": "Top", "kind": "query", "sql": "SELECT a FROM sales" }))
            .unwrap(),
    )
}

#[test]
fn copy_data_ops_kept_query_is_copied_as_its_sql() {
    let source = BTreeMap::from([(id("q"), query())]);
    let mut ids = counter();
    let deck = Deck::default();
    let (ops, _) = copy_data_ops(&source, &[id("q")], &deck, &BTreeSet::new(), &mut ids, Queries::Kept).unwrap();
    let [Op::AddData { name, label, table: None, sql: Some(sql), .. }] = ops.as_slice() else {
        panic!("one query: {ops:?}")
    };
    assert_that!((name.as_str(), label.as_deref(), sql.as_str())).is_equal_to((
        "top",
        Some("Top"),
        "SELECT a FROM sales",
    ));
}

#[test]
fn copy_data_ops_query_without_a_result_is_refused() {
    let source = BTreeMap::from([(id("q"), query())]);
    let none = BTreeMap::new();
    for queries in [Queries::Refused, Queries::Values(&none)] {
        let mut ids = counter();
        let copied = copy_data_ops(&source, &[id("q")], &Deck::default(), &BTreeSet::new(), &mut ids, queries);
        assert_that!(copied).is_err();
    }
}
