use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::{Action, Component, ComponentKind, Dynamic, SeriesSwatch};

fn edit_action() -> Action {
    Action::event("data", [("edit", Dynamic::path("/edit"))])
}

fn chart() -> Component {
    Component::new(
        "data",
        ComponentKind::ChartData {
            kind: Some("bar".into()),
            source: vec![vec![json!("Mois"), json!("Ventes")], vec![json!("Jan"), json!(3)]],
            checkbox_columns: vec![],
            stacked: None,
            series: vec![SeriesSwatch { name: "Ventes".into(), color: Some("#1f4e79".into()) }],
            palette: vec!["#1f4e79".into()],
            edit: Dynamic::path("/edit"),
            action: edit_action(),
        },
    )
}

fn table() -> Component {
    Component::new(
        "data",
        ComponentKind::TableData {
            source: vec![vec![json!("A"), json!("B")], vec![json!(1), Value::Null]],
            header_row: true,
            header_column: false,
            striped: true,
            align: vec!["auto".into(), "end".into()],
            edit: Dynamic::path("/edit"),
            action: edit_action(),
        },
    )
}

#[test]
fn chart_data_json_round_trip_is_identity() {
    let json = serde_json::to_value(chart()).unwrap();
    assert_that!(serde_json::from_value::<Component>(json).ok()).is_equal_to(Some(chart()));
}

#[test]
fn chart_data_json_names_the_component_chart_data() {
    let json = serde_json::to_value(chart()).unwrap();
    assert_that!(json["component"].clone()).is_equal_to(json!("ChartData"));
}

#[test]
fn chart_data_json_omits_an_absent_stack() {
    let json = serde_json::to_value(chart()).unwrap();
    assert_that!(json.get("stacked")).is_none();
}

#[test]
fn table_data_json_round_trip_is_identity() {
    let json = serde_json::to_value(table()).unwrap();
    assert_that!(serde_json::from_value::<Component>(json).ok()).is_equal_to(Some(table()));
}

#[test]
fn chart_data_agent_allowed_is_false() {
    assert_that!(chart().agent_allowed()).is_false();
}

#[test]
fn table_data_agent_allowed_is_false() {
    assert_that!(table().agent_allowed()).is_false();
}

#[test]
fn chart_data_actions_are_its_data_action() {
    assert_that!(chart().kind.actions()).is_equal_to(vec![&edit_action()]);
}

#[test]
fn table_data_without_action_is_none() {
    assert_that!(table().kind.without_action()).is_none();
}

#[test]
fn chart_data_has_no_children() {
    assert_that!(chart().kind.children()).is_empty();
}
