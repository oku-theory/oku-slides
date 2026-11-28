//! The data pane's picker: the data a view may switch to, and the `bind` edit that switches it.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

#[allow(dead_code, reason = "shared helpers: each test binary uses only some of them")]
mod common;

use common::{ok, req, session};
use oku_slides_core::kinds;
use oku_slides_session::{Localize, Localizer, Session, SessionErrorKind};
use serde_json::{Value, json};
use speculoos::prelude::*;

/// A table of `columns` (ids `c1`… named `a`…), one row.
fn table(columns: usize) -> Value {
    let names = ["a", "b", "c"];
    let cols: Vec<Value> = (0..columns).map(|i| json!({ "id": format!("c{}", i + 1), "name": names[i] })).collect();
    json!({ "columns": cols, "rows": [{ "id": "r1", "cells": (0..columns).collect::<Vec<_>>() }] })
}

/// A new deck holding `d1` (c1, c2), `d2` (c1, c2, c3) and `d3` (c1), with table `t` over `d1`
/// on its second slide; returns that slide.
fn three_data(s: &mut Session) -> String {
    ok(s.new_deck());
    let slide = s.snapshot().slides[1].id.to_string();
    let add = |id: &str, name: &str, columns: usize| json!({ "op": "add_data", "id": id, "name": name, "table": table(columns) });
    let element = json!({ "op": "add_element", "slot": { "owner": { "slide": slide }, "param": "blocks" },
        "element": { "id": "t", "kind": kinds::base::TABLE, "params": { "data": { "data": "d1" } } } });
    let ops = json!([add("d1", "ventes", 2), add("d2", "budget", 3), add("d3", "mois", 1), element]);
    ok(s.handle(req(json!({ "cmd": "apply", "label": "data", "ops": ops }))));
    slide
}

/// The `TableData` component of `t`.
fn pane(s: &mut Session, slide: &str) -> Value {
    let text = ok(s.handle(req(json!({ "cmd": "data_panel", "slide": slide, "element": "t" }))));
    let surface: Value = ok(serde_json::from_str(&text));
    let messages = surface["messages"].as_array().cloned().unwrap_or_default();
    let components =
        messages.iter().find_map(|m| m["updateComponents"]["components"].as_array().cloned()).unwrap_or_default();
    components.into_iter().find(|c| c["component"] == "TableData").unwrap_or(Value::Null)
}

/// The data `t` names.
fn bound(s: &mut Session, slide: &str) -> Value {
    let text = ok(s.handle(req(json!({ "cmd": "slide", "id": slide }))));
    let json: Value = ok(serde_json::from_str(&text));
    let blocks = json["slide"]["params"]["blocks"].as_array().cloned().unwrap_or_default();
    blocks.into_iter().find(|b| b["id"] == "t").map_or(Value::Null, |e| e["params"]["data"]["data"].clone())
}

fn bind(s: &mut Session, slide: &str, data: &str) -> oku_slides_session::Result<String> {
    let edit = json!({ "edit": "bind", "data": data });
    s.handle(req(json!({ "cmd": "data_edit", "slide": slide, "element": "t", "edit": edit })))
}

#[test]
fn bind_edit_switches_view_data_one_undo_step() {
    let mut s = session();
    let slide = three_data(&mut s);
    ok(bind(&mut s, &slide, "d2"));
    let switched = bound(&mut s, &slide);
    ok(s.handle(req(json!({ "cmd": "undo" }))));
    assert_that!((switched, bound(&mut s, &slide))).is_equal_to((json!("d2"), json!("d1")));
}

#[test]
fn bind_edit_to_missing_data_is_refused() {
    let mut s = session();
    let slide = three_data(&mut s);
    let e = bind(&mut s, &slide, "d9").err().unwrap_or_else(|| panic!("a missing data is refused"));
    let fr = Localizer::new(Some("fr"));
    let missing = fr.tr_with("op-data-missing", &[("data", "d9".into())]);
    assert_that!((e.kind(), e.localize(&fr).ends_with(&missing), bound(&mut s, &slide))).is_equal_to((
        SessionErrorKind::Op,
        true,
        json!("d1"),
    ));
}

#[test]
fn choices_list_only_data_with_used_columns() {
    let mut s = session();
    let slide = three_data(&mut s);
    let pane = pane(&mut s, &slide);
    assert_that!((pane["data"].clone(), pane["choices"].clone())).is_equal_to((
        json!({ "id": "d1", "name": "ventes", "users": 1 }),
        json!([{ "id": "d1", "name": "ventes" }, { "id": "d2", "name": "budget" }]),
    ));
}

#[test]
fn every_choice_binds() {
    let mut s = session();
    let slide = three_data(&mut s);
    let choices = pane(&mut s, &slide)["choices"].as_array().cloned().unwrap_or_default();
    let ids: Vec<String> = choices.iter().filter_map(|c| c["id"].as_str()).map(str::to_owned).collect();
    let bound_all = ids.iter().all(|id| bind(&mut s, &slide, id).is_ok() && bound(&mut s, &slide) == json!(id));
    assert_that!((ids.len(), bound_all)).is_equal_to((2, true));
}

#[test]
fn data_users_counts_every_view_of_the_data() {
    let mut s = session();
    let slide = three_data(&mut s);
    let second = json!({ "op": "add_element", "slot": { "owner": { "slide": slide }, "param": "blocks" },
        "element": { "id": "u", "kind": kinds::base::TABLE, "params": { "data": { "data": "d1" } } } });
    ok(s.handle(req(json!({ "cmd": "apply", "label": "second", "ops": [second] }))));
    assert_that!(pane(&mut s, &slide)["data"]["users"].clone()).is_equal_to(json!(2));
}

/// The choice ids of `t`'s pane.
fn choice_ids(s: &mut Session, slide: &str) -> Vec<String> {
    let choices = pane(s, slide)["choices"].as_array().cloned().unwrap_or_default();
    choices.iter().filter_map(|c| c["id"].as_str()).map(str::to_owned).collect()
}

#[test]
fn picker_switches_away_from_inline_data_and_back() {
    let mut s = session();
    ok(s.new_deck());
    let slide = s.snapshot().slides[1].id.to_string();
    let element = json!({ "op": "add_element", "slot": { "owner": { "slide": slide }, "param": "blocks" },
        "element": { "id": "t", "kind": kinds::base::TABLE,
            "params": { "data": { "dataset": { "source": [["A", "B"], [1, 2]] } } } } });
    ok(s.handle(req(json!({ "cmd": "apply", "label": "inline", "ops": [element] }))));
    let (lifted, inline, table) = {
        let (id, data) = s.deck().data.iter().next().unwrap_or_else(|| panic!("the dataset is lifted"));
        (
            id.as_str().to_owned(),
            data.inline,
            ok(serde_json::to_value(data.table().unwrap_or_else(|| panic!("a manual data")))),
        )
    };
    // Same columns (ids included), one row of its own.
    let columns = table["columns"].clone();
    let copy = json!({ "op": "add_data", "id": "d2", "name": "copie",
        "table": { "columns": columns, "rows": [{ "id": "r1", "cells": [3, 4] }] } });
    ok(s.handle(req(json!({ "cmd": "apply", "label": "copy", "ops": [copy] }))));
    ok(bind(&mut s, &slide, "d2"));
    let away = (bound(&mut s, &slide), s.deck().data.iter().any(|(id, d)| id.as_str() == lifted && d.inline));
    let offered = choice_ids(&mut s, &slide);
    let back = offered.contains(&lifted) && bind(&mut s, &slide, &lifted).is_ok();
    assert_that!((inline, away, offered.len(), back, bound(&mut s, &slide))).is_equal_to((
        true,
        (json!("d2"), true),
        2,
        true,
        json!(lifted),
    ));
}
