use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::{Category, HostType, ParamType, SchemaFault, SlotMode, WidgetError, WidgetErrorKind, WidgetRegistry};

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
#[case::list_default_wrong_type(
    widget("w", "element", json!([{ "name": "l", "type": "list", "item": [{ "name": "t", "type": "plain" }],
        "default": [{ "t": 3 }] }])),
    SchemaFault::DefaultWrongType
)]
#[case::list_default_beyond_the_item_bound(
    widget("w", "element", json!([{ "name": "l", "type": "list", "item": [{ "name": "t", "type": "plain" }],
        "default": (0..=crate::LIST_ITEMS_MAX).map(|_| json!({})).collect::<Vec<_>>() }])),
    SchemaFault::ListDefaultTooLarge
)]
#[case::list_default_beyond_its_max(
    widget("w", "element", json!([{ "name": "l", "type": "list", "max": 1, "item": [{ "name": "t", "type": "plain" }],
        "default": [{}, {}] }])),
    SchemaFault::ListDefaultTooLarge
)]
#[case::list_default_cell_too_long(
    widget("w", "element", json!([{ "name": "l", "type": "list", "item": [{ "name": "t", "type": "plain" }],
        "default": [{ "t": "x".repeat(crate::PLAIN_MAX + 1) }] }])),
    SchemaFault::ListDefaultTooLarge
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
        "version": 5,
        "slides": [{ "id": "s", "kind": "content", "params": {
            "elements": [{ "id": "e1", "kind": "title", "frame": { "x": 0, "y": 0, "w": 10, "h": 10 } }] } }]
    });
    let deck = crate::Deck::from_json(&json.to_string()).unwrap();
    let issues = crate::validate(&deck, &crate::test_registry::registry(), None);
    let ids: Vec<_> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(ids).is_equal_to(vec!["issue-wrong-category"]);
}

#[test]
fn widget_error_names_the_refused_widget() {
    let pack = vec![widget("card", "element", json!([]))];
    let err = WidgetRegistry::from_probe(probe(vec![card()], Some(pack))).unwrap_err();
    assert_that!(err.widget()).is_equal_to(Some("card"));
    let unreadable = WidgetRegistry::from_probe(json!({ "pack": 1 })).unwrap_err();
    assert_that!(unreadable.widget()).is_none();
}

fn list_probe(params: Value) -> Result<WidgetRegistry, WidgetError> {
    WidgetRegistry::from_probe(json!({ "base": [{ "name": "w", "category": "element", "params": params }] }))
}

fn col(name: &str, ty: &str) -> Value {
    json!({ "name": name, "type": ty })
}

fn list(item: Value) -> Value {
    json!([{ "name": "etapes", "type": "list", "item": item, "min": 0, "max": null }])
}

#[test]
fn from_probe_list_reads_columns_and_bounds() {
    let r = list_probe(json!([{ "name": "etapes", "type": "list", "item-label": "étape",
        "item": [col("date", "plain"), col("texte", "rich")], "min": 1, "max": 5 }]))
    .unwrap();
    let p = r.get("w").unwrap().param("etapes").unwrap();
    let ParamType::List { item, item_label, min, max } = &p.ty else { panic!("pas une liste") };
    assert_that!(item.iter().map(|c| c.name.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["date", "texte"]);
    assert_that!(item_label.as_deref()).is_equal_to(Some("étape"));
    assert_that!((*min, *max)).is_equal_to((1, Some(5)));
}

#[rstest]
#[case(list(json!([col("x", "slot")])))]
#[case(list(json!([col("x", "list")])))]
#[case(list(json!([col("x", "grid")])))]
fn from_probe_list_with_excluded_column_type_fails(#[case] params: Value) {
    assert_that!(list_probe(params)).is_err();
}

#[rstest]
#[case(list(json!([col("id", "plain")])))]
#[case(list(json!([col("kind", "plain")])))]
#[case(list(json!([col("a/b", "plain")])))]
#[case(json!([col("a/b", "plain")]))]
fn from_probe_reserved_name_fails(#[case] params: Value) {
    assert_that!(list_probe(params)).is_err();
}

#[test]
fn from_probe_list_without_columns_fails() {
    assert_that!(list_probe(list(json!([])))).is_err();
}

#[test]
fn from_probe_list_with_max_below_min_fails() {
    let p = json!([{ "name": "l", "type": "list", "item": [col("a", "plain")], "min": 3, "max": 2 }]);
    assert_that!(list_probe(p)).is_err();
}

#[test]
fn from_probe_list_with_wrong_column_default_fails() {
    let p = list(json!([{ "name": "n", "type": "number", "default": "x" }]));
    assert_that!(list_probe(p)).is_err();
}

#[test]
fn from_probe_list_default_with_a_wrongly_typed_value_fails() {
    let p = json!([{ "name": "l", "type": "list", "item": [col("date", "plain")], "min": 0, "max": null,
        "default": [{ "date": 2019 }] }]);
    assert_that!(list_probe(p)).is_err();
}

#[test]
fn from_probe_list_column_with_a_bind_fails() {
    let p = json!([{ "name": "l", "type": "list", "min": 0, "max": null,
        "item": [{ "name": "a", "type": "plain", "bind": "title" }] }]);
    assert_that!(list_probe(p)).is_err();
}

#[test]
fn from_probe_list_default_with_unknown_column_fails() {
    let p = json!([{ "name": "l", "type": "list", "item": [col("a", "plain")], "min": 0, "max": null,
        "default": [{ "b": "x" }] }]);
    assert_that!(list_probe(p)).is_err();
}

/// Une colonne `list` (liste imbriquée) aux colonnes `item`.
fn sublist(name: &str, item: Value) -> Value {
    json!({ "name": name, "type": "list", "item": item, "min": 0, "max": null })
}

fn fault_of(r: Result<WidgetRegistry, WidgetError>) -> Option<SchemaFault> {
    match r.map(drop).map_err(|e| e.kind()) {
        Err(WidgetErrorKind::BadSchema(f)) => Some(f),
        _ => None,
    }
}

#[test]
fn from_probe_list_with_a_nested_list_column_reads_its_columns() {
    let r =
        list_probe(list(json!([col("titre", "plain"), sublist("taches", json!([col("texte", "plain")]))]))).unwrap();
    let ParamType::List { item, .. } = &r.get("w").unwrap().param("etapes").unwrap().ty else {
        panic!("pas une liste")
    };
    let ParamType::List { item: sub, .. } = &item[1].ty else { panic!("colonne imbriquée attendue") };
    assert_that!(sub[0].name.as_str()).is_equal_to("texte");
}

#[test]
fn from_probe_list_nested_two_levels_returns_list_depth() {
    let deep = sublist("taches", json!([sublist("sous", json!([col("t", "plain")]))]));
    assert_that!(fault_of(list_probe(list(json!([deep]))))).is_equal_to(Some(SchemaFault::ListDepth));
}

#[test]
fn from_probe_nested_list_with_an_excluded_column_type_returns_column_type() {
    let bad = sublist("taches", json!([col("x", "grid")]));
    assert_that!(fault_of(list_probe(list(json!([bad]))))).is_equal_to(Some(SchemaFault::ColumnType));
}

#[test]
fn from_probe_nested_list_reserved_column_name_returns_reserved_name() {
    let bad = sublist("taches", json!([col("id", "plain")]));
    assert_that!(fault_of(list_probe(list(json!([bad]))))).is_equal_to(Some(SchemaFault::ReservedName));
}

#[test]
fn from_probe_list_default_with_nested_rows_without_ids_is_accepted() {
    let p = json!([{ "name": "l", "type": "list", "min": 0, "max": null,
        "item": [col("a", "plain"), sublist("taches", json!([col("texte", "plain")]))],
        "default": [{ "a": "x", "taches": [{ "texte": "un" }, { "texte": "deux" }] }] }]);
    assert_that!(list_probe(p)).is_ok();
}

#[test]
fn from_probe_list_default_with_a_wrongly_typed_nested_cell_returns_default_wrong_type() {
    let p = json!([{ "name": "l", "type": "list", "min": 0, "max": null,
        "item": [sublist("taches", json!([col("texte", "plain")]))],
        "default": [{ "taches": [{ "texte": 3 }] }] }]);
    assert_that!(fault_of(list_probe(p))).is_equal_to(Some(SchemaFault::DefaultWrongType));
}

#[test]
fn from_probe_list_default_beyond_the_nested_max_returns_list_default_too_large() {
    let p = json!([{ "name": "l", "type": "list", "min": 0, "max": null,
        "item": [{ "name": "taches", "type": "list", "min": 0, "max": 1, "item": [col("texte", "plain")] }],
        "default": [{ "taches": [{ "texte": "un" }, { "texte": "deux" }] }] }]);
    assert_that!(fault_of(list_probe(p))).is_equal_to(Some(SchemaFault::ListDefaultTooLarge));
}

#[test]
fn param_type_host_of_a_host_param_returns_it() {
    assert_that!(ParamType::Host { host: HostType::Table }.host()).is_equal_to(Some(HostType::Table));
}

#[test]
fn param_type_host_of_plain_returns_none() {
    assert_that!(ParamType::Plain.host()).is_none();
}

#[test]
fn localize_list_depth_in_french_says_one_level() {
    use oku_slides_i18n::Localize;
    let deep = sublist("taches", json!([sublist("sous", json!([col("t", "plain")]))]));
    let err = list_probe(list(json!([deep]))).unwrap_err();
    let text = err.localize(&oku_slides_i18n::Localizer::new(Some("fr")));
    assert_that!(text).contains("w");
    assert_that!(text).contains("un niveau");
}

// Suites de revue (t-495) : défaut d'une liste imbriquée et son minimum, ids explicites.

fn nested_min(min: usize, default: Value) -> Value {
    json!([{ "name": "l", "type": "list", "min": 0, "max": null,
        "item": [{ "name": "taches", "type": "list", "min": min, "max": null, "item": [col("texte", "plain")],
            "default": default }] }])
}

#[rstest]
#[case::one_row(json!([{ "texte": "un" }]))]
#[case::empty(json!([]))]
fn from_probe_nested_default_below_its_min_returns_list_default_below_min(#[case] default: Value) {
    assert_that!(fault_of(list_probe(nested_min(2, default)))).is_equal_to(Some(SchemaFault::ListDefaultBelowMin));
}

#[test]
fn from_probe_outer_default_row_with_a_nested_array_below_min_returns_list_default_below_min() {
    let p = json!([{ "name": "l", "type": "list", "min": 0, "max": null,
        "item": [{ "name": "taches", "type": "list", "min": 2, "max": null, "item": [col("texte", "plain")] }],
        "default": [{ "taches": [{ "texte": "un" }] }] }]);
    assert_that!(fault_of(list_probe(p))).is_equal_to(Some(SchemaFault::ListDefaultBelowMin));
}

#[test]
fn from_probe_nested_default_at_its_min_is_accepted() {
    assert_that!(list_probe(nested_min(2, json!([{ "texte": "un" }, { "texte": "deux" }])))).is_ok();
}

#[test]
fn from_probe_outer_default_row_with_a_null_nested_list_is_accepted() {
    let p = json!([{ "name": "l", "type": "list", "min": 0, "max": null,
        "item": [sublist("taches", json!([col("texte", "plain")]))], "default": [{ "taches": null }] }]);
    assert_that!(list_probe(p)).is_ok();
}

#[test]
fn from_probe_nested_default_rows_with_explicit_ids_are_accepted_and_kept() {
    let p = json!([{ "name": "l", "type": "list", "min": 0, "max": null,
        "item": [sublist("taches", json!([col("texte", "plain")]))],
        "default": [{ "taches": [{ "id": "x", "texte": "un" }, { "texte": "deux" }] }] }]);
    let r = list_probe(p).unwrap();
    let d = crate::default_items(r.get("w").unwrap().param("l").unwrap()).unwrap();
    let ids: Vec<&str> = d[0].values.list("taches").unwrap().iter().map(|t| t.id.as_str()).collect();
    assert_that!(ids).is_equal_to(vec!["x", "d1"]);
}

#[rstest]
#[case::duplicate(json!([{ "id": "x" }, { "id": "x" }]))]
#[case::slash(json!([{ "id": "a/b" }]))]
#[case::not_a_string(json!([{ "id": 3 }]))]
fn from_probe_nested_default_rows_with_bad_ids_return_default_wrong_type(#[case] rows: Value) {
    let p = json!([{ "name": "l", "type": "list", "min": 0, "max": null,
        "item": [sublist("taches", json!([col("texte", "plain")]))], "default": [{ "taches": rows }] }]);
    assert_that!(fault_of(list_probe(p))).is_equal_to(Some(SchemaFault::DefaultWrongType));
}

#[test]
fn localize_list_default_below_min_in_french_names_the_widget() {
    use oku_slides_i18n::Localize;
    let err = list_probe(nested_min(2, json!([{ "texte": "un" }]))).unwrap_err();
    let text = err.localize(&oku_slides_i18n::Localizer::new(Some("fr")));
    assert_that!(text).contains("minimum");
}
