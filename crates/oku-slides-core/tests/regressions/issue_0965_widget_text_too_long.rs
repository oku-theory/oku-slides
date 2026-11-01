//! t-965: a widget's label and description had no length cap; a pack could declare megabytes of
//! text that every add grid and editor then carried. A parameter's help has the description's cap.

use oku_slides_core::{SchemaFault, WIDGET_DESCRIPTION_MAX, WIDGET_LABEL_MAX, WidgetErrorKind, WidgetRegistry};
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

fn fault_of(widget: &Value) -> Option<WidgetErrorKind> {
    WidgetRegistry::from_probe(probe(widget)).err().map(|e| e.kind())
}

#[rstest]
#[case::label(json!({ "name": "w", "category": "element", "label": text(WIDGET_LABEL_MAX + 1), "params": [] }))]
#[case::label_by_locale(json!({ "name": "w", "category": "element",
    "label": { "en": "KPI", "fr": text(WIDGET_LABEL_MAX + 1) }, "params": [] }))]
#[case::description(json!({ "name": "w", "category": "element", "description": text(WIDGET_DESCRIPTION_MAX + 1), "params": [] }))]
#[case::param_label(json!({ "name": "w", "category": "element",
    "params": [{ "name": "p", "type": "plain", "label": text(WIDGET_LABEL_MAX + 1) }] }))]
#[case::param_help(json!({ "name": "w", "category": "element",
    "params": [{ "name": "p", "type": "plain", "help": text(WIDGET_DESCRIPTION_MAX + 1) }] }))]
#[case::param_help_by_locale(json!({ "name": "w", "category": "element",
    "params": [{ "name": "p", "type": "plain", "help": { "en": "Hint", "fr": text(WIDGET_DESCRIPTION_MAX + 1) } }] }))]
#[case::column_help(json!({ "name": "w", "category": "element", "params": [{ "name": "l", "type": "list",
    "item": [{ "name": "c", "type": "plain", "help": text(WIDGET_DESCRIPTION_MAX + 1) }] }] }))]
#[case::column_label(json!({ "name": "w", "category": "element", "params": [{ "name": "l", "type": "list",
    "item": [{ "name": "c", "type": "plain", "label": text(WIDGET_LABEL_MAX + 1) }] }] }))]
#[case::item_label(json!({ "name": "w", "category": "element", "params": [{ "name": "l", "type": "list",
    "item-label": text(WIDGET_LABEL_MAX + 1), "item": [{ "name": "c", "type": "plain" }] }] }))]
fn issue_0965_widget_text_over_its_cap_is_too_long(#[case] widget: Value) {
    assert_that!(fault_of(&widget)).is_equal_to(Some(WidgetErrorKind::BadSchema(SchemaFault::TooLong)));
}

#[test]
fn issue_0965_widget_text_at_its_cap_is_accepted() {
    let widget = json!({ "name": "w", "category": "element", "label": text(WIDGET_LABEL_MAX),
        "description": text(WIDGET_DESCRIPTION_MAX),
        "params": [{ "name": "p", "type": "plain", "label": text(WIDGET_LABEL_MAX), "help": text(WIDGET_DESCRIPTION_MAX) }] });
    assert_that!(fault_of(&widget)).is_none();
}
