use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{Inner, agent_reference, branch_name, clean_doc, reference_from, render_components};
use crate::Example;

#[rstest]
#[case::plain("Child identifiers, top to bottom.", "Child identifiers, top to bottom.")]
#[case::link("Components of the [`CATALOG_ID`](crate::CATALOG_ID) catalog.", "Components of the `CATALOG_ID` catalog.")]
#[case::bare_link("Fate of a text chunk in a [`DiffSegment`].", "Fate of a text chunk in a `DiffSegment`.")]
#[case::first_paragraph("A button.\n\n`primary`: the main action.", "A button.")]
#[case::wrapped("Line one\ncontinues here.", "Line one continues here.")]
fn clean_doc_keeps_the_first_paragraph_without_links(#[case] doc: &str, #[case] expected: &str) {
    assert_that!(clean_doc(doc)).is_equal_to(expected.to_owned());
}

#[test]
fn render_components_lists_exactly_what_the_agent_examples_use() {
    assert_that!(render_components(agent_reference().unwrap())).is_equal_to(
        "Column{children}, Card{child, tone?: accent}, Text{text, variant?: h4|body|caption|note}, \
         Row{children}, Button{label, action}, DiffText{segments}, Disclosure{summary, child, open?}, \
         Badge{text, tone?: accent}, Markdown{text}"
            .to_owned(),
    );
}

#[rstest]
#[case::literal_text("Text", "text", "string")]
#[case::named_array("DiffText", "segments", "[DiffSegment]")]
#[case::named_object("Button", "action", "Action")]
#[case::optional_bool("Disclosure", "open", "boolean")]
#[case::string_array("Column", "children", "[string]")]
fn agent_reference_reads_each_prop_type(#[case] component: &str, #[case] prop: &str, #[case] ty: &str) {
    let reference = agent_reference().unwrap();
    let props = &reference.iter().find(|c| c.component == component).unwrap().props;
    assert_that!(props.iter().find(|p| p.name == prop).unwrap().ty.as_str()).is_equal_to(ty);
}

#[test]
fn agent_reference_button_reads_label_then_action_both_required() {
    let reference = agent_reference().unwrap();
    let button = serde_json::to_value(reference.iter().find(|c| c.component == "Button").unwrap()).unwrap();
    let props: Vec<Value> =
        button["props"].as_array().unwrap().iter().map(|p| json!([p["name"], p["required"], p["type"]])).collect();
    assert_that!(props).is_equal_to(vec![json!(["label", true, "string"]), json!(["action", true, "Action"])]);
}

#[test]
fn agent_reference_matches_its_snapshot() {
    insta::assert_json_snapshot!(agent_reference().unwrap());
}

#[test]
fn agent_reference_descriptions_are_clean_and_present() {
    let faults: Vec<String> = agent_reference()
        .unwrap()
        .iter()
        .flat_map(|c| c.props.iter().map(move |p| (c.component.as_str(), p)))
        .filter(|(_, p)| p.description.is_empty() || p.description.contains("](") || p.description.contains("crate::"))
        .map(|(c, p)| format!("{c}.{}: {:?}", p.name, p.description))
        .collect();
    assert_that!(faults).is_empty();
}

#[rstest]
#[case::const_tag(json!({ "properties": { "component": { "const": "Badge" } } }), Some("Badge"))]
#[case::enum_tag(json!({ "properties": { "component": { "enum": ["Badge"] } } }), Some("Badge"))]
#[case::untagged(json!({ "properties": {} }), None)]
fn branch_name_reads_a_const_or_a_one_value_enum(#[case] branch: Value, #[case] expected: Option<&str>) {
    assert_that!(branch_name(&branch)).is_equal_to(expected);
}

/// The Markdown agent examples alone.
fn markdown_examples() -> Vec<Example> {
    crate::examples().into_iter().filter(|e| e.component == "Markdown").collect()
}

/// A schema with one Markdown branch whose `text` prop is `text`, and `defs`.
fn markdown_schema(text: &Value, defs: &Value) -> Value {
    json!({
        "$defs": defs,
        "oneOf": [{
            "type": "object",
            "properties": { "component": { "const": "Markdown" }, "text": text },
            "required": ["component", "text"],
        }],
    })
}

#[test]
fn reference_from_a_schema_without_one_of_is_no_branches() {
    let e = reference_from(&json!({}), &crate::examples()).unwrap_err();
    assert_that!(matches!(e, Inner::NoBranches { .. })).is_true();
}

#[test]
fn reference_from_a_schema_missing_a_branch_is_missing_branch() {
    let e = reference_from(&json!({ "oneOf": [] }), &crate::examples()).unwrap_err();
    assert_that!(matches!(e, Inner::MissingBranch { ref component, .. } if component == "Column")).is_true();
}

#[test]
fn reference_from_an_undocumented_prop_is_no_description() {
    let schema = markdown_schema(&json!({ "type": "string" }), &json!({}));
    let e = reference_from(&schema, &markdown_examples()).unwrap_err();
    assert_that!(matches!(e, Inner::NoDescription { ref prop, .. } if prop == "text")).is_true();
}

#[test]
fn reference_from_an_unresolved_reference_is_unresolved() {
    let schema = markdown_schema(&json!({ "$ref": "#/$defs/Nowhere", "description": "The text." }), &json!({}));
    let e = reference_from(&schema, &markdown_examples()).unwrap_err();
    assert_that!(matches!(e, Inner::Unresolved { .. })).is_true();
}

#[test]
fn reference_from_a_shape_it_cannot_read_is_unknown_shape() {
    let schema = markdown_schema(&json!({ "not": {}, "description": "The text." }), &json!({}));
    let e = reference_from(&schema, &markdown_examples()).unwrap_err();
    assert_that!(matches!(e, Inner::UnknownShape { .. })).is_true();
}

#[test]
fn reference_from_a_reference_cycle_is_too_deep() {
    let defs = json!({ "Loop": { "$ref": "#/$defs/Loop" } });
    let schema = markdown_schema(&json!({ "$ref": "#/$defs/Loop", "description": "The text." }), &defs);
    let e = reference_from(&schema, &markdown_examples()).unwrap_err();
    assert_that!(matches!(e, Inner::TooDeep { .. })).is_true();
}
