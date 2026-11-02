use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{call, route};
use crate::compact::ToolSpec;
use crate::families::Family;

fn tool(name: &str, schema: Value) -> ToolSpec {
    ToolSpec { name: name.into(), description: format!("Does {name}."), schema }
}

/// The `arguments` schema of `call(tool)`, and the whole schema.
fn args_of(schema: Value) -> (Value, Value) {
    let out = call(&tool("t", schema));
    (out["properties"]["arguments"].clone(), out)
}

#[test]
fn route_offered_keys_are_the_only_choices() {
    let schema = route(&[Family::Edit, Family::Files]);
    assert_that!(schema["title"]).is_equal_to(json!("Route"));
    assert_that!(schema["additionalProperties"]).is_equal_to(json!(false));
    assert_that!(schema["x-order"]).is_equal_to(json!(["families"]));
    let families = &schema["properties"]["families"];
    assert_that!(families["items"]["enum"]).is_equal_to(json!(["edit", "files"]));
    assert_that!((families["minItems"].clone(), families["maxItems"].clone())).is_equal_to((json!(1), json!(2)));
}

#[test]
fn call_is_a_titled_object_with_its_name_as_enum() {
    let out = call(&tool("save", json!({"type": "object", "properties": {}})));
    assert_that!(out["title"]).is_equal_to(json!("save_call"));
    assert_that!(out["properties"]["tool"]).is_equal_to(json!({"type": "string", "enum": ["save"]}));
    assert_that!(out["x-order"]).is_equal_to(json!(["tool", "arguments"]));
    assert_that!(out["required"]).is_equal_to(json!(["tool", "arguments"]));
    assert_that!(out["additionalProperties"]).is_equal_to(json!(false));
}

#[test]
fn call_offers_no_free_text() {
    let out = call(&tool("save", json!({"type": "object", "properties": {}})));
    assert_that!(out.get("anyOf")).is_none();
    assert_that!(out["properties"].get("text")).is_none();
}

#[test]
fn call_object_gets_path_title_full_order_and_no_extra_properties() {
    let (args, _) = args_of(json!({
        "type": "object",
        "title": "Ignored",
        "properties": {"b": {"type": "string"}, "a": {"type": "object", "properties": {"x": {"type": "boolean"}}}},
        "required": ["a"],
    }));
    assert_that!(args["title"]).is_equal_to(json!("t_args"));
    assert_that!(args["x-order"]).is_equal_to(json!(["b", "a"]));
    assert_that!(args["required"]).is_equal_to(json!(["a"]));
    assert_that!(args["additionalProperties"]).is_equal_to(json!(false));
    assert_that!(args["properties"]["a"]["title"]).is_equal_to(json!("t_args_a"));
    assert_that!(args["properties"]["a"]["x-order"]).is_equal_to(json!(["x"]));
}

#[test]
fn call_required_keeps_only_listed_properties() {
    let (args, _) =
        args_of(json!({"type": "object", "properties": {"a": {"type": "string"}}, "required": ["a", "gone"]}));
    assert_that!(args["required"]).is_equal_to(json!(["a"]));
}

#[test]
fn call_defs_are_hoisted_under_the_tool_prefix() {
    let element = json!({
        "type": "object",
        "properties": {"el": {"$ref": "#/$defs/Element"}},
        "$defs": {"Element": {"type": "object", "properties": {"id": {"type": "string"}}, "required": ["id"]}},
    });
    let out = call(&tool("slide.v2", element));
    let defs = out["$defs"].as_object().unwrap();
    assert_that!(defs.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec!["slide_v2_defs_Element".to_owned()]);
    for (name, def) in defs {
        assert_that!(def["title"]).is_equal_to(json!(name));
    }
    let el = &out["properties"]["arguments"]["properties"]["el"];
    assert_that!(el).is_equal_to(&json!({"$ref": "#/$defs/slide_v2_defs_Element"}));
}

#[test]
fn call_nested_args_have_no_defs_left() {
    let (args, _) = args_of(json!({
        "type": "object",
        "properties": {"a": {"$ref": "#/definitions/A"}},
        "definitions": {"A": {"type": "object", "properties": {"x": {"type": "string"}}}},
    }));
    assert_that!(args.get("definitions")).is_none();
    assert_that!(args.get("$defs")).is_none();
}

#[test]
fn call_primitive_def_is_inlined() {
    let (args, out) = args_of(json!({
        "type": "object",
        "properties": {"kind": {"$ref": "#/$defs/Kind"}},
        "$defs": {"Kind": {"type": "string", "enum": ["a", "b"]}},
    }));
    assert_that!(args["properties"]["kind"]).is_equal_to(json!({"type": "string", "enum": ["a", "b"]}));
    assert_that!(out.get("$defs")).is_none();
}

#[test]
fn call_recursive_def_is_hoisted_once() {
    let (args, out) = args_of(json!({
        "type": "object",
        "properties": {"node": {"$ref": "#/$defs/Node"}},
        "$defs": {"Node": {"type": "object", "properties": {"children": {"type": "array", "items": {"$ref": "#/$defs/Node"}}}}},
    }));
    assert_that!(args["properties"]["node"]).is_equal_to(json!({"$ref": "#/$defs/t_defs_Node"}));
    let node = &out["$defs"]["t_defs_Node"];
    assert_that!(node["properties"]["children"]["items"]).is_equal_to(json!({"$ref": "#/$defs/t_defs_Node"}));
}

#[test]
fn call_self_holding_primitive_def_is_cut_short() {
    let (args, _) = args_of(json!({
        "type": "object",
        "properties": {"l": {"$ref": "#/$defs/L"}},
        "$defs": {"L": {"type": "array", "items": {"$ref": "#/$defs/L"}}},
    }));
    assert_that!(args["properties"]["l"]["items"]["title"]).is_equal_to(json!("AnyValue"));
}

#[rstest]
#[case::format(json!({"type": "string", "format": "uri"}), json!({"type": "string"}))]
#[case::pattern(json!({"type": "string", "pattern": "^a$", "minLength": 1}), json!({"type": "string"}))]
#[case::default(json!({"type": "boolean", "default": true, "examples": [true]}), json!({"type": "boolean"}))]
#[case::string_enum(json!({"type": "string", "enum": ["x", "y"]}), json!({"type": "string", "enum": ["x", "y"]}))]
#[case::number_enum(json!({"type": "integer", "enum": [1, 2]}), json!({"type": "integer"}))]
#[case::string_const(json!({"const": "x"}), json!({"const": "x"}))]
#[case::number_const(json!({"const": 3}), json!({"type": "number"}))]
#[case::nullable(json!({"type": ["string", "null"]}), json!({"type": "string"}))]
#[case::integer_bounds(json!({"type": "integer", "minimum": 0.5, "maximum": 9.5}), json!({"type": "integer", "minimum": 1, "maximum": 9}))]
#[case::number_bounds(json!({"type": "number", "minimum": 0.5, "exclusiveMaximum": 2}), json!({"type": "number", "minimum": 0.5}))]
#[case::array_bounds(json!({"type": "array", "items": {"type": "string"}, "maxItems": 4, "uniqueItems": true}), json!({"type": "array", "items": {"type": "string"}, "maxItems": 4}))]
#[case::single_all_of(json!({"allOf": [{"type": "string"}], "description": "d"}), json!({"type": "string"}))]
#[case::nullable_any_of(json!({"anyOf": [{"type": "integer"}, {"type": "null"}]}), json!({"type": "integer"}))]
fn call_property_is_rewritten(#[case] property: Value, #[case] expected: Value) {
    let (args, _) = args_of(json!({"type": "object", "properties": {"p": property}}));
    assert_that!(args["properties"]["p"]).is_equal_to(expected);
}

#[rstest]
#[case::any_of(json!({"anyOf": [{"type": "string"}, {"type": "integer"}]}))]
#[case::one_of(json!({"oneOf": [{"type": "string"}, {"type": "integer"}]}))]
#[case::type_list(json!({"type": ["string", "integer", "null"]}))]
fn call_union_is_a_titled_any_of(#[case] property: Value) {
    let (args, _) = args_of(json!({"type": "object", "properties": {"p": property}}));
    let p = &args["properties"]["p"];
    assert_that!(p["title"]).is_equal_to(json!("t_args_p"));
    assert_that!(p["anyOf"]).is_equal_to(json!([{"type": "string"}, {"type": "integer"}]));
    assert_that!(p.get("oneOf")).is_none();
}

#[rstest]
#[case::untyped(json!({}))]
#[case::schema_true(json!(true))]
#[case::unknown_type(json!({"type": "file"}))]
fn call_untyped_value_is_any_value(#[case] property: Value) {
    let (args, _) = args_of(json!({"type": "object", "properties": {"p": property}}));
    assert_that!(args["properties"]["p"]["title"]).is_equal_to(json!("AnyValue"));
    assert_that!(args["properties"]["p"]["anyOf"].as_array().map(Vec::len)).is_equal_to(Some(3));
}

#[rstest]
#[case::bare(json!({"type": "object"}), json!("AnyValue"))]
#[case::open(json!({"type": "object", "additionalProperties": true}), json!("AnyValue"))]
#[case::typed_values(json!({"type": "object", "additionalProperties": {"type": "string"}}), Value::Null)]
fn call_object_without_properties_is_a_dictionary(#[case] property: Value, #[case] value_title: Value) {
    let (args, _) = args_of(json!({"type": "object", "properties": {"p": property}}));
    let p = &args["properties"]["p"];
    assert_that!(p.get("title")).is_none();
    assert_that!(p["type"]).is_equal_to(json!("object"));
    assert_that!(p["additionalProperties"]["title"]).is_equal_to(value_title);
}

#[test]
fn call_array_without_items_holds_any_value() {
    let (args, _) = args_of(json!({"type": "object", "properties": {"p": {"type": "array"}}}));
    assert_that!(args["properties"]["p"]["items"]["title"]).is_equal_to(json!("AnyValue"));
}

#[test]
fn call_array_of_objects_titles_its_items() {
    let (args, _) = args_of(
        json!({"type": "object", "properties": {"ops": {"type": "array", "items": {"type": "object", "properties": {"op": {"type": "string"}}}}}}),
    );
    assert_that!(args["properties"]["ops"]["items"]["title"]).is_equal_to(json!("t_args_ops_item"));
}

#[test]
fn call_property_names_keep_their_spelling_in_titles_sanitized() {
    let (args, _) = args_of(json!({"type": "object", "properties": {"a-b": {"type": "object", "properties": {}}}}));
    assert_that!(args["properties"]["a-b"]["title"]).is_equal_to(json!("t_args_a_b"));
}

#[test]
fn call_unknown_ref_is_any_value() {
    let (args, _) = args_of(json!({"type": "object", "properties": {"p": {"$ref": "#/$defs/Missing"}}}));
    assert_that!(args["properties"]["p"]["title"]).is_equal_to(json!("AnyValue"));
}
