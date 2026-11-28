use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::{Action, Component, ComponentKind, DataChoice, DataRef, DataState, Dynamic, QueryView, SeriesSwatch};

fn edit_action() -> Action {
    Action::event("data", [("edit", Dynamic::path("/edit"))])
}

fn data() -> DataRef {
    DataRef { id: "d1".into(), name: "ventes".into(), label: None, users: 2 }
}

fn choices() -> Vec<DataChoice> {
    vec![
        DataChoice { id: "d1".into(), name: "ventes".into(), label: None },
        DataChoice { id: "d2".into(), name: "budget".into(), label: Some("Budget".into()) },
    ]
}

fn chart() -> Component {
    Component::new(
        "data",
        ComponentKind::ChartData {
            kind: Some("bar".into()),
            source: vec![vec![json!("Mois"), json!("Ventes")], vec![json!("Jan"), json!(3)]],
            row_ids: vec!["jan".into()],
            column_ids: vec!["mois".into(), "ventes".into()],
            checkbox_columns: vec![],
            stacked: None,
            series: vec![SeriesSwatch { name: "Ventes".into(), color: Some("#1f4e79".into()) }],
            palette: vec!["#1f4e79".into()],
            data: data(),
            choices: choices(),
            query: None,
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
            row_ids: vec!["r1".into()],
            column_ids: vec!["a".into(), "b".into()],
            header_row: true,
            header_column: false,
            striped: true,
            align: vec!["auto".into(), "end".into()],
            data: data(),
            choices: choices(),
            query: None,
            edit: Dynamic::path("/edit"),
            action: edit_action(),
        },
    )
}

#[test]
fn chart_grid_json_round_trip_is_identity() {
    let json = serde_json::to_value(chart()).unwrap();
    assert_that!(serde_json::from_value::<Component>(json).ok()).is_equal_to(Some(chart()));
}

#[test]
fn chart_grid_json_names_its_component() {
    let json = serde_json::to_value(chart()).unwrap();
    assert_that!(json["component"].clone()).is_equal_to(json!("ChartData"));
}

#[test]
fn chart_grid_json_omits_an_absent_stack() {
    let json = serde_json::to_value(chart()).unwrap();
    assert_that!(json.get("stacked")).is_none();
}

#[test]
fn table_grid_json_round_trip_is_identity() {
    let json = serde_json::to_value(table()).unwrap();
    assert_that!(serde_json::from_value::<Component>(json).ok()).is_equal_to(Some(table()));
}

#[test]
fn chart_grid_agent_allowed_is_false() {
    assert_that!(chart().agent_allowed()).is_false();
}

#[test]
fn table_grid_agent_allowed_is_false() {
    assert_that!(table().agent_allowed()).is_false();
}

#[test]
fn chart_grid_actions_are_its_data_action() {
    assert_that!(chart().kind.actions()).is_equal_to(vec![&edit_action()]);
}

#[test]
fn table_grid_without_action_is_none() {
    assert_that!(table().kind.without_action()).is_none();
}

#[test]
fn chart_grid_has_no_children() {
    assert_that!(chart().kind.children()).is_empty();
}

#[test]
fn table_grid_json_omits_an_absent_label() {
    let json = serde_json::to_value(table()).unwrap();
    assert_that!((json["data"].clone(), json["choices"][0].clone()))
        .is_equal_to((json!({ "id": "d1", "name": "ventes", "users": 2 }), json!({ "id": "d1", "name": "ventes" })));
}

/// A failed query's sheet: no rows, its SQL, its error and its input.
fn query_sheet() -> Component {
    Component::new(
        "root",
        ComponentKind::DataSheet {
            data: data(),
            source: vec![vec![]],
            row_ids: vec![],
            column_ids: vec![],
            types: vec![],
            offset: 0,
            total: 0,
            query: Some(QueryView {
                sql: "SELECT a FROM ventes".into(),
                state: DataState::Failed,
                error: Some("la requête a échoué".into()),
                inputs: vec![DataChoice { id: "d2".into(), name: "ventes".into(), label: None }],
            }),
            error: None,
            edit: Dynamic::path("/edit"),
            action: edit_action(),
        },
    )
}

#[test]
fn query_sheet_json_round_trip_is_identity() {
    let json = serde_json::to_value(query_sheet()).unwrap();
    assert_that!(serde_json::from_value::<Component>(json).ok()).is_equal_to(Some(query_sheet()));
}

#[test]
fn query_sheet_json_names_its_state_and_omits_no_refusal() {
    let json = serde_json::to_value(query_sheet()).unwrap();
    assert_that!((json["query"]["state"].clone(), json.get("error").is_none())).is_equal_to((json!("failed"), true));
}

#[test]
fn chart_grid_json_names_its_query_only_over_a_query() {
    let mut over = chart();
    let view = QueryView { sql: "SELECT 1".into(), state: DataState::Ready, error: None, inputs: vec![] };
    if let ComponentKind::ChartData { query, .. } = &mut over.kind {
        *query = Some(view);
    }
    let (own, json) = (serde_json::to_value(chart()).unwrap(), serde_json::to_value(&over).unwrap());
    assert_that!((own.get("query").is_none(), json["query"]["state"].clone())).is_equal_to((true, json!("ready")));
    assert_that!(serde_json::from_value::<Component>(json).ok()).is_equal_to(Some(over));
}
