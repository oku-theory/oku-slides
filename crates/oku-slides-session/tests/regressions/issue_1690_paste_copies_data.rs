//! t-1690: a pasted view kept its source's data ref. Pasting it into another deck left it
//! naming a data that deck lacks; and a ref equal to a pack example data id would have
//! copied the pack's example instead of the source's data. `PasteElement` now copies the
//! source's data under a fresh id and rewrites every ref before the batch applies.

use std::sync::Mutex;

use oku_slides_session::{Localize, Localizer, Result, SessionErrorKind, Workspace, WorkspaceOutcome};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::{data_dir, fixture_path};
use crate::workspace_support::{on, op, send, workspace};

/// The id of the base pack's example chart data.
const EXAMPLE: &str = "base-example-chart";

/// Two tabs on roadmap.json, the first holding the data `EXAMPLE` ("ventes") shown by `ch1`.
fn two_tabs(ws: &Mutex<Workspace>) -> (String, String) {
    let window = send(ws, json!({ "cmd": "view" }))["windows"][0]["label"].as_str().map(str::to_owned);
    send(ws, json!({ "cmd": "new_tab", "window": window.unwrap_or_else(|| panic!("no window")) }));
    let tabs: Vec<String> = Workspace::lock(ws).view().windows[0].tabs.iter().map(|t| t.id.clone()).collect();
    for tab in &tabs {
        on(ws, tab, |s| s.open(&fixture_path())).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    }
    let table = json!({ "columns": [{ "id": "c1", "name": "Mois", "type": "text" },
        { "id": "c2", "name": "ventes", "type": "number" }],
        "rows": [{ "id": "r1", "cells": ["Jan", 7] }] });
    let ops = vec![op(json!({ "op": "add_data", "id": EXAMPLE, "name": "ventes", "table": table })), op(chart())];
    on(ws, &tabs[0], |s| s.apply("data", ops, None)).unwrap_or_else(|e| panic!("apply: {}", e.one_line()));
    (tabs[0].clone(), tabs[1].clone())
}

/// The paste op of the chart `ch1` naming `EXAMPLE`.
fn chart() -> Value {
    json!({ "op": "add_element", "slot": { "owner": { "slide": "s-libre" }, "param": "elements" },
        "element": { "id": "ch1", "frame": { "x": 0, "y": 0, "w": 400, "h": 300 }, "kind": "base/chart", "params": { "option": {
            "data": EXAMPLE, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } } })
}

fn paste(ws: &Mutex<Workspace>, from: &str, to: &str) -> Result<WorkspaceOutcome> {
    let req = json!({ "cmd": "paste_element", "from": from, "to": to, "label": "Coller", "ops": [chart()] });
    Workspace::apply(ws, serde_json::from_value(req).unwrap_or_else(|e| panic!("request: {e}")))
}

#[test]
fn issue_1690_paste_into_another_deck_copies_the_source_data_not_the_pack_example() {
    let data = data_dir();
    let ws = workspace(&data);
    let (a, b) = two_tabs(&ws);
    paste(&ws, &a, &b).unwrap_or_else(|e| panic!("paste: {}", e.one_line()));
    let deck = on(&ws, &b, |s| s.deck().clone());
    let copies: Vec<_> = deck.data.iter().collect();
    assert_that!(copies.len()).is_equal_to(1);
    let (id, copy) = copies[0];
    assert_that!(id.as_str()).is_not_equal_to(EXAMPLE);
    assert_that!(copy.name.as_str()).is_equal_to("ventes");
    assert_that!(copy.table.rows.iter().collect::<Vec<_>>()).is_equal_to(vec!["r1"]);
}

#[test]
fn issue_1690_paste_from_a_closed_tab_never_copies_the_pack_example() {
    let data = data_dir();
    let ws = workspace(&data);
    let (a, b) = two_tabs(&ws);
    send(&ws, json!({ "cmd": "close", "tab": a, "discard": true }));
    let e = paste(&ws, &a, &b).err().unwrap_or_else(|| panic!("a closed source is refused"));
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::Op);
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr_with("op-data-missing", &[("data", EXAMPLE.into())]));
    assert_that!(on(&ws, &b, |s| s.deck().data.len())).is_equal_to(0);
}
