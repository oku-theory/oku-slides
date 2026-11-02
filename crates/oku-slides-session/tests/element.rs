//! The `host-element` surface (`element_panel`) and its writes (`element_param`, `element_list`).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

#[allow(dead_code, reason = "aides partagées : chaque binaire de test n'en utilise qu'une partie")]
mod common;

use common::{ok, req, session, with_element};
use oku_slides_core::kinds;
use oku_slides_session::{Session, SessionErrorKind};
use serde_json::{Value, json};
use speculoos::prelude::*;

const KIND: &str = kinds::base::SHAPE;
const PARAM: &str = "shape";
const NEW: &str = "ellipse";

fn shape() -> Value {
    json!({ "id": "e", "kind": KIND, "params": { PARAM: "rect" } })
}

fn panel(s: &mut Session, slide: &str, element: &str) -> Value {
    let text = ok(s.handle(req(json!({ "cmd": "element_panel", "slide": slide, "element": element }))));
    ok(serde_json::from_str(&text))
}

fn set_key(
    s: &mut Session,
    slide: &str,
    element: &str,
    key: &str,
    value: &Value,
) -> oku_slides_session::Result<String> {
    s.handle(req(json!({ "cmd": "element_param", "slide": slide, "element": element, "key": key, "value": value })))
}

fn set(s: &mut Session, slide: &str, value: &Value) -> oku_slides_session::Result<String> {
    set_key(s, slide, "e", PARAM, value)
}

/// The `key` of every field event of a surface.
fn field_keys(surface: &Value) -> Vec<String> {
    let components = surface["messages"][1]["updateComponents"]["components"].as_array().cloned().unwrap_or_default();
    components.iter().filter_map(|c| c["action"]["event"]["context"]["key"].as_str().map(str::to_owned)).collect()
}

/// The param `PARAM` of `e`, read back through the `slide` request.
fn current(s: &mut Session, slide: &str) -> Value {
    let text = ok(s.handle(req(json!({ "cmd": "slide", "id": slide }))));
    let json: Value = ok(serde_json::from_str(&text));
    let blocks = json["slide"]["params"]["blocks"].as_array().cloned().unwrap_or_default();
    blocks.into_iter().find(|b| b["id"] == "e").map_or(Value::Null, |e| e["params"][PARAM].clone())
}

#[test]
fn element_panel_element_is_the_host_element_surface() {
    let mut s = session();
    let slide = with_element(&mut s, shape());
    assert_that!(panel(&mut s, &slide, "e")["id"].clone()).is_equal_to(json!("host-element"));
}

#[test]
fn element_panel_shows_the_current_value() {
    let mut s = session();
    let slide = with_element(&mut s, shape());
    ok(set(&mut s, &slide, &json!(NEW)));
    let surface = panel(&mut s, &slide, "e");
    let model = surface["messages"].as_array().cloned().unwrap_or_default();
    let data = model.iter().find_map(|m| m.get("updateDataModel")).cloned().unwrap_or(Value::Null);
    assert_that!(data["value"]["f0"].clone()).is_equal_to(json!(NEW));
    assert_that!(field_keys(&surface)).is_equal_to(vec![PARAM.to_owned()]);
}

#[test]
fn element_panel_unknown_element_returns_null() {
    let mut s = session();
    let slide = with_element(&mut s, shape());
    assert_that!(panel(&mut s, &slide, "absent")).is_equal_to(Value::Null);
}

#[test]
fn element_panel_chart_has_no_data_field() {
    let mut s = session();
    let chart = json!({ "id": "e", "kind": kinds::base::CHART, "params": { "option": {
        "dataset": { "source": [["M", "A"], ["x", 1]] }, "series": [{ "type": "bar" }] } } });
    let slide = with_element(&mut s, chart);
    let surface = panel(&mut s, &slide, "e");
    assert_that!(surface["id"].clone()).is_equal_to(json!("host-element"));
    assert_that!(field_keys(&surface).contains(&"option".to_owned())).is_false();
}

#[test]
fn element_panel_in_history_returns_null() {
    let mut s = session();
    let slide = with_element(&mut s, shape());
    ok(s.handle(req(json!({ "cmd": "history_open" }))));
    assert_that!(panel(&mut s, &slide, "e")).is_equal_to(Value::Null);
}

#[test]
fn element_param_undo_restores_the_previous_value() {
    let mut s = session();
    let slide = with_element(&mut s, shape());
    ok(set(&mut s, &slide, &json!(NEW)));
    assert_that!(current(&mut s, &slide)).is_equal_to(json!(NEW));
    ok(s.handle(req(json!({ "cmd": "undo" }))));
    assert_that!(current(&mut s, &slide)).is_equal_to(json!("rect"));
}

#[test]
fn element_param_undo_label_is_the_param_label_in_fr() {
    let mut s = session();
    let slide = with_element(&mut s, shape());
    ok(set(&mut s, &slide, &json!(NEW)));
    assert_that!(common::snapshot_json(&mut s)["undo_label"].clone()).is_equal_to(json!("Réglage : Forme"));
}

#[test]
fn element_param_unknown_key_is_a_core_refusal() {
    let mut s = session();
    let slide = with_element(&mut s, shape());
    let r = set_key(&mut s, &slide, "e", "zz", &json!(1));
    assert_that!(r.map_err(|e| e.kind())).is_equal_to(Err(SessionErrorKind::Op));
}

#[test]
fn element_param_bad_value_is_a_core_refusal() {
    let mut s = session();
    let slide = with_element(&mut s, shape());
    let r = set(&mut s, &slide, &json!(12));
    assert_that!(r.map_err(|e| e.kind())).is_equal_to(Err(SessionErrorKind::Op));
}

#[test]
fn element_param_unknown_element_is_a_core_refusal() {
    let mut s = session();
    let slide = with_element(&mut s, shape());
    let r = set_key(&mut s, &slide, "absent", PARAM, &json!(NEW));
    assert_that!(r.map_err(|e| e.kind())).is_equal_to(Err(SessionErrorKind::Op));
}
