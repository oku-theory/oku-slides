//! The ops schema, and its TypeScript mirror in the UI.

use speculoos::prelude::*;

use super::{Op, ops_schema};

/// Les noms d'op (`op`) que déclare le schéma, en descendant partout.
fn op_names(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Object(map) => {
            if let Some(op) = map.get("properties").and_then(|p| p.get("op")) {
                let consts = op
                    .get("const")
                    .into_iter()
                    .chain(op.get("enum").into_iter().filter_map(|e| e.as_array()).flatten());
                out.extend(consts.filter_map(|c| c.as_str().map(str::to_owned)));
            }
            map.values().for_each(|x| op_names(x, out));
        }
        serde_json::Value::Array(items) => items.iter().for_each(|x| op_names(x, out)),
        _ => {}
    }
}

/// t-355: every `Op` variant has its own in the `Op` type of `ui/src/slideTypes.ts`.
#[test]
fn every_op_variant_is_mirrored_in_the_ui_op_type() {
    // The generated schema, knowledge ops included.
    let schema = serde_json::to_value(schemars::schema_for!(Vec<Op>)).unwrap();
    // `Op` alone: `TableOp` and `ListOp`, its sub-ops, also have an `op` field.
    let op = schema.get("$defs").or_else(|| schema.get("definitions")).and_then(|d| d.get("Op")).unwrap();
    let mut names = Vec::new();
    op_names(op.get("oneOf").or_else(|| op.get("anyOf")).unwrap(), &mut names);
    names.sort();
    names.dedup();
    assert_that!(names.len()).is_greater_than_or_equal_to(27);
    let types = include_str!("../../../../ui/src/slideTypes.ts");
    let ui = &types[types.find("export type Op =").unwrap()..];
    let ui = &ui[..ui.find(";\n").unwrap()];
    let missing: Vec<&String> = names.iter().filter(|n| !ui.contains(&format!("op: \"{n}\""))).collect();
    assert_that!(missing).is_empty();
}

/// The `description`s of the schema, descending everywhere.
fn descriptions<'a>(v: &'a serde_json::Value, out: &mut Vec<&'a str>) {
    match v {
        serde_json::Value::Object(map) => {
            out.extend(map.get("description").and_then(serde_json::Value::as_str));
            map.values().for_each(|x| descriptions(x, out));
        }
        serde_json::Value::Array(items) => items.iter().for_each(|x| descriptions(x, out)),
        _ => {}
    }
}

/// No description in the schema contains a rustdoc link.
///
/// A description goes to the LLM as is.
#[test]
fn ops_schema_descriptions_have_no_rustdoc_links() {
    let schema = ops_schema().unwrap();
    let mut all = Vec::new();
    descriptions(&schema, &mut all);
    assert_that!(all.len()).is_greater_than(0);
    let linked: Vec<&str> = all.into_iter().filter(|d| d.contains("](") || d.contains("[`")).collect();
    assert_that!(linked).is_empty();
}

/// Accented letters that betray French text left in a schema description.
const FRENCH_LETTERS: &[char] =
    &['é', 'è', 'ê', 'à', 'â', 'ç', 'ù', 'û', 'ô', 'î', 'œ', 'É', 'È', 'Ê', 'À', 'Â', 'Ç', 'Ù', 'Û', 'Ô', 'Î', 'Œ'];

/// No description in the schema contains a French accented letter.
///
/// The descriptions are the LLM's documentation of the ops, written in English.
#[test]
fn ops_schema_descriptions_have_no_french_letters() {
    let schema = ops_schema().unwrap();
    let mut all = Vec::new();
    descriptions(&schema, &mut all);
    assert_that!(all.len()).is_greater_than(0);
    let french: Vec<&str> = all.into_iter().filter(|d| d.contains(FRENCH_LETTERS)).collect();
    assert_that!(french).is_empty();
}

/// The `Op` variant of the schema named `name`.
fn op_variant<'a>(schema: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    let defs = schema.get("$defs").or_else(|| schema.get("definitions")).unwrap();
    let op = defs.get("Op").unwrap();
    let variants = op.get("oneOf").or_else(|| op.get("anyOf")).and_then(serde_json::Value::as_array).unwrap();
    variants
        .iter()
        .find(|v| {
            let mut names = Vec::new();
            op_names(v, &mut names);
            names == [name]
        })
        .unwrap()
}

/// Each data op sample parses, round trips, names a schema variant and gives its required fields.
#[test]
fn ops_schema_validates_one_sample_of_each_data_op_including_remove_rename_retype() {
    let schema = ops_schema().unwrap();
    let samples = [
        serde_json::json!({ "op": "add_data", "name": "Sales",
            "table": { "columns": [{ "name": "Month" }], "rows": [{ "cells": ["Jan"] }] } }),
        serde_json::json!({ "op": "edit_data", "data": "d1",
            "change": { "op": "set_cell", "row": "r1", "column": "c1", "value": 3 } }),
        serde_json::json!({ "op": "bind_data", "target": { "slide": "s", "element": "g" }, "param": "option",
            "data": "d1", "columns": ["c1"] }),
        serde_json::json!({ "op": "remove_data", "data": "d1" }),
        serde_json::json!({ "op": "rename_data", "data": "d1", "name": "ventes" }),
        serde_json::json!({ "op": "set_column_type", "data": "d1", "column": "c2", "type": "integer" }),
    ];
    for sample in samples {
        let name = sample["op"].as_str().unwrap();
        let variant = op_variant(&schema, name);
        let required: Vec<&str> = variant
            .get("required")
            .and_then(serde_json::Value::as_array)
            .unwrap()
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect();
        let missing: Vec<&&str> = required.iter().filter(|f| sample.get(**f).is_none()).collect();
        assert_that!(missing).is_empty();
        let op: super::Op = serde_json::from_value(sample.clone()).unwrap();
        let back: super::Op = serde_json::from_value(serde_json::to_value(&op).unwrap()).unwrap();
        assert_that!(back).is_equal_to(op);
    }
}

/// A query data and a new query parse, round trip and give their schema's required fields;
/// `add_data` requires neither `table` nor `sql`, as it takes one of them.
#[test]
fn ops_schema_validates_add_data_with_sql_and_set_query() {
    let schema = ops_schema().unwrap();
    let samples = [
        serde_json::json!({ "op": "add_data", "name": "top", "sql": "SELECT * FROM sales" }),
        serde_json::json!({ "op": "set_query", "data": "d1", "sql": "SELECT Month FROM sales" }),
    ];
    for sample in samples {
        let variant = op_variant(&schema, sample["op"].as_str().unwrap());
        let required: Vec<&str> = variant
            .get("required")
            .and_then(serde_json::Value::as_array)
            .unwrap()
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect();
        assert_that!(required.iter().all(|f| sample.get(*f).is_some())).is_true();
        if sample["op"] == "add_data" {
            assert_that!(required.contains(&"table") || required.contains(&"sql")).is_false();
        }
        let op: super::Op = serde_json::from_value(sample.clone()).unwrap();
        let back: super::Op = serde_json::from_value(serde_json::to_value(&op).unwrap()).unwrap();
        assert_that!(back).is_equal_to(op);
    }
    let properties = &op_variant(&schema, "add_data")["properties"];
    assert_that!(properties.get("sql").is_some()).is_true();
}
