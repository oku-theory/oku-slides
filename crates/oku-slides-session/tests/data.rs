//! Données d'un graphique ou d'un tableau au dock : la surface `host-data` (`data_panel`)
//! et ses modifications (`data_edit`).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

#[allow(dead_code, reason = "aides partagées : chaque binaire de test n'en utilise qu'une partie")]
mod common;

use common::{req, session};
use oku_slides_core::kinds;
use oku_slides_session::Session;
use serde_json::{Value, json};
use speculoos::prelude::*;

fn ok<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("échec inattendu : {e:?}"))
}

/// Ajoute l'élément `element` à la slide 2 d'un deck neuf ; rend l'id de la slide.
fn with_element(s: &mut Session, element: Value) -> String {
    ok(s.new_deck());
    let slide = s.snapshot().slides[1].id.to_string();
    let mut add = json!({ "op": "add_element", "slot": { "owner": { "slide": slide }, "param": "blocks" } });
    add["element"] = element;
    ok(s.handle(req(json!({ "cmd": "apply", "label": "ajout", "ops": [add] }))));
    slide
}

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

/// La surface `host-data` de `g`, ou `null`.
fn panel(s: &mut Session, slide: &str, element: &str) -> Value {
    let text = ok(s.handle(req(json!({ "cmd": "data_panel", "slide": slide, "element": element }))));
    ok(serde_json::from_str(&text))
}

/// Le composant de données de la surface (`ChartData` ou `TableData`).
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

/// Le paramètre de données de `g` (`option` d'un graphique, `data` d'un tableau).
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

#[test]
fn data_edit_cells_sets_the_cell() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    ok(edit(
        &mut s,
        &slide,
        json!({ "edit": "cells", "change": { "op": "set_cell", "row": 1, "col": 1, "value": 9 } }),
    ));
    assert_that!(param(&mut s, &slide, "option")["dataset"]["source"][1][1].clone()).is_equal_to(json!(9.0));
}

#[test]
fn data_edit_cells_twice_on_one_cell_is_one_undo_step() {
    let mut s = session();
    let slide = with_element(&mut s, two_bars());
    for v in [8, 9] {
        ok(edit(
            &mut s,
            &slide,
            json!({ "edit": "cells", "change": { "op": "set_cell", "row": 1, "col": 1, "value": v } }),
        ));
    }
    undo(&mut s);
    assert_that!(param(&mut s, &slide, "option")["dataset"]["source"][1][1].clone()).is_equal_to(json!(1.0));
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
    ok(edit(&mut s, &slide, json!({ "edit": "show_as", "view": "table" })));
    assert_that!(param(&mut s, &slide, "data")["dataset"]["source"][0].clone()).is_equal_to(json!(["M", "A", "B"]));
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
