//! The data of a chart or a table in the dock: the `host-data` surface (`data_panel`) and its
//! edits (`data_edit`), which name the deck data's rows and columns by id.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

#[allow(dead_code, reason = "shared helpers: each test binary uses only some of them")]
mod common;

use common::{ok, req, session, with_element};
use oku_slides_core::kinds;
use oku_slides_session::Session;
use serde_json::{Value, json};
use speculoos::prelude::*;

fn chart(series: Value) -> Value {
    let mut element = json!({ "id": "g", "kind": kinds::base::CHART, "params": { "option": {
        "dataset": { "source": [["M", "A", "B"], ["x", 1, 2], ["y", 3, 4]] },
        "xAxis": { "type": "category" } } } });
    element["params"]["option"]["series"] = series;
    element
}

fn two_bars() -> Value {
    chart(json!([{ "type": "bar" }, { "type": "bar", "itemStyle": { "color": "#ABC" } }]))
}

fn table() -> Value {
    json!({ "id": "g", "kind": kinds::base::TABLE, "params": { "data": {
        "dataset": { "source": [["A", "B"], [1, 2]] }, "striped": true, "align": ["auto", "end"] } } })
}

/// The `host-data` surface of `element`, or `null`.
fn panel(s: &mut Session, slide: &str, element: &str) -> Value {
    let text = ok(s.handle(req(json!({ "cmd": "data_panel", "slide": slide, "element": element }))));
    ok(serde_json::from_str(&text))
}

/// The data component of the surface (`ChartData` or `TableData`).
fn data_component(surface: &Value) -> Value {
    let messages = surface["messages"].as_array().cloned().unwrap_or_default();
    let components =
        messages.iter().find_map(|m| m["updateComponents"]["components"].as_array().cloned()).unwrap_or_default();
    components
        .into_iter()
        .find(|c| c["component"] == "ChartData" || c["component"] == "TableData")
        .unwrap_or(Value::Null)
}

#[test]
fn data_panel_chart_is_the_host_data_surface() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    assert_that!(panel(&mut s, &slide, "g")["id"].clone()).is_equal_to(json!("host-data"));
}

#[test]
fn data_panel_chart_carries_kind_and_dataset() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    let c = data_component(&panel(&mut s, &slide, "g"));
    assert_that!((c["kind"].clone(), c["source"][2].clone())).is_equal_to((json!("bar"), json!(["y", 3.0, 4.0])));
}

#[test]
fn data_panel_two_bar_series_are_stackable_and_not_stacked() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    assert_that!(data_component(&panel(&mut s, &slide, "g"))["stacked"].clone()).is_equal_to(json!(false));
}

#[test]
fn data_panel_one_series_is_not_stackable() {
    let mut s = session();
    let slide = with_element(&mut s, chart(json!([{ "type": "bar" }])));
    assert_that!(data_component(&panel(&mut s, &slide, "g")).get("stacked").cloned()).is_none();
}

#[test]
fn data_panel_series_are_named_by_header_with_normalized_colors() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    assert_that!(data_component(&panel(&mut s, &slide, "g"))["series"].clone())
        .is_equal_to(json!([{ "name": "A" }, { "name": "B", "color": "#aabbcc" }]));
}

#[test]
fn data_panel_waterfall_edits_its_total_column_as_checkboxes() {
    let mut s = session();
    let slide = with_element(
        &mut s,
        json!({ "id": "g", "kind": kinds::base::CHART, "params": { "option": {
        "dataset": { "source": [["M", "V", "Total"], ["x", 1, false], ["T", 0, true]] },
        "xAxis": { "type": "category" }, "series": [{ "type": "waterfall" }] } } }),
    );
    assert_that!(data_component(&panel(&mut s, &slide, "g"))["checkbox_columns"].clone()).is_equal_to(json!([2]));
}

#[test]
fn data_panel_table_carries_flags_and_alignment() {
    let mut s = session();
    let slide = with_element(&mut s, table());
    let c = data_component(&panel(&mut s, &slide, "g"));
    assert_that!((c["component"].clone(), c["striped"].clone(), c["header_row"].clone(), c["align"].clone()))
        .is_equal_to((json!("TableData"), json!(true), json!(true), json!(["auto", "end"])));
}

#[test]
fn data_panel_text_element_is_null() {
    let mut s = session();
    let slide = with_element(&mut s, json!({ "id": "g", "kind": kinds::base::TEXT, "params": { "body": [] } }));
    assert_that!(panel(&mut s, &slide, "g")).is_equal_to(Value::Null);
}

#[test]
fn data_panel_missing_element_is_null() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    assert_that!(panel(&mut s, &slide, "absent")).is_equal_to(Value::Null);
}

fn edit(s: &mut Session, slide: &str, edit: Value) -> Result<String, oku_slides_session::SessionError> {
    let mut request = json!({ "cmd": "data_edit", "slide": slide, "element": "g" });
    request["edit"] = edit;
    s.handle(req(request))
}

/// The data param of `g` (`option` of a chart, `data` of a table).
fn param(s: &mut Session, slide: &str, name: &str) -> Value {
    let text = ok(s.handle(req(json!({ "cmd": "slide", "id": slide }))));
    let json: Value = ok(serde_json::from_str(&text));
    let blocks = json["slide"]["params"]["blocks"].as_array().cloned().unwrap_or_default();
    blocks.into_iter().find(|b| b["id"] == "g").map_or(Value::Null, |g| g["params"][name].clone())
}

fn undo_label(s: &mut Session) -> Value {
    common::snapshot_json(s)["undo_label"].clone()
}

fn undo(s: &mut Session) {
    ok(s.handle(req(json!({ "cmd": "undo" }))));
}

/// The pane of `g`: its source, row ids and column ids.
fn pane(s: &mut Session, slide: &str) -> (Value, Vec<String>, Vec<String>) {
    let c = data_component(&panel(s, slide, "g"));
    let strings = |v: &Value| -> Vec<String> {
        v.as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect()
    };
    (c["source"].clone(), strings(&c["row_ids"]), strings(&c["column_ids"]))
}

/// A `cells` edit of `g` setting cell (`row`, `column`) of the pane, by its ids, to `value`.
fn set_cell(s: &mut Session, slide: &str, row: usize, column: usize, value: &Value) {
    let (_, rows, columns) = pane(s, slide);
    let change = json!({ "op": "set_cell", "row": rows[row], "column": columns[column], "value": value });
    ok(edit(s, slide, json!({ "edit": "cells", "change": change })));
}

/// The data id `g` names in its `param`.
fn data_of(s: &mut Session, slide: &str, param_name: &str) -> String {
    param(s, slide, param_name)["data"].as_str().unwrap_or_default().to_owned()
}

#[test]
fn data_edit_cells_sets_the_cell() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    set_cell(&mut s, &slide, 0, 1, &json!(9));
    assert_that!(pane(&mut s, &slide).0[1][1].clone()).is_equal_to(json!(9.0));
}

#[test]
fn data_edit_cells_twice_on_one_cell_is_one_undo_step() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    for v in [8, 9] {
        set_cell(&mut s, &slide, 0, 1, &json!(v));
    }
    undo(&mut s);
    assert_that!(pane(&mut s, &slide).0[1][1].clone()).is_equal_to(json!(1.0));
}

#[test]
fn cells_merge_key_uses_data_and_row_ids() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    set_cell(&mut s, &slide, 0, 1, &json!(8));
    set_cell(&mut s, &slide, 1, 1, &json!(9));
    undo(&mut s);
    // Another row is another merge key: the undo takes back only the second edit.
    let source = pane(&mut s, &slide).0;
    assert_that!((source[1][1].clone(), source[2][1].clone())).is_equal_to((json!(8.0), json!(3.0)));
}

#[test]
fn cells_edits_of_one_cell_from_a_chart_and_a_table_sharing_its_data_are_one_undo_step() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    let data = data_of(&mut s, &slide, "option");
    let table = json!({ "op": "add_element", "slot": { "owner": { "slide": slide }, "param": "blocks" },
        "element": { "id": "t", "kind": kinds::base::TABLE, "params": { "data": { "data": data } } } });
    ok(s.handle(req(json!({ "cmd": "apply", "label": "table", "ops": [table] }))));
    set_cell(&mut s, &slide, 0, 1, &json!(8));
    let table_pane = data_component(&panel(&mut s, &slide, "t"));
    let change = json!({ "op": "set_cell", "row": table_pane["row_ids"][0], "column": table_pane["column_ids"][1],
        "value": 9 });
    let request = json!({ "cmd": "data_edit", "slide": slide, "element": "t",
        "edit": { "edit": "cells", "change": change } });
    ok(s.handle(req(request)));
    // The key names the data, not the element: one cell edited from two views is one step.
    undo(&mut s);
    assert_that!(pane(&mut s, &slide).0[1][1].clone()).is_equal_to(json!(1.0));
}

#[test]
fn header_edit_renames_column() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    let (_, _, columns) = pane(&mut s, &slide);
    let change = json!({ "op": "rename_column", "column": columns[1], "name": "Alpha" });
    ok(edit(&mut s, &slide, json!({ "edit": "cells", "change": change })));
    let (source, _, after) = pane(&mut s, &slide);
    assert_that!((source[0].clone(), after)).is_equal_to((json!(["M", "Alpha", "B"]), columns));
}

#[test]
fn cells_edit_on_row_removed_by_peer_is_refused_without_panic() {
    let dir = common::data_dir();
    let (path, other) = (dir.join("deck.oku-slides.pdf"), dir.join("peer.oku-slides.pdf"));
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    ok(s.save(Some(&path)));
    let (_, rows, columns) = pane(&mut s, &slide);
    // A peer opens the saved copy, removes the row this pane still shows, and saves its copy.
    let mut peer = session();
    ok(peer.open(&path));
    let data = data_of(&mut peer, &slide, "option");
    let remove = json!({ "op": "edit_data", "data": data, "change": { "op": "remove_row", "row": rows[0] } });
    ok(peer.handle(req(json!({ "cmd": "apply", "label": "peer", "ops": [remove] }))));
    ok(peer.save(Some(&other)));
    ok(s.merge_file(&other));
    let change = json!({ "op": "set_cell", "row": rows[0], "column": columns[1], "value": 9 });
    let Err(e) = edit(&mut s, &slide, json!({ "edit": "cells", "change": change })) else {
        panic!("the edit of a removed row was accepted");
    };
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    let gone = fr.tr_with("data-no-row", &[("row", rows[0].clone().into())]);
    assert_that!(e.kind()).is_equal_to(oku_slides_session::SessionErrorKind::Op);
    assert_that!(oku_slides_session::Localize::localize(&e, &fr).as_str()).contains(gone.as_str());
}

#[test]
fn cells_insert_column_adds_its_series() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    let (_, _, columns) = pane(&mut s, &slide);
    let change = json!({ "op": "insert_column", "after": columns[2], "name": "C", "cells": {} });
    ok(edit(&mut s, &slide, json!({ "edit": "cells", "change": change })));
    assert_that!(param(&mut s, &slide, "option")["series"].as_array().map(Vec::len)).is_equal_to(Some(3));
}

#[test]
fn edit_cell_from_table_panel_updates_chart_sharing_data() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    let data = data_of(&mut s, &slide, "option");
    let table = json!({ "op": "add_element", "slot": { "owner": { "slide": slide }, "param": "blocks" },
        "element": { "id": "t", "kind": kinds::base::TABLE, "params": { "data": { "data": data } } } });
    ok(s.handle(req(json!({ "cmd": "apply", "label": "table", "ops": [table] }))));
    let table_pane = data_component(&panel(&mut s, &slide, "t"));
    let change =
        json!({ "op": "set_cell", "row": table_pane["row_ids"][0], "column": table_pane["column_ids"][1], "value": 7 });
    let request =
        json!({ "cmd": "data_edit", "slide": slide, "element": "t", "edit": { "edit": "cells", "change": change } });
    ok(s.handle(req(request)));
    assert_that!(pane(&mut s, &slide).0[1][1].clone()).is_equal_to(json!(7.0));
}

#[test]
fn data_pane_shows_first_99_rows() {
    let mut s = session();
    let rows: Vec<Value> = (0..150).map(|i| json!([format!("r{i}"), i])).collect();
    let mut source = vec![json!(["M", "A"])];
    source.extend(rows);
    let slide = with_element(
        &mut s,
        json!({ "id": "g", "kind": kinds::base::CHART, "params": { "option": {
            "dataset": { "source": source }, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } }),
    );
    let (source, rows, _) = pane(&mut s, &slide);
    assert_that!((source.as_array().map(Vec::len), rows.len())).is_equal_to((Some(100), 99));
}

#[test]
fn data_edit_kind_converts_the_chart() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    ok(edit(&mut s, &slide, json!({ "edit": "kind", "kind": "line" })));
    assert_that!(param(&mut s, &slide, "option")["series"][0]["type"].clone()).is_equal_to(json!("line"));
}

#[test]
fn data_edit_stack_puts_every_series_in_one_group() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    ok(edit(&mut s, &slide, json!({ "edit": "stack", "stacked": true })));
    let series = param(&mut s, &slide, "option")["series"].clone();
    assert_that!((series[0]["stack"].clone(), series[1]["stack"].clone()))
        .is_equal_to((json!("total"), json!("total")));
}

#[test]
fn data_edit_unstack_keeps_the_series_colour() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    ok(edit(&mut s, &slide, json!({ "edit": "stack", "stacked": true })));
    ok(edit(&mut s, &slide, json!({ "edit": "stack", "stacked": false })));
    let series = param(&mut s, &slide, "option")["series"].clone();
    assert_that!((series[1].get("stack").cloned(), series[1]["itemStyle"]["color"].clone()))
        .is_equal_to((None, json!("#ABC")));
}

#[test]
fn data_edit_color_sets_then_clears_a_series_colour() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    ok(edit(&mut s, &slide, json!({ "edit": "color", "index": 0, "color": "#112233" })));
    let set = param(&mut s, &slide, "option")["series"][0]["itemStyle"]["color"].clone();
    ok(edit(&mut s, &slide, json!({ "edit": "color", "index": 0, "color": null })));
    let cleared = param(&mut s, &slide, "option")["series"][0].get("itemStyle").cloned();
    assert_that!((set, cleared)).is_equal_to((json!("#112233"), None));
}

#[test]
fn data_edit_color_not_hex_is_refused() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    assert_that!(edit(&mut s, &slide, json!({ "edit": "color", "index": 0, "color": "red" }))).is_err();
}

#[test]
fn data_edit_flag_sets_the_table_option() {
    let mut s = session();
    let slide = with_element(&mut s, table());
    ok(edit(&mut s, &slide, json!({ "edit": "flag", "flag": "header-column", "on": true })));
    assert_that!(param(&mut s, &slide, "data")["header-column"].clone()).is_equal_to(json!(true));
}

#[test]
fn data_edit_align_back_to_auto_trims_the_list() {
    let mut s = session();
    let slide = with_element(&mut s, table());
    ok(edit(&mut s, &slide, json!({ "edit": "align", "column": 1, "align": "auto" })));
    assert_that!(param(&mut s, &slide, "data").get("align").cloned()).is_none();
}

#[test]
fn data_edit_show_as_table_turns_the_chart_into_a_table() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    let data = data_of(&mut s, &slide, "option");
    ok(edit(&mut s, &slide, json!({ "edit": "show_as", "view": "table" })));
    let c = data_component(&panel(&mut s, &slide, "g"));
    assert_that!((c["component"].clone(), c["source"][0].clone(), data_of(&mut s, &slide, "data"))).is_equal_to((
        json!("TableData"),
        json!(["M", "A", "B"]),
        data,
    ));
}

#[test]
fn data_edit_labels_the_undo_step_in_the_user_language() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    ok(edit(&mut s, &slide, json!({ "edit": "stack", "stacked": true })));
    let expected =
        oku_slides_i18n::Localizer::new(Some("fr")).tr_with("session-undo-chart-stack", &[("stacked", "yes".into())]);
    assert_that!(undo_label(&mut s)).is_equal_to(json!(expected));
}

#[test]
fn data_edit_unknown_edit_is_refused() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    let request = json!({ "cmd": "data_edit", "slide": slide, "element": "g", "edit": { "edit": "explode" } });
    assert_that!(serde_json::from_value::<oku_slides_session::Request>(request)).is_err();
}

#[test]
fn data_edit_on_a_text_element_is_refused() {
    let mut s = session();
    let slide = with_element(&mut s, json!({ "id": "g", "kind": kinds::base::TEXT, "params": { "body": [] } }));
    assert_that!(edit(&mut s, &slide, json!({ "edit": "stack", "stacked": true }))).is_err();
}

#[test]
fn cells_remove_column_drops_it_from_the_projection_and_its_series() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    let (_, _, columns) = pane(&mut s, &slide);
    let change = json!({ "op": "remove_column", "column": columns[1] });
    ok(edit(&mut s, &slide, json!({ "edit": "cells", "change": change })));
    let option = param(&mut s, &slide, "option");
    assert_that!((option["series"].as_array().map(Vec::len), option["columns"].as_array().map(Vec::len)))
        .is_equal_to((Some(1), Some(2)));
    assert_that!(pane(&mut s, &slide).2).is_equal_to(vec![columns[0].clone(), columns[2].clone()]);
}
