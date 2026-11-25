//! Views of deck data through `Session::apply`: inline data lifted into one data, and a cell
//! edit redrawing only the views that show it.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

#[allow(dead_code, reason = "shared helpers: each test binary uses only some of them")]
mod common;

use common::{data_dir, ok, registry, session};
use oku_slides_core::{Op, kinds};
use oku_slides_data::{CellRead, DataCell};
use oku_slides_render::resolve_views;
use oku_slides_session::Session;
use serde_json::{Value, json};
use speculoos::prelude::*;

fn op(v: Value) -> Op {
    ok(serde_json::from_value(v))
}

fn apply(s: &mut Session, ops: Vec<Value>) {
    ok(s.apply("edit", ops.into_iter().map(op).collect(), None));
}

/// A chart option drawing `values` as one bar series over the months `x` and `y`.
fn inline_option(values: [f64; 2]) -> Value {
    json!({ "dataset": { "source": [["M", "A"], ["x", values[0]], ["y", values[1]]] },
        "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] })
}

/// A fresh deck with chart `g`, drawn from inline `values`, on slide `s-g`.
fn inline_chart(s: &mut Session, values: [f64; 2]) {
    ok(s.new_deck());
    apply(
        s,
        vec![json!({ "op": "add_slide", "slide": { "id": "s-g", "kind": kinds::essentiel::CONTENT, "params": {
        "title": "G", "blocks": [{ "id": "g", "kind": kinds::base::CHART, "params": { "option": inline_option(values) } }] } } })],
    );
}

fn reset(s: &mut Session, values: [f64; 2]) {
    apply(
        s,
        vec![json!({ "op": "set_param", "target": { "slide": "s-g", "element": "g" }, "param": "option",
        "value": inline_option(values) })],
    );
}

/// The data chart `g` reads.
fn bound(s: &Session) -> String {
    let slide = s.deck().slides.iter().find(|x| x.id == "s-g").unwrap_or_else(|| panic!("no slide s-g"));
    let g = slide.walk().find(|l| l.element.id == "g").unwrap_or_else(|| panic!("no chart g")).element;
    match g.params.get("option") {
        Some(oku_slides_core::ParamValue::Value(v)) => v["data"].as_str().unwrap_or_default().to_owned(),
        other => panic!("no chart option: {other:?}"),
    }
}

/// Cell (`row`, 1) of the deck's only data.
fn value(s: &Session, row: usize) -> f64 {
    let data = s.deck().data.values().next().unwrap_or_else(|| panic!("no data"));
    let table = &data.table;
    let row = table.rows.get(row).unwrap_or_else(|| panic!("no row {row}"));
    match table.cell(row, &table.columns[1].id) {
        CellRead::Overlay(DataCell::Number(n)) => *n,
        other => panic!("not a number: {other:?}"),
    }
}

#[test]
fn reset_inline_three_times_via_session_apply_keeps_one_data() {
    let dir = data_dir();
    let mut s = session();
    inline_chart(&mut s, [1.0, 2.0]);
    let id = bound(&s);
    let copy = dir.join("copy.oku-slides.pdf");
    ok(s.save(Some(&copy)));
    reset(&mut s, [3.0, 4.0]);
    reset(&mut s, [5.0, 6.0]);
    assert_that!(s.deck().data.len()).is_equal_to(1);
    assert_that!(bound(&s)).is_equal_to(&id);

    ok(s.undo());
    assert_that!(value(&s, 0)).is_equal_to(3.0);
    ok(s.undo());
    ok(s.undo());
    assert_that!(s.deck().data.is_empty()).is_true();
    for _ in 0..3 {
        ok(s.redo());
    }

    // A peer resets the same inline data from the saved copy: the merge keeps one data.
    let mut peer = session();
    ok(peer.open(&copy));
    reset(&mut peer, [7.0, 8.0]);
    let theirs = dir.join("peer.oku-slides.pdf");
    ok(peer.save(Some(&theirs)));
    ok(s.merge_file(&theirs));
    assert_that!(s.deck().data.len()).is_equal_to(1);
    assert_that!(bound(&s)).is_equal_to(&id);
}

/// Data `d`, 150 rows of (row, row × 10), shown by chart `g` on `s-g` and table `t` on `s-t`.
fn shared_150(s: &mut Session) {
    ok(s.new_deck());
    let rows: Vec<Value> = (0..150).map(|r| json!({ "id": format!("r{r}"), "cells": [r, r * 10] })).collect();
    let table = json!({ "generation": "g1", "columns": [{ "id": "k", "name": "K" }, { "id": "v", "name": "V" }], "rows": rows });
    let slide = |id: &str, block: Value| json!({ "op": "add_slide", "slide": { "id": id, "kind": kinds::essentiel::CONTENT, "params": { "title": id, "blocks": [block] } } });
    apply(
        s,
        vec![
            json!({ "op": "add_data", "id": "d", "name": "d", "table": table }),
            slide(
                "s-g",
                json!({ "id": "g", "kind": kinds::base::CHART, "params": { "option": {
                "data": "d", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } }),
            ),
            slide("s-t", json!({ "id": "t", "kind": kinds::base::TABLE, "params": { "data": { "data": "d" } } })),
        ],
    );
}

#[test]
fn edit_on_row_150_reaches_chart_not_table() {
    let mut s = session();
    shared_150(&mut s);
    // The view key: the slide as the renderer receives it, its views resolved. (Page hashes
    // move with any deck edit, so they cannot tell views apart.)
    let key = |s: &Session, slide: &str| {
        let resolved = resolve_views(s.deck(), &registry(), None);
        let found = resolved.slides.iter().find(|x| x.id == slide);
        ok(serde_json::to_value(&found.unwrap_or_else(|| panic!("slide {slide}")).params))
    };
    let (chart, table) = (key(&s, "s-g"), key(&s, "s-t"));
    let edit = json!({ "op": "edit_data", "data": "d", "change": { "op": "set_cell", "row": "r149", "column": "v", "value": 918_273 } });
    apply(&mut s, vec![edit]);
    assert_that!(key(&s, "s-g")).is_not_equal_to(chart);
    assert_that!(key(&s, "s-t")).is_equal_to(table);
}
