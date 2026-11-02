//! The Apple Act step in parts, so each guided reply fits the 4,096-token context.
//!
//! All the routed tools' schemas together do not fit (the `edit` family is ~13,900 tokens),
//! and `apply` alone is ~4,900. So the Apple engine first picks a tool by name (`pick`),
//! then writes its call with only that tool's schema (`apple_schema::call`). When that
//! schema stays above `FILL_BYTES`, its largest tagged union (`apply`'s `Op`) is narrowed
//! first: the model picks 1-3 tags (`kinds`) and the call schema keeps only those branches
//! and the definitions they reach (`narrow`).

use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use crate::compact::ToolSpec;

/// Above this size, a call schema is narrowed: ~1,800 tokens at ~3.4 bytes per token.
pub(crate) const FILL_BYTES: usize = 6_000;

/// Prefix of a reference to a root definition.
const DEFS: &str = "#/$defs/";

/// What the pick step chose.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Pick {
    /// One of the offered tools.
    Tool(String),
    /// No offered tool, with the reason in English.
    Invalid(String),
}

/// A titled object with the one required property `key`.
fn single(title: &str, key: &str, value: &Value) -> Value {
    json!({
        "title": title,
        "type": "object",
        "additionalProperties": false,
        "properties": {key: value},
        "required": [key],
        "x-order": [key],
    })
}

/// The pick schema: `{tool}`, one of the names of `tools`.
pub(crate) fn pick(tools: &[ToolSpec]) -> Value {
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    single("Pick", "tool", &json!({"type": "string", "enum": names}))
}

/// The tool the pick reply names, if `tools` has it.
pub(crate) fn parse_pick(json: &str, tools: &[ToolSpec]) -> Pick {
    let value: Value = match serde_json::from_str(json) {
        Ok(value) => value,
        Err(e) => return Pick::Invalid(format!("not valid JSON: {e}")),
    };
    match value.get("tool").and_then(Value::as_str) {
        Some(name) if tools.iter().any(|t| t.name == name) => Pick::Tool(name.to_owned()),
        Some(name) => Pick::Invalid(format!("unknown tool {name}")),
        None => Pick::Invalid("the reply needs a \"tool\"".to_owned()),
    }
}

/// The tag of a union branch: the value of its one `const` string property.
fn tag(branch: &Value) -> Option<&str> {
    let props = branch.get("properties")?.as_object()?;
    let mut tags = props.values().filter_map(|p| p.get("const").and_then(Value::as_str));
    let first = tags.next()?;
    tags.next().is_none().then_some(first)
}

/// The largest root definition that is a union of tagged objects, and its tags in order.
pub(crate) fn tagged_union(schema: &Value) -> Option<(String, Vec<String>)> {
    let defs = schema.get("$defs")?.as_object()?;
    defs.iter()
        .filter_map(|(name, def)| {
            let branches = def.get("anyOf")?.as_array()?;
            let tags: Option<Vec<String>> = branches.iter().map(|b| tag(b).map(str::to_owned)).collect();
            Some((name.clone(), tags.filter(|t| t.len() > 1)?, def.to_string().len()))
        })
        .max_by_key(|(_, _, size)| *size)
        .map(|(name, tags, _)| (name, tags))
}

/// The kinds schema: `{kinds}`, one to three of `tags`.
pub(crate) fn kinds(tags: &[String]) -> Value {
    let items = json!({"type": "array", "items": {"type": "string", "enum": tags}, "minItems": 1, "maxItems": 3});
    single("Kinds", "kinds", &items)
}

/// The offered tags the kinds reply names; none if it is not valid.
pub(crate) fn parse_kinds(json: &str, tags: &[String]) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(json) else { return vec![] };
    let named = value.get("kinds").and_then(Value::as_array).cloned().unwrap_or_default();
    named.iter().filter_map(Value::as_str).filter(|t| tags.iter().any(|k| k == t)).map(str::to_owned).collect()
}

/// `schema` with union `def` cut to the branches tagged `kept`, and only the reachable definitions.
///
/// No branch tagged `kept`: the union stays whole, and an overflow then fails the turn cleanly.
pub(crate) fn narrow(schema: &Value, def: &str, kept: &[String]) -> Value {
    let mut out = schema.clone();
    if let Some(Value::Array(branches)) = out.pointer_mut(&format!("/$defs/{def}/anyOf")) {
        let left: Vec<Value> =
            branches.iter().filter(|b| tag(b).is_some_and(|t| kept.iter().any(|k| k == t))).cloned().collect();
        if !left.is_empty() {
            *branches = left;
        }
    }
    prune(&mut out);
    out
}

/// Removes the root definitions that nothing outside them reaches.
fn prune(schema: &mut Value) {
    let Some(Value::Object(defs)) = schema.get("$defs").cloned() else { return };
    let mut body = schema.clone();
    if let Value::Object(root) = &mut body {
        root.remove("$defs");
    }
    let mut reached = BTreeSet::new();
    let mut todo = Vec::new();
    refs(&body, &mut todo);
    while let Some(name) = todo.pop() {
        if reached.insert(name.clone())
            && let Some(def) = defs.get(&name)
        {
            refs(def, &mut todo);
        }
    }
    let kept: Map<String, Value> = defs.into_iter().filter(|(name, _)| reached.contains(name)).collect();
    if let Value::Object(root) = schema {
        // Like `apple_schema::act`: no `$defs` at all rather than an empty one.
        if kept.is_empty() {
            root.remove("$defs");
        } else {
            root.insert("$defs".into(), Value::Object(kept));
        }
    }
}

/// The definition names `v` refers to, at any depth.
fn refs(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            if let Some(name) = map.get("$ref").and_then(Value::as_str).and_then(|r| r.strip_prefix(DEFS)) {
                out.push(name.to_owned());
            }
            map.values().for_each(|c| refs(c, out));
        }
        Value::Array(items) => items.iter().for_each(|c| refs(c, out)),
        _ => {}
    }
}

#[cfg(test)]
#[path = "apple_steps_tests.rs"]
mod apple_steps_tests;
