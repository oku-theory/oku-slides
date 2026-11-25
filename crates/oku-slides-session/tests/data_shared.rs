//! Pane edits of a data larger than the pane or shared by several views: a paste, a chart type
//! change, the merge key of a cell edit, and an edit on a row a peer removed.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

#[allow(dead_code, reason = "shared helpers: each test binary uses only some of them")]
mod common;

use common::{ok, req, session, with_element};
use oku_slides_core::kinds;
use oku_slides_data::{DataCell, Dataset};
use oku_slides_session::{Localize, Session};
use serde_json::{Value, json};
use speculoos::prelude::*;

/// Chart `g` over 150 rows `r{i}` of columns M, A (`i`), B (`i`) and C (`1000 + i`), drawing
/// A and B only: C is a column of the data the chart does not project.
fn long_chart(s: &mut Session) -> String {
    let mut source = vec![json!(["M", "A", "B", "C"])];
    source.extend((0..150).map(|i| json!([format!("r{i}"), i, i, 1000 + i])));
    let option = json!({ "dataset": { "source": source }, "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }, { "type": "bar" }] });
    with_element(s, json!({ "id": "g", "kind": kinds::base::CHART, "params": { "option": option } }))
}

/// The surface component of `element` (`ChartData` or `TableData`).
fn component(s: &mut Session, slide: &str, element: &str) -> Value {
    let text = ok(s.handle(req(json!({ "cmd": "data_panel", "slide": slide, "element": element }))));
    let surface: Value = ok(serde_json::from_str(&text));
    let messages = surface["messages"].as_array().cloned().unwrap_or_default();
    let components =
        messages.iter().find_map(|m| m["updateComponents"]["components"].as_array().cloned()).unwrap_or_default();
    components
        .into_iter()
        .find(|c| c["component"] == "ChartData" || c["component"] == "TableData")
        .unwrap_or(Value::Null)
}

fn ids(v: &Value) -> Vec<String> {
    v.as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect()
}

/// The param `name` of `element` on `slide`.
fn param(s: &mut Session, slide: &str, element: &str, name: &str) -> Value {
    let text = ok(s.handle(req(json!({ "cmd": "slide", "id": slide }))));
    let json: Value = ok(serde_json::from_str(&text));
    let blocks = json["slide"]["params"]["blocks"].as_array().cloned().unwrap_or_default();
    blocks.into_iter().find(|b| b["id"] == element).map_or(Value::Null, |e| e["params"][name].clone())
}

/// The whole data `g` reads, every row and column.
fn whole(s: &Session, data: &str) -> Dataset {
    let stored = s.deck().data.get(data).unwrap_or_else(|| panic!("no data {data}"));
    ok(stored.table.to_dataset(None, 0..usize::MAX, usize::MAX))
}

fn edit(s: &mut Session, slide: &str, element: &str, edit: &Value) -> oku_slides_session::Result<String> {
    s.handle(req(json!({ "cmd": "data_edit", "slide": slide, "element": element, "edit": edit })))
}

fn undo(s: &mut Session) {
    ok(s.handle(req(json!({ "cmd": "undo" }))));
}

fn num(n: f64) -> DataCell {
    DataCell::Number(n)
}

/// Pastes 4 rows × 2 columns at the pane's row 98 (`r97`) and column B of [`long_chart`].
fn paste_at_r97(s: &mut Session, slide: &str) -> (Vec<String>, Vec<String>) {
    let pane = component(s, slide, "g");
    let (rows, columns) = (ids(&pane["row_ids"]), ids(&pane["column_ids"]));
    let paste = json!({ "edit": "paste", "row": rows[97], "column": columns[2],
        "cells": [[1, 2], [3, 4], [5, 6], [7, 8]], "names": ["N"] });
    ok(edit(s, slide, "g", &paste));
    (rows, columns)
}

#[test]
fn paste_past_the_pane_keeps_rows_beyond_99_and_unprojected_columns() {
    let mut s = session();
    let slide = long_chart(&mut s);
    let data = param(&mut s, &slide, "g", "option")["data"].as_str().unwrap_or_default().to_owned();
    paste_at_r97(&mut s, &slide);
    let all = whole(&s, &data);
    let header: Vec<DataCell> = all.source[0].clone();
    let c = header.iter().position(|h| *h == DataCell::Text("C".into())).unwrap_or(usize::MAX);
    let column_c: Vec<DataCell> = all.source[1..].iter().map(|r| r.get(c).cloned().unwrap_or(DataCell::Null)).collect();
    let expected_c: Vec<DataCell> = (0..150).map(|i| num(f64::from(1000 + i))).collect();
    assert_that!(all.source.len()).is_equal_to(151);
    assert_that!(column_c).is_equal_to(expected_c);
    // Row r150 of the deck (index 149) is beyond the pasted block and the pane: untouched.
    assert_that!(all.source[150][..3].to_vec()).is_equal_to(vec![
        DataCell::Text("r149".into()),
        num(149.0),
        num(149.0),
    ]);
}

#[test]
fn paste_sets_cells_past_the_pane_and_adds_the_overflow_column_to_the_view() {
    let mut s = session();
    let slide = long_chart(&mut s);
    paste_at_r97(&mut s, &slide);
    let pane = component(&mut s, &slide, "g");
    let option = param(&mut s, &slide, "g", "option");
    let data = option["data"].as_str().unwrap_or_default().to_owned();
    let all = whole(&s, &data);
    let n = all.source[0].iter().position(|h| *h == DataCell::Text("N".into())).unwrap_or(usize::MAX);
    // r100 (deck row 101) is beyond the pane: its B cell still takes the paste.
    assert_that!((all.source[101][2].clone(), all.source.get(98).and_then(|r| r.get(n)).cloned()))
        .is_equal_to((num(7.0), Some(num(2.0))));
    assert_that!((pane["source"][0].clone(), option["series"].as_array().map(Vec::len)))
        .is_equal_to((json!(["M", "A", "B", "N"]), Some(3)));
}

#[test]
fn paste_is_one_undo_step() {
    let mut s = session();
    let slide = long_chart(&mut s);
    let data = param(&mut s, &slide, "g", "option")["data"].as_str().unwrap_or_default().to_owned();
    let before = whole(&s, &data);
    let option = param(&mut s, &slide, "g", "option");
    paste_at_r97(&mut s, &slide);
    undo(&mut s);
    assert_that!((whole(&s, &data).source, param(&mut s, &slide, "g", "option"))).is_equal_to((before.source, option));
}

#[test]
fn paste_past_the_last_row_appends_rows() {
    let mut s = session();
    let slide = long_chart(&mut s);
    let data = param(&mut s, &slide, "g", "option")["data"].as_str().unwrap_or_default().to_owned();
    let last = whole(&s, &data).ids[150].clone();
    let columns = ids(&component(&mut s, &slide, "g")["column_ids"]);
    let paste = json!({ "edit": "paste", "row": last, "column": columns[0],
        "cells": [["z1", 1], ["z2", 2], ["z3", 3]], "names": [] });
    ok(edit(&mut s, &slide, "g", &paste));
    let all = whole(&s, &data);
    let tail: Vec<Vec<DataCell>> = all.source[150..].iter().map(|r| r[..2].to_vec()).collect();
    assert_that!(tail).is_equal_to(vec![
        vec![DataCell::Text("z1".into()), num(1.0)],
        vec![DataCell::Text("z2".into()), num(2.0)],
        vec![DataCell::Text("z3".into()), num(3.0)],
    ]);
}

#[test]
fn add_row_from_a_truncated_pane_appends_after_the_last_row() {
    let mut s = session();
    let slide = long_chart(&mut s);
    let data = param(&mut s, &slide, "g", "option")["data"].as_str().unwrap_or_default().to_owned();
    ok(edit(&mut s, &slide, "g", &json!({ "edit": "add_row" })));
    let all = whole(&s, &data);
    let around: Vec<DataCell> = all.source[99..].iter().map(|r| r[0].clone()).collect();
    let mut expected: Vec<DataCell> = (98..150).map(|i| DataCell::Text(format!("r{i}"))).collect();
    expected.push(DataCell::Null);
    assert_that!(around).is_equal_to(expected);
}

/// Chart `g` (two bars over M, A, B) and table `t` on the same slide, showing the same data.
fn shared_chart_and_table(s: &mut Session) -> String {
    let element = json!({ "id": "g", "kind": kinds::base::CHART, "params": { "option": {
        "dataset": { "source": [["M", "A", "B"], ["x", 1, 2], ["y", 3, 4]] },
        "xAxis": { "type": "category" }, "series": [{ "type": "bar" }, { "type": "bar" }] } } });
    let slide = with_element(s, element);
    let data = param(s, &slide, "g", "option")["data"].clone();
    let table = json!({ "op": "add_element", "slot": { "owner": { "slide": slide }, "param": "blocks" },
        "element": { "id": "t", "kind": kinds::base::TABLE, "params": { "data": { "data": data } } } });
    ok(s.handle(req(json!({ "cmd": "apply", "label": "table", "ops": [table] }))));
    slide
}

/// Whether `g` and `t` still name the same data.
fn still_shared(s: &mut Session, slide: &str) -> bool {
    let (g, t) = (param(s, slide, "g", "option")["data"].clone(), param(s, slide, "t", "data")["data"].clone());
    g.is_string() && g == t
}

#[rstest::rstest]
#[case::waterfall("waterfall")]
#[case::scatter("scatter")]
fn kind_needing_new_columns_on_shared_data_is_refused_and_keeps_the_ref(#[case] kind: &str) {
    let mut s = session();
    let slide = shared_chart_and_table(&mut s);
    let refused = edit(&mut s, &slide, "g", &json!({ "edit": "kind", "kind": kind })).is_err();
    assert_that!((refused, still_shared(&mut s, &slide))).is_equal_to((true, true));
}

#[rstest::rstest]
#[case::line("line")]
#[case::pie("pie")]
fn kind_reading_existing_columns_on_shared_data_keeps_the_ref(#[case] kind: &str) {
    let mut s = session();
    let slide = shared_chart_and_table(&mut s);
    ok(edit(&mut s, &slide, "g", &json!({ "edit": "kind", "kind": kind })));
    assert_that!(still_shared(&mut s, &slide)).is_true();
}

#[test]
fn kind_on_shared_data_refusal_is_localized() {
    let mut s = session();
    let slide = shared_chart_and_table(&mut s);
    let Err(e) = edit(&mut s, &slide, "g", &json!({ "edit": "kind", "kind": "waterfall" })) else {
        panic!("the waterfall was accepted");
    };
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    let expected = fr.tr_with("session-chart-kind-shared", &[("subject", "g".into())]);
    assert_that!((e.kind(), e.localize(&fr)))
        .is_equal_to((oku_slides_session::SessionErrorKind::ChartKindShared, expected));
}
