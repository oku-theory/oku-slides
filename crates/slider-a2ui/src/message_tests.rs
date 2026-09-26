use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::{Action, CATALOG_ID, Component, ComponentKind, Dynamic, ServerMessage, TextVariant};

fn to_json(msg: &ServerMessage) -> Option<Value> {
    serde_json::to_value(msg).ok()
}

#[test]
fn create_surface_names_the_oku_slides_catalog() {
    assert_that!(to_json(&ServerMessage::create_surface("panel"))).is_some().is_equal_to(json!({
        "version": "v0.9",
        "createSurface": { "surfaceId": "panel", "catalogId": CATALOG_ID }
    }));
}

#[test]
fn catalog_id_names_oku_slides() {
    assert_that!(CATALOG_ID).is_equal_to("urn:oku-slides:a2ui:catalog:panel:v1");
}

#[test]
fn update_components_is_flat_with_component_tag() {
    let msg = ServerMessage::update_components(
        "panel",
        vec![
            Component::new("root", ComponentKind::Column { children: vec!["title".into()] }),
            Component::new(
                "title",
                ComponentKind::Text { text: Dynamic::Literal("Grille".into()), variant: Some(TextVariant::H4) },
            ),
        ],
    );
    assert_that!(to_json(&msg)).is_some().is_equal_to(json!({
        "version": "v0.9",
        "updateComponents": {
            "surfaceId": "panel",
            "components": [
                { "id": "root", "component": "Column", "children": ["title"] },
                { "id": "title", "component": "Text", "text": "Grille", "variant": "h4" }
            ]
        }
    }));
}

#[test]
fn card_has_a_single_child() {
    let msg =
        ServerMessage::update_components("p", vec![Component::new("root", ComponentKind::Card { child: "c".into() })]);
    assert_that!(to_json(&msg).map(|v| v["updateComponents"]["components"][0].clone()))
        .is_some()
        .is_equal_to(json!({ "id": "root", "component": "Card", "child": "c" }));
}

#[test]
fn text_field_binds_value_and_sends_action_with_path_context() {
    let field = Component::new(
        "name",
        ComponentKind::TextField {
            label: Dynamic::Literal("Nom".into()),
            value: Dynamic::path("/template/name"),
            action: Some(Action::event("rename_template", [("value", Dynamic::path("/template/name"))])),
        },
    );
    let msg = ServerMessage::update_components("p", vec![field]);
    assert_that!(to_json(&msg).map(|v| v["updateComponents"]["components"][0].clone())).is_some().is_equal_to(json!({
        "id": "name", "component": "TextField", "label": "Nom", "value": { "path": "/template/name" },
        "action": { "event": { "name": "rename_template", "context": { "value": { "path": "/template/name" } } } }
    }));
}

#[test]
fn stepper_leaves_out_missing_bounds_and_action() {
    let stepper = Component::new(
        "cols",
        ComponentKind::Stepper {
            label: Dynamic::Literal("Colonnes".into()),
            value: Dynamic::path("/grid/columns"),
            min: Some(1),
            max: None,
            action: None,
        },
    );
    let msg = ServerMessage::update_components("p", vec![stepper]);
    assert_that!(to_json(&msg).map(|v| v["updateComponents"]["components"][0].clone())).is_some().is_equal_to(json!({
        "id": "cols", "component": "Stepper", "label": "Colonnes", "value": { "path": "/grid/columns" }, "min": 1
    }));
}

#[test]
fn set_data_model_writes_the_root_without_path() {
    let msg = ServerMessage::set_data_model("panel", json!({ "grid": { "columns": 12 } }));
    assert_that!(to_json(&msg)).is_some().is_equal_to(json!({
        "version": "v0.9",
        "updateDataModel": { "surfaceId": "panel", "value": { "grid": { "columns": 12 } } }
    }));
}

#[test]
fn delete_surface_names_the_surface() {
    assert_that!(to_json(&ServerMessage::delete_surface("panel"))).is_some().is_equal_to(json!({
        "version": "v0.9",
        "deleteSurface": { "surfaceId": "panel" }
    }));
}
