use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{FILL_BYTES, Pick, kinds, narrow, parse_kinds, parse_pick, pick, tagged_union};
use crate::compact::ToolSpec;

fn tool(name: &str) -> ToolSpec {
    ToolSpec { name: name.into(), description: format!("Does {name}."), schema: json!({"type": "object"}) }
}

/// A call schema whose `t_defs_Op` is a union tagged by `op`, one def reached only from `add`.
fn with_ops() -> Value {
    let branch = |i: usize, op: &str, extra: Value| {
        let mut props = json!({"op": {"const": op}});
        if let Value::Object(e) = extra {
            props.as_object_mut().unwrap().extend(e);
        }
        json!({"title": format!("t_defs_Op_{i}"), "type": "object", "additionalProperties": false,
            "properties": props, "required": ["op"], "x-order": ["op"]})
    };
    json!({"title": "Reply", "anyOf": [
        {"title": "t_call", "type": "object", "properties": {"tool": {"type": "string", "enum": ["t"]},
            "arguments": {"title": "t_args", "type": "object", "properties": {"ops": {"type": "array",
            "items": {"$ref": "#/$defs/t_defs_Op"}}}}}},
        {"title": "Reply_text", "type": "object", "properties": {"text": {"type": "string"}}}],
        "$defs": {
            "t_defs_Op": {"title": "t_defs_Op", "anyOf": [
                branch(0, "set", json!({})),
                branch(1, "add", json!({"element": {"$ref": "#/$defs/t_defs_Element"}})),
                branch(2, "remove", json!({}))]},
            "t_defs_Element": {"title": "t_defs_Element", "type": "object", "properties": {}},
            "t_defs_Other": {"title": "t_defs_Other", "type": "object", "properties": {}}}})
}

#[test]
fn pick_offers_exactly_the_tool_names() {
    let schema = pick(&[tool("apply"), tool("report")]);
    assert_that!(schema["title"]).is_equal_to(json!("Pick"));
    assert_that!(schema["properties"]["tool"]["enum"]).is_equal_to(json!(["apply", "report"]));
    assert_that!(schema["x-order"]).is_equal_to(json!(["tool"]));
    assert_that!(schema["additionalProperties"]).is_equal_to(json!(false));
}

#[rstest]
#[case::known(r#"{"tool": "report"}"#, Pick::Tool("report".into()))]
#[case::unknown(r#"{"tool": "nope"}"#, Pick::Invalid("unknown tool nope".into()))]
#[case::not_json("{", Pick::Invalid("not valid JSON".into()))]
fn parse_pick_reads_a_known_name(#[case] json: &str, #[case] expected: Pick) {
    let got = parse_pick(json, &[tool("apply"), tool("report")]);
    match (&got, &expected) {
        (Pick::Invalid(a), Pick::Invalid(b)) => assert_that!(a.as_str()).starts_with(b.as_str()),
        _ => assert_that!(got).is_equal_to(expected),
    }
}

#[test]
fn tagged_union_finds_the_op_union_and_its_tags() {
    let (name, tags) = tagged_union(&with_ops()).unwrap();
    assert_that!(name.as_str()).is_equal_to("t_defs_Op");
    assert_that!(tags).is_equal_to(vec!["set".to_owned(), "add".to_owned(), "remove".to_owned()]);
}

#[test]
fn tagged_union_of_a_schema_without_one_is_none() {
    assert_that!(tagged_union(&json!({"title": "Reply", "anyOf": []}))).is_none();
}

#[test]
fn narrow_keeps_only_the_kept_branches() {
    let narrowed = narrow(&with_ops(), "t_defs_Op", &["remove".to_owned()]);
    let branches = narrowed["$defs"]["t_defs_Op"]["anyOf"].as_array().unwrap().clone();
    assert_that!(branches).has_length(1);
    assert_that!(branches[0]["properties"]["op"]).is_equal_to(json!({"const": "remove"}));
}

#[rstest]
#[case::reached(&["add"], true)]
#[case::unreached(&["set"], false)]
fn narrow_prunes_the_definitions_no_kept_branch_reaches(#[case] kept: &[&str], #[case] element: bool) {
    let kept: Vec<String> = kept.iter().map(|s| (*s).to_owned()).collect();
    let narrowed = narrow(&with_ops(), "t_defs_Op", &kept);
    assert_that!(narrowed["$defs"].get("t_defs_Element").is_some()).is_equal_to(element);
    assert_that!(narrowed["$defs"].get("t_defs_Other")).is_none();
}

#[test]
fn narrow_with_no_known_tag_keeps_the_whole_union() {
    let narrowed = narrow(&with_ops(), "t_defs_Op", &["nope".to_owned()]);
    assert_that!(narrowed["$defs"]["t_defs_Op"]["anyOf"].as_array().unwrap()).has_length(3);
}

#[test]
fn kinds_offers_the_tags_one_to_three() {
    let schema = kinds(&["set".to_owned(), "add".to_owned()]);
    let items = &schema["properties"]["kinds"];
    assert_that!(items["items"]["enum"]).is_equal_to(json!(["set", "add"]));
    assert_that!((items["minItems"].clone(), items["maxItems"].clone())).is_equal_to((json!(1), json!(3)));
}

#[test]
fn parse_kinds_keeps_the_offered_tags() {
    let tags = ["set".to_owned(), "add".to_owned()];
    assert_that!(parse_kinds(r#"{"kinds": ["add", "nope"]}"#, &tags)).is_equal_to(vec!["add".to_owned()]);
    assert_that!(parse_kinds("{", &tags)).is_empty();
}

#[test]
fn fill_bytes_leaves_room_in_the_context() {
    // About 3.4 bytes per token on Apple's tokenizer: under half of the 4,096-token window.
    assert_that!(FILL_BYTES / 3).is_less_than(2048);
}
