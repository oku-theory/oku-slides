use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{list, nest_schema};

/// Chaque `$ref` du schéma désigne un chemin qui existe.
fn refs_resolve(root: &Value, node: &Value) -> bool {
    match node {
        Value::Object(map) => map.iter().all(|(k, v)| {
            if k == "$ref" {
                v.as_str().and_then(|r| r.strip_prefix('#')).is_some_and(|p| root.pointer(p).is_some())
            } else {
                refs_resolve(root, v)
            }
        }),
        Value::Array(items) => items.iter().all(|v| refs_resolve(root, v)),
        _ => true,
    }
}

#[test]
fn nest_schema_moves_defs_under_first_property_and_rewrites_refs() {
    let schema = json!({
        "type": "object",
        "properties": { "ops": { "type": "array", "items": { "$ref": "#/$defs/Op" } } },
        "required": ["ops"],
        "$defs": { "Op": { "type": "object", "properties": { "next": { "$ref": "#/$defs/Op" } } } }
    });
    let nested = nest_schema(schema);
    assert_that!(nested.get("$defs")).is_none();
    assert_that!(nested.pointer("/properties/ops/$defs/Op")).is_some();
    assert_that!(refs_resolve(&nested, &nested)).is_true();
}

#[test]
fn every_tool_schema_keeps_only_properties_and_required_at_root_and_resolves() {
    for tool in list() {
        let schema = serde_json::to_value(&tool.input_schema).unwrap();
        assert_that!(refs_resolve(&schema, &schema)).named(&tool.name).is_true();
    }
}

#[test]
fn tool_names_are_unique() {
    let mut names: Vec<String> = list().into_iter().map(|t| t.name).collect();
    let n = names.len();
    names.sort();
    names.dedup();
    assert_that!(names.len()).is_equal_to(n);
}
