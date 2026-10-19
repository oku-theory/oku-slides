//! Tableaux dans une session : conversion depuis et vers un graphique, édition, annulation.

#[allow(dead_code, reason = "aides partagées : chaque binaire de test n'en utilise qu'une partie")]
mod common;

use common::{req, session, snapshot_json};
use oku_slides_core::kinds;
use oku_slides_session::{Session, SessionErrorKind};
use serde_json::{Value, json};
use speculoos::prelude::*;

fn ok<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("échec inattendu : {e:?}"))
}

/// Une session sur un deck neuf, avec le graphique `g` sur la slide 2 ; rend l'id de la slide.
fn with_chart(s: &mut Session) -> String {
    ok(s.new_deck());
    let slide = s.snapshot().slides[1].id.to_string();
    ok(s.handle(req(json!({ "cmd": "apply", "label": "graphique", "ops": [{
        "op": "add_element", "slot": { "owner": { "slide": slide }, "param": "blocks" },
        "element": { "id": "g", "kind": kinds::base::CHART, "params": { "option": {
            "dataset": { "source": [["M", "A", "B"], ["x", 1, 2], ["y", 3, 4]] },
            "xAxis": { "type": "category" }, "series": [{ "type": "bar" }, { "type": "bar" }] } } } }] }))));
    slide
}

fn slide_json(s: &mut Session, slide: &str) -> Value {
    let text = ok(s.handle(req(json!({ "cmd": "slide", "id": slide }))));
    ok(serde_json::from_str(&text))
}

fn element(s: &mut Session, slide: &str) -> Value {
    // Le deck neuf porte déjà un bloc de texte : on cherche `g` par son id.
    let json = slide_json(s, slide);
    let blocks = json["slide"]["params"]["blocks"].as_array().cloned().unwrap_or_default();
    blocks.into_iter().find(|b| b["id"] == "g").unwrap_or(Value::Null)
}

fn show_as(s: &mut Session, slide: &str, view: &str) -> Result<String, oku_slides_session::SessionError> {
    s.handle(req(json!({ "cmd": "show_as", "label": "vue", "slide": slide, "element": "g", "view": view })))
}

#[test]
fn show_as_table_keeps_id_and_dataset() {
    let mut s = session();
    let slide = with_chart(&mut s);
    ok(show_as(&mut s, &slide, "table"));
    let e = element(&mut s, &slide);
    assert_that!(e["kind"]).is_equal_to(json!(kinds::base::TABLE));
    assert_that!(e["params"]["data"]["dataset"]["source"][2]).is_equal_to(json!(["y", 3.0, 4.0]));
}

#[test]
fn show_as_table_makes_a_base_table_with_the_same_dataset() {
    let mut s = session();
    let slide = with_chart(&mut s);
    let before = element(&mut s, &slide)["params"]["option"]["dataset"]["source"].clone();
    ok(show_as(&mut s, &slide, "table"));
    let after = element(&mut s, &slide);
    assert_that!((after["kind"].as_str(), after["params"]["data"]["dataset"]["source"].clone()))
        .is_equal_to((Some(kinds::base::TABLE), before));
}

#[test]
fn show_as_table_then_undo_restores_the_chart_in_one_step() {
    let mut s = session();
    let slide = with_chart(&mut s);
    let before = slide_json(&mut s, &slide);
    ok(show_as(&mut s, &slide, "table"));
    ok(s.handle(req(json!({ "cmd": "undo" }))));
    assert_that!(slide_json(&mut s, &slide)).is_equal_to(before);
}

#[test]
fn show_as_chart_from_a_table_gets_one_series_per_column() {
    let mut s = session();
    let slide = with_chart(&mut s);
    ok(show_as(&mut s, &slide, "table"));
    ok(show_as(&mut s, &slide, "chart"));
    let e = element(&mut s, &slide);
    assert_that!(e["kind"]).is_equal_to(json!(kinds::base::CHART));
    assert_that!(e["params"]["option"]["series"].as_array().map(Vec::len)).is_equal_to(Some(2));
}

#[test]
fn show_as_table_on_a_table_is_no_chart() {
    let mut s = session();
    let slide = with_chart(&mut s);
    ok(show_as(&mut s, &slide, "table"));
    let err = show_as(&mut s, &slide, "table").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NoChart);
}

#[test]
fn show_as_chart_on_a_chart_is_no_table() {
    let mut s = session();
    let slide = with_chart(&mut s);
    let err = show_as(&mut s, &slide, "chart").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NoTable);
}

#[test]
fn show_as_table_on_a_chart_beyond_table_limits_is_refused() {
    let mut s = session();
    ok(s.new_deck());
    let slide = s.snapshot().slides[1].id.to_string();
    // 21 colonnes : une de plus que MAX_TABLE_COLUMNS.
    let header: Vec<Value> = (0..21).map(|i| json!(format!("c{i}"))).collect();
    let row: Vec<Value> = (0..21).map(|i| json!(i)).collect();
    ok(s.handle(req(json!({ "cmd": "apply", "label": "graphique large", "ops": [{
        "op": "add_element", "slot": { "owner": { "slide": slide }, "param": "blocks" },
        "element": { "id": "g", "kind": kinds::base::CHART, "params": { "option": {
            "dataset": { "source": [header, row] }, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } } }] }))));
    let before = slide_json(&mut s, &slide);
    assert_that!(show_as(&mut s, &slide, "table").is_err()).is_true();
    assert_that!(slide_json(&mut s, &slide)).is_equal_to(before);
}

#[test]
fn set_table_option_then_undo_restores_the_table() {
    let mut s = session();
    let slide = with_chart(&mut s);
    ok(show_as(&mut s, &slide, "table"));
    let before = element(&mut s, &slide);
    ok(s.handle(req(json!({ "cmd": "apply", "label": "zébrage", "ops": [{ "op": "set_table_option",
        "target": { "slide": slide, "element": "g" }, "param": "data", "patch": { "striped": true } }] }))));
    assert_that!(element(&mut s, &slide)["params"]["data"]["striped"]).is_equal_to(json!(true));
    ok(s.handle(req(json!({ "cmd": "undo" }))));
    assert_that!(element(&mut s, &slide)).is_equal_to(before);
}

#[test]
fn set_data_on_a_table_then_undo_restores_the_cell() {
    let mut s = session();
    let slide = with_chart(&mut s);
    ok(show_as(&mut s, &slide, "table"));
    ok(s.handle(req(json!({ "cmd": "apply", "label": "cellule", "ops": [{ "op": "set_data",
        "target": { "slide": slide, "element": "g" }, "param": "data",
        "change": { "op": "set_cell", "row": 1, "col": 1, "value": 9 } }] }))));
    assert_that!(element(&mut s, &slide)["params"]["data"]["dataset"]["source"][1][1]).is_equal_to(json!(9.0));
    ok(s.handle(req(json!({ "cmd": "undo" }))));
    assert_that!(element(&mut s, &slide)["params"]["data"]["dataset"]["source"][1][1]).is_equal_to(json!(1.0));
}

#[test]
fn snapshot_after_show_as_offers_undo() {
    let mut s = session();
    let slide = with_chart(&mut s);
    ok(show_as(&mut s, &slide, "table"));
    assert_that!(snapshot_json(&mut s)["undo_label"]).is_equal_to(json!("vue"));
}

#[test]
fn show_as_round_trip_keeps_the_chart_type() {
    let mut s = session();
    ok(s.new_deck());
    let slide = s.snapshot().slides[1].id.to_string();
    ok(s.handle(req(json!({ "cmd": "apply", "label": "graphique", "ops": [{
        "op": "add_element", "slot": { "owner": { "slide": slide }, "param": "blocks" },
        "element": { "id": "g", "kind": kinds::base::CHART, "params": { "option": {
            "dataset": { "source": [["M", "A", "B"], ["x", 1, 2], ["y", 3, 4]] },
            "xAxis": { "type": "category" }, "series": [{ "type": "line" }, { "type": "line" }] } } } }] }))));
    ok(show_as(&mut s, &slide, "table"));
    ok(show_as(&mut s, &slide, "chart"));
    let e = element(&mut s, &slide);
    assert_that!(e["params"]["option"]["series"][0]["type"].clone()).is_equal_to(json!("line"));
}

#[test]
fn show_as_round_trip_after_removing_a_middle_column_keeps_each_series_color() {
    let mut s = session();
    ok(s.new_deck());
    let slide = s.snapshot().slides[1].id.to_string();
    ok(s.handle(req(json!({ "cmd": "apply", "label": "graphique", "ops": [{
        "op": "add_element", "slot": { "owner": { "slide": slide }, "param": "blocks" },
        "element": { "id": "g", "kind": kinds::base::CHART, "params": { "option": {
            "dataset": { "source": [["M", "A", "B", "C"], ["x", 1, 2, 3], ["y", 4, 5, 6]] },
            "xAxis": { "type": "category" }, "series": [
                { "type": "line", "itemStyle": { "color": "#ff0000" } },
                { "type": "line", "itemStyle": { "color": "#00ff00" } },
                { "type": "line", "itemStyle": { "color": "#0000ff" } }] } } } }] }))));
    ok(show_as(&mut s, &slide, "table"));
    ok(s.handle(req(json!({ "cmd": "apply", "label": "colonne", "ops": [{ "op": "set_data",
        "target": { "slide": slide, "element": "g" }, "param": "data",
        "change": { "op": "remove_column", "at": 2 } }] }))));
    ok(show_as(&mut s, &slide, "chart"));
    let series = element(&mut s, &slide)["params"]["option"]["series"].as_array().cloned().unwrap_or_default();
    let colors: Vec<_> = series.iter().map(|s| s["itemStyle"]["color"].clone()).collect();
    assert_that!(colors).is_equal_to(vec![json!("#ff0000"), json!("#0000ff")]);
}
