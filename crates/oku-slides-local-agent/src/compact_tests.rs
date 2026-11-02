use serde_json::json;
use speculoos::prelude::*;

use super::{ToolSpec, compact, exposed};
use crate::families::Family;

fn spec(name: &str) -> ToolSpec {
    ToolSpec {
        name: name.into(),
        description: "Save the deck. The user confirms first.".into(),
        schema: json!({"type": "object", "properties": {"path": {"type": "string", "description": "long"}}, "required": ["path"]}),
    }
}

#[test]
fn compact_keeps_first_sentence() {
    assert_that!(compact(&spec("save")).description.as_str()).is_equal_to("Save the deck.");
}

#[test]
fn compact_keeps_whole_text_without_sentence_end() {
    let mut s = spec("save");
    s.description = "Save the deck".into();
    assert_that!(compact(&s).description.as_str()).is_equal_to("Save the deck");
}

#[test]
fn compact_drops_nested_descriptions_keeps_required() {
    let c = compact(&spec("save"));
    assert_that!(c.schema["properties"]["path"].get("description")).is_none();
    assert_that!(c.schema["required"]).is_equal_to(json!(["path"]));
}

#[test]
fn compact_drops_descriptions_inside_arrays() {
    let mut s = spec("save");
    s.schema =
        json!({"oneOf": [{"type": "string", "description": "x"}], "items": {"description": "y", "type": "integer"}});
    let c = compact(&s);
    assert_that!(c.schema).is_equal_to(json!({"oneOf": [{"type": "string"}], "items": {"type": "integer"}}));
}

#[test]
fn exposed_adds_ask_report_and_filters_by_route() {
    let all = vec![spec("save"), spec("apply"), spec("report")];
    let names: Vec<String> = exposed(&all, &[Family::Edit]).into_iter().map(|s| s.name).collect();
    assert_that!(names).is_equal_to(vec!["apply".to_owned(), "report".to_owned()]);
}

#[test]
fn compact_keeps_property_named_description() {
    let mut s = spec("apply");
    s.schema = json!({"type": "object", "description": "top", "properties": {"description": {"type": "string", "description": "inner"}}});
    let c = compact(&s);
    assert_that!(c.schema).is_equal_to(json!({"type": "object", "properties": {"description": {"type": "string"}}}));
}

#[rstest::rstest]
#[case::render("render", "Markdown{text}")]
#[case::examples("a2ui_examples", "Props and full examples")]
fn compact_of_an_a2ui_tool_keeps_what_the_agent_needs(#[case] name: &str, #[case] kept: &str) {
    let (name, description) = oku_slides_mcp::tool_descriptions_for(oku_slides_session::DocumentKind::Deck)
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap();
    let spec = ToolSpec { name, description, schema: json!({ "type": "object" }) };
    assert_that!(compact(&spec).description).contains(kept);
}

#[rstest::rstest]
#[case::period_newline("Save the deck.\nThe user confirms first.", "Save the deck.")]
#[case::final_period("Save the deck.", "Save the deck.")]
#[case::eg("Move an element, e.g. a chart. Then render.", "Move an element, e.g. a chart.")]
#[case::ie("Pick a layout, i.e. a slide kind. Then apply.", "Pick a layout, i.e. a slide kind.")]
#[case::eg_newline("Move one, e.g.\na chart. Then render.", "Move one, e.g.\na chart.")]
fn compact_first_sentence_ends_at_a_real_period(#[case] description: &str, #[case] expected: &str) {
    let mut s = spec("save");
    s.description = description.into();
    s.schema = json!({"type": "object"});
    assert_that!(compact(&s).description.as_str()).is_equal_to(expected);
}

#[test]
fn compact_of_report_keeps_its_facts_hint() {
    let (name, description) = oku_slides_mcp::tool_descriptions_for(oku_slides_session::DocumentKind::Deck)
        .into_iter()
        .find(|(n, _)| n == "report")
        .unwrap();
    let schema = json!({"type": "object", "properties": {"title": {"type": "string"}, "facts": {"type": "array"}, "why": {"type": "string"}}, "required": ["title", "facts"]});
    let compacted = compact(&ToolSpec { name, description, schema }).description;
    assert_that!(compacted).contains("facts: 1-4 short lines");
    assert_that!(compacted).does_not_contain("why:");
}

#[test]
fn exposed_lists_a_tool_of_several_families_once() {
    let all = vec![spec("deck_outline"), spec("report")];
    let names: Vec<String> = exposed(&all, &[Family::Edit, Family::Present]).into_iter().map(|s| s.name).collect();
    assert_that!(names).is_equal_to(vec!["deck_outline".to_owned(), "report".to_owned()]);
}

#[rstest::rstest]
#[case::defs("$defs")]
#[case::definitions("definitions")]
fn compact_keeps_a_definition_named_description(#[case] key: &str) {
    let mut s = spec("apply");
    s.schema = json!({"type": "object", key: {"description": {"type": "string", "description": "inner"}}});
    let c = compact(&s);
    assert_that!(c.schema).is_equal_to(json!({"type": "object", key: {"description": {"type": "string"}}}));
}
