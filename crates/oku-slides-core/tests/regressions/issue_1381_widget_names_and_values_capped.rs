//! t-1381: t-965's caps left a parameter's name, a choice's values, a string default, a `bind`
//! and a slot's `accepts` bounded only by the pack size; and a schema fault named the bare widget,
//! so the probe assembly could blame a same-named widget of another pack.

use oku_slides_core::{
    PARAM_NAME_MAX, PLAIN_MAX, SCHEMA_LIST_MAX, SchemaFault, WIDGET_LABEL_MAX, WidgetErrorKind, WidgetRegistry,
};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

/// The probe of pack `t` with the single element widget `w`.
fn probe(widget: &Value) -> Value {
    json!({ "packs": [{ "namespace": "t", "kind": "widgets", "widgets": [widget] }] })
}

fn text(n: usize) -> String {
    "é".repeat(n)
}

fn values(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("v{i}")).collect()
}

fn widget(param: &Value) -> Value {
    json!({ "name": "w", "category": "element", "params": [param] })
}

fn fault_of(widget: &Value) -> Option<WidgetErrorKind> {
    WidgetRegistry::from_probe(probe(widget)).err().map(|e| e.kind())
}

#[rstest]
#[case::param_name(json!({ "name": text(PARAM_NAME_MAX + 1), "type": "plain" }))]
#[case::column_name(json!({ "name": "l", "type": "list", "item": [{ "name": text(PARAM_NAME_MAX + 1), "type": "plain" }] }))]
#[case::choice_count(json!({ "name": "p", "type": "choice", "values": values(SCHEMA_LIST_MAX + 1) }))]
#[case::choice_value(json!({ "name": "p", "type": "choice", "values": [text(WIDGET_LABEL_MAX + 1)] }))]
#[case::plain_default(json!({ "name": "p", "type": "plain", "default": "a".repeat(PLAIN_MAX + 1) }))]
#[case::bind(json!({ "name": "p", "type": "plain", "bind": text(PARAM_NAME_MAX + 1) }))]
#[case::accepts_count(json!({ "name": "s", "type": "slot", "mode": "flow", "accepts": values(SCHEMA_LIST_MAX + 1) }))]
#[case::accepts_name(json!({ "name": "s", "type": "slot", "mode": "flow", "accepts": [text(WIDGET_LABEL_MAX + 1)] }))]
fn issue_1381_schema_entry_over_its_cap_is_too_long(#[case] param: Value) {
    assert_that!(fault_of(&widget(&param))).is_equal_to(Some(WidgetErrorKind::BadSchema(SchemaFault::TooLong)));
}

#[test]
fn issue_1381_schema_entries_at_their_caps_are_accepted() {
    let params = json!([
        { "name": text(PARAM_NAME_MAX), "type": "plain", "default": "a".repeat(PLAIN_MAX), "bind": text(PARAM_NAME_MAX) },
        { "name": "c", "type": "choice", "values": values(SCHEMA_LIST_MAX) },
    ]);
    let w = json!({ "name": "w", "category": "element", "params": params });
    assert_that!(fault_of(&w)).is_none();
}

#[test]
fn issue_1381_schema_fault_names_the_qualified_widget() {
    let bad = json!({ "name": "card", "category": "element", "params": [{ "name": "", "type": "plain" }] });
    let probe = json!({ "packs": [{ "namespace": "acme", "kind": "widgets", "widgets": [bad] }] });
    let err = WidgetRegistry::from_probe(probe).err().unwrap_or_else(|| panic!("an empty parameter name is refused"));
    assert_that!(err.widget()).is_equal_to(Some("acme/card"));
}
