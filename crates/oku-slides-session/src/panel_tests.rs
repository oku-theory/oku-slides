use oku_slides_core::{Params, WidgetRegistry, WidgetSchema};
use oku_slides_i18n::Localizer;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::HostParams;

/// A registry of one element pack `t` holding `widgets`.
fn registry(widgets: &[Value]) -> WidgetRegistry {
    let probe = json!({ "packs": [{ "namespace": "t", "kind": "widgets", "widgets": widgets }] });
    WidgetRegistry::from_probe(probe).expect("test probe")
}

/// An element widget `name` declaring `params`.
fn widget(name: &str, params: &Value) -> Value {
    json!({ "name": name, "label": null, "category": "element", "params": params })
}

/// The element panel of `schema` at its defaults, in `lang`.
fn element(schema: &WidgetSchema, lang: &str) -> HostParams {
    HostParams::from_schema(schema, &Params::default(), "s", "session-widget-params", true, &Localizer::new(Some(lang)))
}

fn french() -> Localizer {
    Localizer::new(Some("fr"))
}

#[test]
fn from_schema_host_param_only_notes_the_data_button() {
    let r = registry(&[widget("chart", &json!([{ "name": "option", "type": "host", "host": "chart" }]))]);
    let p = element(r.get("t/chart").expect("t/chart"), "fr");
    assert_that!(p.spec.fields).is_empty();
    assert_that!(p.notes).is_equal_to(vec![french().tr("session-element-data-only")]);
    assert_that!(p.notes).is_equal_to(vec!["Modifiez ses données avec le bouton Données.".to_owned()]);
}

#[test]
fn from_schema_slot_and_grid_only_has_no_params_note() {
    let r = registry(&[widget(
        "frame",
        &json!([{ "name": "kids", "type": "slot", "mode": "flow" }, { "name": "grid", "type": "grid" }]),
    )]);
    let p = element(r.get("t/frame").expect("t/frame"), "fr");
    assert_that!(p.notes).is_equal_to(vec![french().tr("session-widget-no-params")]);
}

#[test]
fn from_schema_host_slot_and_grid_params_have_no_field() {
    let r = registry(&[widget(
        "mixed",
        &json!([
            { "name": "title", "type": "plain" },
            { "name": "option", "type": "host", "host": "chart" },
            { "name": "kids", "type": "slot", "mode": "flow" },
            { "name": "grid", "type": "grid" },
        ]),
    )]);
    let p = element(r.get("t/mixed").expect("t/mixed"), "fr");
    assert_that!(p.keys).is_equal_to(vec!["title".to_owned()]);
    assert_that!(p.notes).is_empty();
}

#[test]
fn from_schema_unsettable_param_notes_it_fixed_here() {
    let r = registry(&[widget("photo", &json!([{ "name": "src", "type": "asset", "label": "Image" }]))]);
    let p = element(r.get("t/photo").expect("t/photo"), "fr");
    assert_that!(p.spec.fields).is_empty();
    assert_that!(p.notes)
        .is_equal_to(vec![french().tr_with("session-element-param-fixed", &[("name", "Image".into())])]);
    assert_that!(p.notes).is_equal_to(vec!["Image : non réglable ici".to_owned()]);
}
