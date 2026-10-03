use oku_slides_i18n::Localizer;
use serde_json::json;
use speculoos::prelude::*;

use crate::agent::{AskConfirm, Surface, forms};

/// Une surface quelconque avec un modèle de données racine (vide).
fn surface() -> Surface {
    forms::confirm("show-5", &AskConfirm { message: "Ok ?".into(), danger: false }, &Localizer::new(Some("fr")))
}

#[test]
fn apply_patch_replaces_the_root_data_model_and_keeps_components() {
    let mut s = surface();
    let n = s.messages.len();
    s.apply_patch(&[oku_slides_a2ui::ServerMessage::set_data_model("show-5", json!({"f0": 3}))]);
    assert_that!(s.messages.len()).is_equal_to(n);
    let json = serde_json::to_value(&s).unwrap();
    let model = json["messages"].as_array().unwrap().iter().find_map(|m| m.get("updateDataModel")).unwrap();
    assert_that!(model["value"].clone()).is_equal_to(json!({"f0": 3}));
}

#[test]
fn apply_patch_without_value_removes_the_key() {
    let mut s = surface();
    s.apply_patch(&[oku_slides_a2ui::ServerMessage::set_data_model("show-5", json!({"a": 1, "b": 2}))]);
    let delete = oku_slides_a2ui::ServerMessage {
        version: oku_slides_a2ui::Version::V0_9,
        body: oku_slides_a2ui::ServerBody::UpdateDataModel(oku_slides_a2ui::UpdateDataModel {
            surface_id: "show-5".into(),
            path: Some("/a".into()),
            value: None,
        }),
    };
    s.apply_patch(&[delete]);
    let json = serde_json::to_value(&s).unwrap();
    let model = json["messages"].as_array().unwrap().iter().find_map(|m| m.get("updateDataModel")).unwrap();
    assert_that!(model["value"].clone()).is_equal_to(json!({"b": 2}));
}

#[test]
fn apply_patch_decodes_escaped_pointer_keys() {
    let mut s = surface();
    s.apply_patch(&[oku_slides_a2ui::ServerMessage::update_data_model("show-5", "/a~1b", json!(1))]);
    let json = serde_json::to_value(&s).unwrap();
    let model = json["messages"].as_array().unwrap().iter().find_map(|m| m.get("updateDataModel")).unwrap();
    assert_that!(model["value"]["a/b"].clone()).is_equal_to(json!(1));
}

#[test]
fn apply_patch_root_replacement_drops_earlier_nested_writes() {
    let mut s = surface();
    s.apply_patch(&[oku_slides_a2ui::ServerMessage::update_data_model("show-5", "/deep/x", json!(1))]);
    s.apply_patch(&[oku_slides_a2ui::ServerMessage::set_data_model("show-5", json!({"f0": 3}))]);
    let nested = s
        .messages
        .iter()
        .filter(|m| matches!(&m.body, oku_slides_a2ui::ServerBody::UpdateDataModel(u) if u.path.is_some()))
        .count();
    assert_that!(nested).is_equal_to(0);
}

#[test]
fn apply_patch_writes_a_pointer_into_the_root_data_model() {
    let mut s = surface();
    s.apply_patch(&[oku_slides_a2ui::ServerMessage::update_data_model("show-5", "/f2", json!("#000000"))]);
    let json = serde_json::to_value(&s).unwrap();
    let model = json["messages"].as_array().unwrap().iter().find_map(|m| m.get("updateDataModel")).unwrap();
    assert_that!(model["value"]["f2"].clone()).is_equal_to(json!("#000000"));
}

fn params() -> Surface {
    let spec = crate::agent::ShowParams {
        title: "Réglages".into(),
        slide: "s-4".into(),
        fields: vec![crate::agent::ParamField {
            label: "Colonnes".into(),
            kind: crate::agent::FieldKind::Stepper,
            value: json!(2),
            op_template: json!({ "op": "set_style", "slide": "s-4", "value": "$value" }),
            min: None,
            max: None,
            step: None,
            options: vec![],
        }],
    };
    forms::params("show-5", &spec, &[], &Localizer::new(Some("fr")))
}

#[test]
fn without_actions_on_params_removes_buttons_and_actions() {
    let json = serde_json::to_string(&params().without_actions()).unwrap();
    assert_that!(json).does_not_contain("\"action\"");
    assert_that!(json).does_not_contain("\"close\"");
}

#[test]
fn without_actions_drops_a_card_whose_only_child_was_a_button() {
    let s = forms::confirm("c", &AskConfirm { message: "Ok ?".into(), danger: false }, &Localizer::new(Some("fr")));
    let mut s = s;
    // Une carte autour d'un seul bouton, sous la racine.
    if let oku_slides_a2ui::ServerBody::UpdateComponents(u) = &mut s.messages[1].body {
        u.components = vec![
            oku_slides_a2ui::Component::column("root", &["card"]),
            oku_slides_a2ui::Component::card("card", "b"),
            oku_slides_a2ui::Component::button("b", "Ok", false, oku_slides_a2ui::Action::named("suggest")),
        ];
    }
    let json = serde_json::to_value(s.without_actions()).unwrap();
    let ids: Vec<_> = json["messages"][1]["updateComponents"]["components"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].clone())
        .collect();
    assert_that!(ids).is_equal_to(vec![json!("root")]);
    assert_that!(json["messages"][1]["updateComponents"]["components"][0]["children"].clone()).is_equal_to(json!([]));
}

#[test]
fn without_actions_keeps_a_root_button_as_an_empty_column() {
    let mut s = forms::confirm("c", &AskConfirm { message: "Ok ?".into(), danger: false }, &Localizer::new(Some("fr")));
    if let oku_slides_a2ui::ServerBody::UpdateComponents(u) = &mut s.messages[1].body {
        u.components =
            vec![oku_slides_a2ui::Component::button("root", "Ok", false, oku_slides_a2ui::Action::named("suggest"))];
    }
    let json = serde_json::to_value(s.without_actions()).unwrap();
    assert_that!(json["messages"][1]["updateComponents"]["components"].clone())
        .is_equal_to(json!([{"id": "root", "component": "Column", "children": []}]));
}

#[test]
fn has_actions_sees_a_button() {
    assert_that!(params().has_actions()).is_true();
    assert_that!(params().without_actions().has_actions()).is_false();
}
