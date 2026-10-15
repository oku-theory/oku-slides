//! Le schéma des ops, et son miroir TypeScript dans l'interface.

use speculoos::prelude::*;

use super::ops_schema;

/// Les noms d'op (`op`) que déclare le schéma, en descendant partout.
fn op_names(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Object(map) => {
            if let Some(op) = map.get("properties").and_then(|p| p.get("op")) {
                let consts =
                    op.get("const").into_iter().chain(op.get("enum").into_iter().flat_map(|e| e.as_array()).flatten());
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
