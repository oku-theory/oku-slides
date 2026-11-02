//! Schemas for Apple's guided generation, in the JSON form `GenerationSchema` decodes.
//!
//! Apple's decoder (probed on macOS 27) is stricter than JSON Schema: every object needs a
//! `title`, an `x-order` listing all its properties and `additionalProperties: false`; an
//! `anyOf` needs a `title`; `$defs` live at the root only, each a titled object or `anyOf`
//! whose title is its key; equal titles are merged into one type; `enum` and `const` take
//! strings only; arrays need `items`; integer bounds must be integers. So every title here
//! is built from the schema path (unique), definitions are hoisted to the root under
//! `<tool>_defs_<name>` or inlined when they are not objects, and unsupported keywords
//! (`format`, `pattern`, `default`, `oneOf` → `anyOf`, …) are dropped.

use std::collections::HashMap;

use serde_json::{Map, Value, json};

use crate::compact::ToolSpec;
use crate::families::Family;

/// Title of the "any JSON value" type: equal everywhere, so merging it is harmless.
const ANY_VALUE: &str = "AnyValue";

/// The Route reply: one or two of the offered family keys.
pub(crate) fn route(offered: &[Family]) -> Value {
    let keys: Vec<&str> = offered.iter().map(|f| f.key()).collect();
    object(
        "Route",
        vec![(
            "families".to_owned(),
            json!({"type": "array", "items": {"type": "string", "enum": keys}, "minItems": 1, "maxItems": 2}),
        )],
        &["families".to_owned()],
    )
}

/// The call of `tool`: `{tool, arguments}`, its definitions hoisted to the root `$defs`.
pub(crate) fn call(tool: &ToolSpec) -> Value {
    let mut defs = Map::new();
    let prefix = sanitize(&tool.name);
    let arguments = Convert::new(&prefix, &tool.schema, &mut defs).schema(&tool.schema, &format!("{prefix}_args"));
    let props =
        vec![("tool".to_owned(), json!({"type": "string", "enum": [tool.name]})), ("arguments".to_owned(), arguments)];
    let mut out = object(&format!("{prefix}_call"), props, &["tool".to_owned(), "arguments".to_owned()]);
    if !defs.is_empty() {
        out["$defs"] = Value::Object(defs);
    }
    out
}

/// A titled object with these properties, in order, `required` limited to them.
fn object(title: &str, props: Vec<(String, Value)>, required: &[String]) -> Value {
    let order: Vec<String> = props.iter().map(|(k, _)| k.clone()).collect();
    let required: Vec<&String> = required.iter().filter(|r| order.contains(r)).collect();
    json!({
        "title": title,
        "type": "object",
        "additionalProperties": false,
        "properties": props.into_iter().collect::<Map<String, Value>>(),
        "required": required,
        "x-order": order,
    })
}

/// Any string, number or boolean: the closest Apple's schemas come to an untyped value.
fn any_value() -> Value {
    json!({"title": ANY_VALUE, "anyOf": [{"type": "string"}, {"type": "number"}, {"type": "boolean"}]})
}

/// Letters, digits and `_` only: Apple's definition names allow nothing else.
fn sanitize(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect()
}

/// What a source definition became.
enum Def {
    /// Hoisted to the root `$defs` under this name.
    Hoisted(String),
    /// Not an object: copied at each use; `true` while it is being converted.
    Inline(bool),
}

/// The conversion of one tool's schema; hoisted definitions go to `out`.
struct Convert<'a> {
    prefix: &'a str,
    source: HashMap<String, &'a Value>,
    done: HashMap<String, Def>,
    out: &'a mut Map<String, Value>,
}

impl<'a> Convert<'a> {
    fn new(prefix: &'a str, schema: &'a Value, out: &'a mut Map<String, Value>) -> Self {
        let mut source = HashMap::new();
        collect_defs(schema, &mut source);
        Self { prefix, source, done: HashMap::new(), out }
    }

    /// The schema `v` in Apple's form, titled `title` if it needs a title.
    fn schema(&mut self, v: &Value, title: &str) -> Value {
        let Value::Object(map) = v else { return any_value() };
        if let Some(Value::String(r)) = map.get("$ref") {
            return self.reference(r, title);
        }
        if let Some(Value::Array(all)) = map.get("allOf")
            && let Some(first) = all.first()
        {
            // One member is the usual wrapper of a reference; more are rare and kept to the first.
            return self.schema(first, title);
        }
        if let Some(Value::Array(branches)) = map.get("anyOf").or_else(|| map.get("oneOf")) {
            let branches: Vec<&Value> = branches.iter().filter(|b| !is_null(b)).collect();
            return self.union(&branches, title);
        }
        match map.get("type") {
            Some(Value::String(t)) => self.typed(t, map, title),
            Some(Value::Array(types)) => {
                let types: Vec<&str> = types.iter().filter_map(Value::as_str).filter(|t| *t != "null").collect();
                match types.as_slice() {
                    [] => json!({"type": "null"}),
                    [one] => self.typed(one, map, title),
                    many => {
                        let branches: Vec<Value> = many.iter().map(|t| json!({"type": t})).collect();
                        self.union(&branches.iter().collect::<Vec<_>>(), title)
                    }
                }
            }
            _ => self.untyped(map, title),
        }
    }

    /// A union of branches: one left is that branch, more become a titled `anyOf`.
    fn union(&mut self, branches: &[&Value], title: &str) -> Value {
        match branches {
            [] => any_value(),
            [one] => self.schema(one, title),
            many => {
                let converted: Vec<Value> =
                    many.iter().enumerate().map(|(i, b)| self.schema(b, &format!("{title}_{i}"))).collect();
                json!({"title": title, "anyOf": converted})
            }
        }
    }

    /// A schema without `type`: guessed from its keywords.
    fn untyped(&mut self, map: &Map<String, Value>, title: &str) -> Value {
        if map.contains_key("properties") {
            return self.typed("object", map, title);
        }
        if map.contains_key("items") {
            return self.typed("array", map, title);
        }
        match map.get("const").or_else(|| map.get("enum").and_then(|e| e.get(0))) {
            Some(Value::String(_)) => self.typed("string", map, title),
            Some(Value::Bool(_)) => json!({"type": "boolean"}),
            Some(Value::Number(_)) => json!({"type": "number"}),
            _ => any_value(),
        }
    }

    fn typed(&mut self, kind: &str, map: &Map<String, Value>, title: &str) -> Value {
        match kind {
            "object" => self.object(map, title),
            "array" => {
                let items = map.get("items").map_or_else(any_value, |i| self.schema(i, &format!("{title}_item")));
                let mut out = json!({"type": "array", "items": items});
                for key in ["minItems", "maxItems"] {
                    if let Some(n) = map.get(key).and_then(Value::as_u64) {
                        out[key] = n.into();
                    }
                }
                out
            }
            "string" => {
                if let Some(Value::String(c)) = map.get("const") {
                    return json!({"const": c});
                }
                let mut out = json!({"type": "string"});
                if let Some(Value::Array(values)) = map.get("enum")
                    && !values.is_empty()
                    && values.iter().all(Value::is_string)
                {
                    out["enum"] = Value::Array(values.clone());
                }
                out
            }
            "integer" => {
                let mut out = json!({"type": "integer"});
                if let Some(min) = map.get("minimum").and_then(Value::as_f64).and_then(|n| whole(n.ceil())) {
                    out["minimum"] = min;
                }
                if let Some(max) = map.get("maximum").and_then(Value::as_f64).and_then(|n| whole(n.floor())) {
                    out["maximum"] = max;
                }
                out
            }
            "number" => {
                let mut out = json!({"type": "number"});
                for key in ["minimum", "maximum"] {
                    if let Some(n) = map.get(key).filter(|n| n.is_number()) {
                        out[key] = n.clone();
                    }
                }
                out
            }
            "boolean" => json!({"type": "boolean"}),
            "null" => json!({"type": "null"}),
            _ => any_value(),
        }
    }

    /// An object with properties is titled; one without is a dictionary of its values.
    fn object(&mut self, map: &Map<String, Value>, title: &str) -> Value {
        let Some(Value::Object(props)) = map.get("properties") else {
            return match map.get("additionalProperties") {
                Some(Value::Bool(false)) => object(title, vec![], &[]),
                Some(extra @ Value::Object(_)) => {
                    json!({"type": "object", "additionalProperties": self.schema(extra, &format!("{title}_value"))})
                }
                _ => json!({"type": "object", "additionalProperties": any_value()}),
            };
        };
        let converted: Vec<(String, Value)> =
            props.iter().map(|(k, v)| (k.clone(), self.schema(v, &format!("{title}_{}", sanitize(k))))).collect();
        let required: Vec<String> = match map.get("required") {
            Some(Value::Array(r)) => r.iter().filter_map(Value::as_str).map(str::to_owned).collect(),
            _ => vec![],
        };
        object(title, converted, &required)
    }

    /// A `$ref`: a hoisted object's reference, or the inlined definition.
    fn reference(&mut self, reference: &str, title: &str) -> Value {
        let name = reference.rsplit('/').next().unwrap_or(reference).to_owned();
        let Some(def) = self.source.get(&name).copied() else { return any_value() };
        match self.done.get(&name) {
            Some(Def::Hoisted(hoisted)) => return json!({"$ref": format!("#/$defs/{hoisted}")}),
            // A definition that holds itself without an object in between: cut it short.
            Some(Def::Inline(true)) => return any_value(),
            Some(Def::Inline(false)) | None => {}
        }
        if titled(def) {
            let hoisted = format!("{}_defs_{}", self.prefix, sanitize(&name));
            self.done.insert(name, Def::Hoisted(hoisted.clone()));
            let converted = self.schema(def, &hoisted);
            self.out.insert(hoisted.clone(), converted);
            return json!({"$ref": format!("#/$defs/{hoisted}")});
        }
        self.done.insert(name.clone(), Def::Inline(true));
        let converted = self.schema(def, title);
        self.done.insert(name, Def::Inline(false));
        converted
    }
}

/// True if Apple needs the converted `v` titled: an object with properties, or a union.
fn titled(v: &Value) -> bool {
    let Value::Object(map) = v else { return false };
    if let Some(Value::Array(all)) = map.get("allOf")
        && let Some(first) = all.first()
    {
        return titled(first);
    }
    let union = |key: &str| {
        map.get(key).and_then(Value::as_array).is_some_and(|b| b.iter().filter(|b| !is_null(b)).count() > 1)
    };
    let types =
        map.get("type").and_then(Value::as_array).is_some_and(|t| t.iter().filter(|t| *t != "null").count() > 1);
    map.contains_key("properties") || union("anyOf") || union("oneOf") || types
}

fn is_null(v: &Value) -> bool {
    v.get("type").and_then(Value::as_str) == Some("null")
}

/// A whole `f64` as a JSON integer; `None` beyond the `i64` range (the bound is dropped).
fn whole(n: f64) -> Option<Value> {
    format!("{n:.0}").parse::<i64>().ok().map(Value::from)
}

/// Every `$defs` and `definitions` entry of `v`, at any depth; the first of a name wins.
fn collect_defs<'a>(v: &'a Value, out: &mut HashMap<String, &'a Value>) {
    match v {
        Value::Object(map) => {
            for (key, child) in map {
                if (key == "$defs" || key == "definitions")
                    && let Value::Object(defs) = child
                {
                    for (name, def) in defs {
                        out.entry(name.clone()).or_insert(def);
                    }
                }
                collect_defs(child, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|i| collect_defs(i, out)),
        _ => {}
    }
}

#[cfg(test)]
#[path = "apple_schema_tests.rs"]
mod apple_schema_tests;
