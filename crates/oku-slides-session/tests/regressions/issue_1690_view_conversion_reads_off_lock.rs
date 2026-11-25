//! t-1690: a chart kind change or a chart/table swap on a data kept in a base read the base
//! file with the tab's lock held. Both now plan the read under the lock and run it with the
//! lock released, like an edit's fold: the request must be routed through that folding path.

use std::time::{Duration, Instant};

use oku_slides_chart::SeriesType;
use oku_slides_core::Op;
use oku_slides_data::limits::COMPACT_AT;
use oku_slides_session::{Request, SessionErrorKind, View, Workspace};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::{data_dir, fixture_path};
use crate::workspace_support::{on, op, workspace};

/// `d9`, over the compaction threshold, and the chart `ch1` on `s-libre` that shows it.
fn data_and_chart() -> Vec<Op> {
    let rows: Vec<Value> =
        (0..COMPACT_AT).map(|i| json!({ "id": format!("r{i}"), "cells": [format!("m{i}"), i] })).collect();
    let table = json!({ "columns": [{ "id": "c1", "name": "mois", "type": "text" },
        { "id": "c2", "name": "ventes", "type": "number" }], "rows": rows });
    vec![
        op(json!({ "op": "add_data", "id": "d9", "name": "ventes", "table": table })),
        op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-libre" }, "param": "elements" },
            "element": { "id": "ch1", "frame": { "x": 0, "y": 0, "w": 400, "h": 300 }, "kind": "base/chart",
                "params": { "option": { "data": "d9", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } } })),
    ]
}

/// A tab whose `d9` reads from a base: saved, reopened with no undo touching it, compacted.
fn based_tab(ws: &std::sync::Mutex<Workspace>, data: &crate::common::Data) -> String {
    let tab = Workspace::lock(ws).view().windows[0].tabs[0].id.clone();
    let saved = data.join("ventes.oku-slides.pdf");
    on(ws, &tab, |s| {
        s.open(&fixture_path())?;
        s.apply("données", data_and_chart(), None)?;
        s.save(Some(&saved))?;
        s.open(&saved)
    })
    .unwrap_or_else(|e| panic!("setup: {}", e.one_line()));
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        // Every request schedules the compactions due, run in the background.
        let pane = Request::DataPanel { slide: "s-libre".to_owned(), element: "ch1".to_owned() };
        drop(Workspace::request(ws, &tab, pane));
        if on(ws, &tab, |s| s.deck().data.values().any(|d| d.table.base.is_some())) {
            return tab;
        }
        assert_that!(Instant::now() < deadline).named("d9 compacted in time").is_true();
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn ch1(ws: &std::sync::Mutex<Workspace>, tab: &str) -> Value {
    on(ws, tab, |s| {
        let slide = s.deck().slide("s-libre").unwrap_or_else(|| panic!("no s-libre"));
        let located = slide.locate("ch1").unwrap_or_else(|| panic!("no ch1"));
        serde_json::to_value(located.element).unwrap_or_else(|e| panic!("{e}"))
    })
}

#[test]
fn issue_1690_chart_kind_on_based_data_is_applied() {
    let data = data_dir();
    let ws = workspace(&data);
    let tab = based_tab(&ws, &data);
    let req = Request::ChartKind {
        label: "type".to_owned(),
        slide: "s-libre".to_owned(),
        element: "ch1".to_owned(),
        kind: SeriesType::Line,
    };

    let (reply, _) = Workspace::request(&ws, &tab, req);

    assert_that!(reply.map(drop).map_err(|e| e.one_line())).is_ok();
    assert_that!(ch1(&ws, &tab).pointer("/params/option/series/0/type")).is_equal_to(Some(&json!("line")));
}

/// A base holds at least `COMPACT_AT` rows, more than a table shows: the swap reads the base
/// off the lock, then the conversion itself refuses, never the lock's stale read.
#[test]
fn issue_1690_show_as_table_on_based_data_reaches_the_conversion() {
    let data = data_dir();
    let ws = workspace(&data);
    let tab = based_tab(&ws, &data);
    let before = ch1(&ws, &tab);
    let req = Request::ShowAs {
        label: "tableau".to_owned(),
        slide: "s-libre".to_owned(),
        element: "ch1".to_owned(),
        view: View::Table,
    };

    let (reply, _) = Workspace::request(&ws, &tab, req);

    let kind = reply.map(drop).map_err(|e| e.kind());
    assert_that!(kind).is_equal_to(Err(SessionErrorKind::Chart));
    assert_that!(ch1(&ws, &tab)).is_equal_to(before);
}
