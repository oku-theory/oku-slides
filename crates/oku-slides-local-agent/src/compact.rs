//! Compact schemas: the description cut to its first sentence, no nested descriptions.

use serde_json::Value;

use crate::families::{Family, families_of};

/// Un outil MCP tel que la boucle de tour le présente au modèle.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ToolSpec {
    /// Nom de l'outil.
    pub name: String,
    /// Description de l'outil.
    pub description: String,
    /// Schéma JSON des arguments.
    pub schema: Value,
}

/// Keys whose object maps names to schemas: the names are kept, even one named `description`.
const NAMED_SCHEMAS: [&str; 3] = ["properties", "$defs", "definitions"];

/// Removes the `description` keyword from every schema, recursively.
///
/// The keys of a `properties`, `$defs` or `definitions` object are names, not keywords:
/// they are kept, even one named `description`, and only each named schema is cleaned.
fn strip(v: &mut Value) {
    match v {
        Value::Object(map) => {
            map.remove("description");
            for (key, child) in map.iter_mut() {
                match child {
                    Value::Object(named) if NAMED_SCHEMAS.contains(&key.as_str()) => {
                        named.values_mut().for_each(strip);
                    }
                    _ => strip(child),
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(strip),
        _ => {}
    }
}

/// The first sentence of `text`: up to a period followed by a space, a newline or the end.
///
/// The periods of `e.g.` and `i.e.` end no sentence.
fn first_sentence(text: &str) -> &str {
    for (i, _) in text.match_indices('.') {
        let after = &text[i + 1..];
        let before = &text[..i];
        let ends = after.is_empty() || after.starts_with([' ', '\n']);
        if ends && !before.ends_with("e.g") && !before.ends_with("i.e") {
            return &text[..=i];
        }
    }
    text
}

/// The `name: hint` clauses of `rest` that name a required argument, in their order.
///
/// A clause ends at a `;` or at the end of its sentence (`report`'s `facts: 1-4 short lines`).
fn required_hints<'a>(rest: &'a str, schema: &Value) -> Vec<&'a str> {
    let Some(required) = schema.get("required").and_then(Value::as_array) else { return vec![] };
    let names: Vec<&str> = required.iter().filter_map(Value::as_str).collect();
    rest.split(';')
        .map(|piece| first_sentence(piece.trim()).trim_end_matches('.'))
        .filter(|clause| clause.split_once(": ").is_some_and(|(name, _)| names.contains(&name)))
        .collect()
}

/// Cuts the description to its first sentence plus the hints of required arguments, and
/// removes the schema's descriptions.
pub(crate) fn compact(spec: &ToolSpec) -> ToolSpec {
    let first = first_sentence(&spec.description);
    let hints = required_hints(&spec.description[first.len()..], &spec.schema);
    let description = if hints.is_empty() { first.to_owned() } else { format!("{first} {}.", hints.join("; ")) };
    let mut schema = spec.schema.clone();
    strip(&mut schema);
    ToolSpec { name: spec.name.clone(), description, schema }
}

/// The compact tools of the route and of `AskReport`, each once, in the order of `all`.
pub(crate) fn exposed(all: &[ToolSpec], route: &[Family]) -> Vec<ToolSpec> {
    all.iter()
        .filter(|s| families_of(&s.name).iter().any(|f| *f == Family::AskReport || route.contains(f)))
        .map(compact)
        .collect()
}

#[cfg(test)]
#[path = "compact_tests.rs"]
mod compact_tests;
