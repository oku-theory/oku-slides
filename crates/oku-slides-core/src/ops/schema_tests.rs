//! The ops schema, and its TypeScript mirror in the UI.

use speculoos::prelude::*;

use super::ops_schema;

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

/// t-355 : chaque variante de `Op` a la sienne dans le type `Op` de `ui/src/api.ts`.
#[test]
fn every_op_variant_is_mirrored_in_the_ui_op_type() {
    let schema = ops_schema().unwrap();
    // `Op` seul : `DataOp` et `ListOp`, ses sous-ops, ont aussi un champ `op`.
    let op = schema.get("$defs").or_else(|| schema.get("definitions")).and_then(|d| d.get("Op")).unwrap();
    let mut names = Vec::new();
    op_names(op.get("oneOf").or_else(|| op.get("anyOf")).unwrap(), &mut names);
    names.sort();
    names.dedup();
    assert_that!(names.len()).is_greater_than_or_equal_to(27);
    let api = include_str!("../../../../ui/src/api.ts");
    let ui = &api[api.find("export type Op =").unwrap()..];
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
