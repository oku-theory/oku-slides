//! The agent's reference: the catalog schema, cut to what the agent examples use.
//!
//! Only components with an example `render` accepts appear, with their required props and the
//! optional props and enum values those examples set: the agent never reads what `render` refuses.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use schemars::Schema;
use schemars::generate::SchemaSettings;
use schemars::transform::{Transform, transform_subschemas};
use serde::Serialize;
use serde_json::{Map, Value};
use snafu::{Backtrace, OptionExt, ResultExt, Snafu, ensure};

use super::Example;
use crate::component::{Component, ComponentKind};

/// One prop of a component, as the agent reads it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PropReference {
    /// The prop's key.
    pub name: String,
    /// Whether every component must set it.
    pub required: bool,
    /// Its type: `string`, `integer`, `boolean`, `[DiffSegment]`, `Action`, `any`…
    #[serde(rename = "type")]
    pub ty: String,
    /// The string values it takes, cut to those the agent examples use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    /// The first paragraph of its doc comment, without rustdoc links.
    pub description: String,
}

/// One example of a component, as a ready `render` input.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExampleReference {
    /// The variant shown.
    pub variant: &'static str,
    /// When to use it.
    pub note: &'static str,
    /// The components to pass to `render`, root first.
    pub surface: Vec<Component>,
}

/// A component `render` accepts: its props and its examples.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComponentReference {
    /// The v0.9 component name.
    pub component: String,
    /// Its props, in declaration order.
    pub props: Vec<PropReference>,
    /// Its agent examples.
    pub examples: Vec<ExampleReference>,
}

/// The catalog schema could not be read into the agent reference.
#[derive(Debug, Snafu)]
pub struct PlaybookError(Inner);

#[derive(Debug, Snafu)]
enum Inner {
    #[snafu(display("the catalog schema has no oneOf"))]
    NoBranches { backtrace: Backtrace },
    #[snafu(display("a catalog schema branch names no component"))]
    Unnamed { backtrace: Backtrace },
    #[snafu(display("unresolved schema reference {reference}"))]
    Unresolved { reference: String, backtrace: Backtrace },
    #[snafu(display("{component}.{prop} has no description"))]
    NoDescription { component: String, prop: String, backtrace: Backtrace },
    #[snafu(display("an agent example names {component}, absent from the schema"))]
    MissingBranch { component: String, backtrace: Backtrace },
    #[snafu(display("cannot serialize an example component"))]
    ToJson { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("a prop schema the reference cannot read: {shape}"))]
    UnknownShape { shape: String, backtrace: Backtrace },
    #[snafu(display("schema references nest deeper than {MAX_DEPTH}"))]
    TooDeep { backtrace: Backtrace },
}

/// How deep `describe` follows `$ref`s and unions; the catalog needs 3.
const MAX_DEPTH: usize = 8;

/// Rewrites every description to its first paragraph, without rustdoc links.
#[derive(Debug, Clone)]
struct CleanDocs;

impl Transform for CleanDocs {
    fn transform(&mut self, schema: &mut Schema) {
        if let Some(Value::String(doc)) = schema.get_mut("description") {
            *doc = clean_doc(doc);
        }
        transform_subschemas(self, schema);
    }
}

/// The first paragraph of `doc` on one line, ``[`X`](path)`` and ``[`X`]`` written `` `X` ``.
fn clean_doc(doc: &str) -> String {
    let first = doc.split("\n\n").next().unwrap_or_default();
    let line = first.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::with_capacity(line.len());
    let mut rest = line.as_str();
    while let Some(start) = rest.find("[`") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find("`]") else {
            out.push_str(&rest[start..]);
            return out;
        };
        out.push_str(&after[..=end]);
        rest = &after[end + 2..];
        if let Some(target) = rest.strip_prefix('(')
            && let Some(close) = target.find(')')
        {
            rest = &target[close + 1..];
        }
    }
    out.push_str(rest);
    out
}

/// The catalog schema with clean descriptions.
fn catalog_schema() -> Value {
    SchemaSettings::draft2020_12()
        .with_transform(CleanDocs)
        .into_generator()
        .into_root_schema_for::<ComponentKind>()
        .to_value()
}

/// The reference of every component `render` accepts, in catalog order.
///
/// Built once: the schema and the examples are constants of the build.
///
/// # Errors
///
/// [`PlaybookError`] if the catalog schema cannot be read (a programming fault).
pub fn agent_reference() -> Result<&'static [ComponentReference], &'static PlaybookError> {
    static REFERENCE: OnceLock<Result<Vec<ComponentReference>, PlaybookError>> = OnceLock::new();
    REFERENCE.get_or_init(|| Ok(reference_from(&catalog_schema(), &super::examples())?)).as_ref().map(Vec::as_slice)
}

/// The reference that `schema` and `examples` give.
fn reference_from(schema: &Value, examples: &[Example]) -> Result<Vec<ComponentReference>, Inner> {
    let empty = Map::new();
    let defs = schema.get("$defs").and_then(Value::as_object).unwrap_or(&empty);
    let branches = schema.get("oneOf").and_then(Value::as_array).context(NoBranchesSnafu)?;
    let mut out = Vec::new();
    let mut named = BTreeSet::new();
    for branch in branches {
        let name = branch_name(branch).context(UnnamedSnafu)?;
        named.insert(name.to_owned());
        let agent: Vec<&Example> = examples.iter().filter(|e| e.agent && e.component == name).collect();
        if agent.is_empty() {
            continue;
        }
        let used = used_values(name, &agent)?;
        let required: BTreeSet<&str> =
            branch.get("required").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).collect();
        let mut props = Vec::new();
        for (prop, sub) in branch.get("properties").and_then(Value::as_object).into_iter().flatten() {
            let is_required = required.contains(prop.as_str());
            if prop == "component" || (!is_required && !used.contains_key(prop)) {
                continue;
            }
            let (ty, all_values) = describe(sub, defs, 0)?;
            let values = all_values.map(|v| match used.get(prop) {
                Some(seen) => v.into_iter().filter(|x| seen.contains(x)).collect(),
                None => v,
            });
            let description = description(sub, defs)?
                .filter(|d| !d.is_empty())
                .context(NoDescriptionSnafu { component: name.to_owned(), prop: prop.clone() })?;
            props.push(PropReference { name: prop.clone(), required: is_required, ty, values, description });
        }
        let examples = agent
            .iter()
            .map(|e| ExampleReference { variant: e.variant, note: e.note, surface: e.components.clone() })
            .collect();
        out.push(ComponentReference { component: name.to_owned(), props, examples });
    }
    if let Some(e) = examples.iter().find(|e| e.agent && !named.contains(e.component)) {
        return MissingBranchSnafu { component: e.component.to_owned() }.fail();
    }
    Ok(out)
}

/// The component name of a `oneOf` branch (`const`, or a one-value `enum`).
pub(crate) fn branch_name(branch: &Value) -> Option<&str> {
    let tag = branch.get("properties")?.get("component")?;
    tag.get("const").and_then(Value::as_str).or_else(|| tag.get("enum")?.get(0)?.as_str())
}

/// Per prop set (non-null) by a `name` component of the agent examples: the string values seen.
fn used_values(name: &str, agent: &[&Example]) -> Result<BTreeMap<String, BTreeSet<String>>, Inner> {
    let mut used: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for e in agent {
        for c in &e.components {
            let v = serde_json::to_value(c).context(ToJsonSnafu)?;
            if v.get("component").and_then(Value::as_str) != Some(name) {
                continue;
            }
            for (k, x) in v.as_object().into_iter().flatten().filter(|(_, x)| !x.is_null()) {
                let seen = used.entry(k.clone()).or_default();
                if let Some(s) = x.as_str() {
                    seen.insert(s.to_owned());
                }
            }
        }
    }
    Ok(used)
}

/// The schema `#/$defs/<name>` points to, and that name.
fn resolve<'a>(reference: &str, defs: &'a Map<String, Value>) -> Result<(&'a str, &'a Value), Inner> {
    let name = reference.strip_prefix("#/$defs/").context(UnresolvedSnafu { reference })?;
    defs.get_key_value(name).map(|(k, v)| (k.as_str(), v)).context(UnresolvedSnafu { reference })
}

/// A prop's own description, else the description of the type it points to.
fn description(sub: &Value, defs: &Map<String, Value>) -> Result<Option<String>, Inner> {
    if let Some(d) = sub.get("description").and_then(Value::as_str) {
        return Ok(Some(d.to_owned()));
    }
    match sub.get("$ref").and_then(Value::as_str) {
        Some(r) => Ok(resolve(r, defs)?.1.get("description").and_then(Value::as_str).map(str::to_owned)),
        None => Ok(None),
    }
}

/// `{path}`: the data-model branch of a `Dynamic`.
fn is_path(s: &Value) -> bool {
    s.get("properties").and_then(Value::as_object).is_some_and(|p| p.len() == 1 && p.contains_key("path"))
}

/// The type a schema reads as, and its string values when it is a string enum.
///
/// `any` only for a schema that accepts any JSON (`true` or `{}`); a shape it cannot read
/// is an error, so a new prop type never reaches the agent as a vague `any`.
fn describe(s: &Value, defs: &Map<String, Value>, depth: usize) -> Result<(String, Option<Vec<String>>), Inner> {
    ensure!(depth < MAX_DEPTH, TooDeepSnafu);
    let unknown = || UnknownShapeSnafu { shape: s.to_string() };
    if s == &Value::Bool(true) || s.as_object().is_some_and(Map::is_empty) {
        return Ok(("any".into(), None));
    }
    if let Some(r) = s.get("$ref").and_then(Value::as_str) {
        let (name, target) = resolve(r, defs)?;
        if target.get("properties").is_some() {
            return Ok((name.to_owned(), None));
        }
        return describe(target, defs, depth + 1);
    }
    if let Some(branches) = s.get("anyOf").or_else(|| s.get("oneOf")).and_then(Value::as_array) {
        let kept: Vec<&Value> =
            branches.iter().filter(|b| b.get("type").and_then(Value::as_str) != Some("null") && !is_path(b)).collect();
        let consts: Option<Vec<String>> =
            kept.iter().map(|b| b.get("const").and_then(Value::as_str).map(str::to_owned)).collect();
        if let Some(values) = consts.filter(|v| !v.is_empty()) {
            return Ok(("string".into(), Some(values)));
        }
        return match kept.as_slice() {
            [one] => describe(one, defs, depth + 1),
            _ => unknown().fail(),
        };
    }
    if let Some(values) = s.get("enum").and_then(Value::as_array) {
        return Ok(("string".into(), Some(values.iter().filter_map(Value::as_str).map(str::to_owned).collect())));
    }
    let ty = match s.get("type") {
        Some(Value::String(t)) => t.clone(),
        Some(Value::Array(ts)) => {
            ts.iter().filter_map(Value::as_str).find(|t| *t != "null").context(unknown())?.to_owned()
        }
        _ => return unknown().fail(),
    };
    if ty == "array" {
        let (item, _) = describe(s.get("items").context(unknown())?, defs, depth + 1)?;
        return Ok((format!("[{item}]"), None));
    }
    Ok((ty, None))
}

/// The reference as `Name{prop, prop?: a|b}`, comma-separated: `render`'s component list.
#[must_use]
pub fn render_components(reference: &[ComponentReference]) -> String {
    reference
        .iter()
        .map(|c| {
            let props: Vec<String> = c
                .props
                .iter()
                .map(|p| {
                    let mark = if p.required { "" } else { "?" };
                    match &p.values {
                        Some(v) => format!("{}{mark}: {}", p.name, v.join("|")),
                        None => format!("{}{mark}", p.name),
                    }
                })
                .collect();
            format!("{}{{{}}}", c.component, props.join(", "))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
#[path = "reference_tests.rs"]
mod tests;
