use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::{
    CATALOG_ID, Component, ComponentKind, Dynamic, ServerBody, ServerMessage, TextVariant, UpdateDataModel, Version,
};

#[test]
fn server_message_json_round_trips() {
    let m = ServerMessage {
        version: Version::V0_9,
        body: ServerBody::UpdateDataModel(UpdateDataModel {
            surface_id: "s-1".into(),
            path: Some("/narration".into()),
            value: Some(serde_json::json!("bonjour")),
        }),
    };
    let back: ServerMessage = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
    assert_that!(back).is_equal_to(&m);
}

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

#[test]
fn catalog_id_v4_is_the_current_urn() {
    assert_that!(CATALOG_ID).is_equal_to("urn:oku-slides:a2ui:catalog:v4");
}

#[test]
fn update_data_model_with_path_serializes_it() {
    let m = ServerMessage::update_data_model("turn-1", "/narration", json!("Je lis la slide"));
    assert_that!(serde_json::to_value(&m).unwrap()).is_equal_to(json!({
        "version": "v0.9",
        "updateDataModel": {"surfaceId": "turn-1", "path": "/narration", "value": "Je lis la slide"}
    }));
}
