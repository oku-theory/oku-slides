use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::{Category, ParamType, SchemaFault, SlotMode, WidgetError, WidgetErrorKind, WidgetRegistry};

fn widget(name: &str, category: &str, params: Value) -> Value {
    json!({ "name": name, "label": null, "category": category, "params": params })
}

fn card() -> Value {
    widget(
        "card",
        "element",
        json!([
            { "name": "kicker", "type": "plain", "label": null, "default": null, "bind": null },
            { "name": "title", "type": "plain", "label": null, "default": null, "bind": null },
            { "name": "body", "type": "rich", "label": null, "default": null },
            { "name": "highlight", "type": "flag", "label": null, "default": false },
        ]),
    )
}

fn probe(base: Vec<Value>, pack: Option<Vec<Value>>) -> Value {
    json!({ "base": base, "pack": pack })
}

#[test]
fn from_probe_base_keeps_params_in_declaration_order() {
    let r = WidgetRegistry::from_probe(probe(vec![card()], None)).unwrap();
    let names: Vec<&str> = r.get("card").unwrap().params.iter().map(|p| p.name.as_str()).collect();
    assert_that!(names).is_equal_to(vec!["kicker", "title", "body", "highlight"]);
}

#[test]
fn from_probe_reads_every_param_type() {
    let all = widget(
        "all",
        "slide",
        json!([
            { "name": "a", "type": "plain", "default": "x", "bind": "title" },
            { "name": "b", "type": "rich" },
            { "name": "c", "type": "flag", "default": true },
            { "name": "d", "type": "number", "min": 1, "max": null, "default": null },
            { "name": "e", "type": "choice", "values": ["cover", "contain"], "default": "cover" },
            { "name": "f", "type": "asset" },
            { "name": "g", "type": "slot", "mode": "free", "accepts": ["card"] },
            { "name": "h", "type": "grid", "default": { "columns": 12, "rows": 6 } },
            { "name": "i", "type": "host", "host": "chart" },
        ]),
    );
    let r = WidgetRegistry::from_probe(probe(vec![card(), all], None)).unwrap();
    let w = r.get("all").unwrap();
    assert_that!(w.category).is_equal_to(Category::Slide);
    let ty = |n: &str| w.param(n).unwrap().ty.clone();
    assert_that!(ty("a")).is_equal_to(ParamType::Plain);
    assert_that!(w.param("a").unwrap().bind.as_deref()).is_equal_to(Some("title"));
    assert_that!(ty("d")).is_equal_to(ParamType::Number { min: Some(1.0), max: None });
    assert_that!(ty("e")).is_equal_to(ParamType::Choice { values: vec!["cover".into(), "contain".into()] });
    assert_that!(ty("g")).is_equal_to(ParamType::Slot { mode: SlotMode::Free, accepts: Some(vec!["card".into()]) });
    assert_that!(ty("i")).is_equal_to(ParamType::Host { host: crate::HostType::Chart });
}

#[test]
fn from_probe_unreadable_json_returns_unreadable() {
    let err = WidgetRegistry::from_probe(json!({ "base": 3 })).unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::Unreadable);
}

#[test]
fn widget_error_unreadable_builds_from_a_json_error() {
    let source = serde_json::from_str::<i32>("nope").unwrap_err();
    let err = WidgetError::unreadable(source);
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::Unreadable);
}

#[rstest]
#[case::empty_name(widget("", "element", json!([])), SchemaFault::EmptyName)]
#[case::duplicate_param(
    widget("w", "element", json!([{ "name": "a", "type": "rich" }, { "name": "a", "type": "rich" }])),
    SchemaFault::DuplicateParam
)]
#[case::empty_choice(
    widget("w", "element", json!([{ "name": "a", "type": "choice", "values": [] }])),
    SchemaFault::EmptyChoice
)]
#[case::default_not_in_choice(
    widget("w", "element", json!([{ "name": "a", "type": "choice", "values": ["x"], "default": "y" }])),
    SchemaFault::DefaultNotInChoice
)]
#[case::bind_not_plain(
    widget("w", "element", json!([{ "name": "a", "type": "rich", "bind": "title" }])),
    SchemaFault::BindNotPlain
)]
#[case::unknown_slot_kind(
    widget("w", "element", json!([{ "name": "a", "type": "slot", "mode": "flow", "accepts": ["nope"] }])),
    SchemaFault::UnknownSlotKind
)]
#[case::default_wrong_type_flag(
    widget("w", "element", json!([{ "name": "a", "type": "flag", "default": "yes" }])),
    SchemaFault::DefaultWrongType
)]
#[case::default_wrong_type_number_not_a_number(
    widget("w", "element", json!([{ "name": "a", "type": "number", "default": "1" }])),
    SchemaFault::DefaultWrongType
)]
#[case::default_wrong_type_number_below_min(
    widget("w", "element", json!([{ "name": "a", "type": "number", "min": 1, "default": 0 }])),
    SchemaFault::DefaultWrongType
)]
#[case::default_wrong_type_number_above_max(
    widget("w", "element", json!([{ "name": "a", "type": "number", "max": 10, "default": 11 }])),
    SchemaFault::DefaultWrongType
)]
#[case::default_wrong_type_plain(
    widget("w", "element", json!([{ "name": "a", "type": "plain", "default": 3 }])),
    SchemaFault::DefaultWrongType
)]
#[case::default_wrong_type_grid(
    widget("w", "element", json!([{ "name": "a", "type": "grid", "default": "nope" }])),
    SchemaFault::DefaultWrongType
)]
fn from_probe_bad_widget_returns_its_fault(#[case] bad: Value, #[case] fault: SchemaFault) {
    let err = WidgetRegistry::from_probe(probe(vec![card(), bad], None)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::BadSchema(fault));
}

#[test]
fn from_probe_bad_schema_display_is_lowercase() {
    let err = WidgetRegistry::from_probe(probe(vec![card(), widget("", "element", json!([]))], None)).unwrap_err();
    let text = err.to_string();
    assert_that!(text).contains("empty name");
    assert_that!(text).does_not_contain("EmptyName");
}

#[test]
fn from_probe_duplicate_widget_returns_duplicate_widget() {
    let err = WidgetRegistry::from_probe(probe(vec![card(), card()], None)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::BadSchema(SchemaFault::DuplicateWidget));
}

#[test]
fn from_probe_slot_accepting_a_slide_widget_returns_unknown_slot_kind() {
    let slide =
        widget("content", "slide", json!([{ "name": "b", "type": "slot", "mode": "flow", "accepts": ["content"] }]));
    let err = WidgetRegistry::from_probe(probe(vec![slide], None)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::BadSchema(SchemaFault::UnknownSlotKind));
}

#[test]
fn from_probe_pack_adds_a_new_widget() {
    let quote = widget("quote", "element", json!([{ "name": "body", "type": "rich" }]));
    let r = WidgetRegistry::from_probe(probe(vec![card()], Some(vec![quote]))).unwrap();
    assert_that!(r.len()).is_equal_to(2);
}

#[test]
fn from_probe_pack_override_adding_a_param_replaces_the_base_widget() {
    let mut params = card()["params"].as_array().unwrap().clone();
    params.push(json!({ "name": "icon", "type": "asset" }));
    let r =
        WidgetRegistry::from_probe(probe(vec![card()], Some(vec![widget("card", "element", Value::Array(params))])))
            .unwrap();
    assert_that!(r.get("card").unwrap().param("icon")).is_some();
}

#[rstest]
#[case::dropped(json!([{ "name": "title", "type": "plain" }, { "name": "body", "type": "rich" }, { "name": "highlight", "type": "flag" }]), "kicker")]
#[case::retyped(json!([{ "name": "kicker", "type": "rich" }, { "name": "title", "type": "plain" }, { "name": "body", "type": "rich" }, { "name": "highlight", "type": "flag" }]), "kicker")]
fn from_probe_pack_override_breaking_a_param_returns_incompatible_override(#[case] params: Value, #[case] param: &str) {
    let err =
        WidgetRegistry::from_probe(probe(vec![card()], Some(vec![widget("card", "element", params)]))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::IncompatibleOverride);
    assert_that!(err.to_string()).contains(param);
}

#[rstest]
#[case::number_narrowed_min(
    widget("panel", "element", json!([{ "name": "n", "type": "number", "min": 0, "max": 10 }])),
    widget("panel", "element", json!([{ "name": "n", "type": "number", "min": 1, "max": 10 }])),
    "n"
)]
#[case::number_narrowed_max(
    widget("panel", "element", json!([{ "name": "n", "type": "number", "min": 0, "max": 10 }])),
    widget("panel", "element", json!([{ "name": "n", "type": "number", "min": 0, "max": 5 }])),
    "n"
)]
#[case::slot_accepts_narrowed(
    widget("gated", "element", json!([{ "name": "kids", "type": "slot", "mode": "flow", "accepts": ["card", "quote"] }])),
    widget("gated", "element", json!([{ "name": "kids", "type": "slot", "mode": "flow", "accepts": ["card"] }])),
    "kids"
)]
#[case::bind_dropped(
    widget("titled", "element", json!([{ "name": "title", "type": "plain", "bind": "title" }])),
    widget("titled", "element", json!([{ "name": "title", "type": "plain" }])),
    "title"
)]
#[case::bind_changed(
    widget("titled", "element", json!([{ "name": "title", "type": "plain", "bind": "title" }])),
    widget("titled", "element", json!([{ "name": "title", "type": "plain", "bind": "subtitle" }])),
    "title"
)]
fn from_probe_pack_override_incompatible_param_returns_incompatible_override(
    #[case] base: Value,
    #[case] pack: Value,
    #[case] param: &str,
) {
    let err = WidgetRegistry::from_probe(probe(vec![base], Some(vec![pack]))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::IncompatibleOverride);
    assert_that!(err.to_string()).contains(param);
}

#[test]
fn from_probe_pack_override_number_widening_range_is_compatible() {
    let base = widget("panel", "element", json!([{ "name": "n", "type": "number", "min": 0, "max": 10 }]));
    let pack = widget("panel", "element", json!([{ "name": "n", "type": "number", "min": -5, "max": 20 }]));
    let r = WidgetRegistry::from_probe(probe(vec![base], Some(vec![pack]))).unwrap();
    assert_that!(r.get("panel").unwrap().param("n").unwrap().ty.clone())
        .is_equal_to(ParamType::Number { min: Some(-5.0), max: Some(20.0) });
}

#[test]
fn from_probe_pack_override_slot_accepts_widened_to_none_is_compatible() {
    let base =
        widget("gated", "element", json!([{ "name": "kids", "type": "slot", "mode": "flow", "accepts": ["card"] }]));
    let pack = widget("gated", "element", json!([{ "name": "kids", "type": "slot", "mode": "flow", "accepts": null }]));
    let r = WidgetRegistry::from_probe(probe(vec![base], Some(vec![pack]))).unwrap();
    assert_that!(r.get("gated").unwrap().param("kids").unwrap().ty.clone())
        .is_equal_to(ParamType::Slot { mode: SlotMode::Flow, accepts: None });
}

#[test]
fn from_probe_pack_override_narrowing_a_choice_returns_incompatible_override() {
    let base = widget("image", "element", json!([{ "name": "fit", "type": "choice", "values": ["cover", "contain"] }]));
    let pack = widget("image", "element", json!([{ "name": "fit", "type": "choice", "values": ["cover"] }]));
    let err = WidgetRegistry::from_probe(probe(vec![base], Some(vec![pack]))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::IncompatibleOverride);
}

#[test]
fn from_probe_pack_override_changing_category_returns_incompatible_category() {
    let err =
        WidgetRegistry::from_probe(probe(vec![card()], Some(vec![widget("card", "slide", card()["params"].clone())])))
            .unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::IncompatibleCategory);
}

#[test]
fn localize_incompatible_override_names_widget_and_param() {
    use oku_slides_i18n::Localize;
    let pack = widget("card", "element", json!([{ "name": "title", "type": "plain" }]));
    let err = WidgetRegistry::from_probe(probe(vec![card()], Some(vec![pack]))).unwrap_err();
    let text = err.localize(oku_slides_i18n::english());
    assert_that!(text).contains("card");
    assert_that!(text).contains("kicker");
}

// Un widget de slide utilisé comme élément (p. ex. "title") : le modèle v2
// le garde (rien ne se perd), `validate` le signale, et le rendu le filtre
// par catégorie (`data.typ`).
#[test]
fn element_kind_title_is_kept_by_the_model_and_reported_by_validate() {
    let json = serde_json::json!({
        "version": 3,
        "slides": [{ "id": "s", "kind": "content", "params": {
            "elements": [{ "id": "e1", "kind": "title", "frame": { "x": 0, "y": 0, "w": 10, "h": 10 } }] } }]
    });
    let deck = crate::Deck::from_json(&json.to_string()).unwrap();
    let issues = crate::validate(&deck, &crate::test_registry::registry(), None);
    let ids: Vec<_> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(ids).is_equal_to(vec!["issue-wrong-category"]);
}
