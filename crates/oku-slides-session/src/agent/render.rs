//! `render` : une composition libre du catalogue v3 par l'agent, validée
//! ici. Une erreur revient à l'agent (`AgentForm`), lisible, pour qu'il corrige.

use std::collections::{HashMap, HashSet};

use oku_slides_a2ui::{Component, ComponentKind, Dynamic, Tone};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::ensure;

use super::bounds::{LABEL_MAX, TEXT_MAX, WHY_MAX, check_text};
use super::forms::{Place, Surface, surface};
use crate::Result;
use crate::error::AgentFormSnafu;

/// `render` : une carte composée par l'agent, figée dans son tour.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Render {
    /// Components of the Oku Slides A2UI catalog v3 (A2UI v0.9 shape: {id, component, …}); one has id "root".
    /// Values are literal: no {path} bindings.
    pub components: Vec<serde_json::Value>,
}
/// Composants au plus dans un appel `render`.
pub const RENDER_MAX: usize = 40;
/// Profondeur maximale de l'arbre (`root` compte pour 1).
pub const RENDER_DEPTH: usize = 6;
/// Taille JSON maximale des composants d'un appel.
pub const RENDER_BYTES: usize = 64 * 1024;

/// Refus lisible par l'agent.
fn refuse<T>(reason: String) -> Result<T> {
    Err(AgentFormSnafu { reason }.build().into())
}

/// Une clé de `given` absente de `known`, en ignorant les `null` explicites
/// (une propriété facultative écrite `null` n'est pas inconnue).
fn unknown_key<'a>(given: &'a Value, known: &Value) -> Option<&'a String> {
    let (Value::Object(given), Value::Object(known)) = (given, known) else { return None };
    given.iter().find(|(k, v)| !v.is_null() && !known.contains_key(*k)).map(|(k, _)| k)
}

/// Un objet `{"path": …}` quelque part dans `v` : une liaison au modèle de données.
fn has_path(v: &Value) -> bool {
    match v {
        Value::Object(map) => (map.len() == 1 && map.contains_key("path")) || map.values().any(has_path),
        Value::Array(items) => items.iter().any(has_path),
        _ => false,
    }
}

/// Les styles réservés à l'hôte : un agent ne peut pas imiter une demande de permission.
fn check_host_styles(i: usize, c: &Component) -> Result<()> {
    let reserved = matches!(
        &c.kind,
        ComponentKind::Button { primary: Some(true), .. }
            | ComponentKind::Button { danger: Some(true), .. }
            | ComponentKind::Card { tone: Some(Tone::Danger), .. }
            | ComponentKind::Badge { tone: Some(Tone::Danger), .. }
    );
    ensure!(
        !reserved,
        AgentFormSnafu { reason: format!("components[{i}]: primary/danger styles are reserved to Oku Slides") }
    );
    Ok(())
}

/// Texte littéral : une liaison ici est refusée (fail closed), jamais ignorée.
fn lit(i: usize, d: &Dynamic<String>) -> Result<&str> {
    match d {
        Dynamic::Literal(s) => Ok(s),
        Dynamic::Path { .. } => refuse(format!("components[{i}]: values must be literal (no {{path}} bindings)")),
    }
}

/// Les longueurs de chaque texte visible ; sans `_` : un nouveau composant doit dire comment il se borne.
fn check_content(i: usize, c: &Component) -> Result<()> {
    let at = |what: &str| format!("components[{i}].{what}");
    match &c.kind {
        ComponentKind::Text { text, .. } | ComponentKind::Markdown { text } => {
            check_text(&at("text"), lit(i, text)?, WHY_MAX)
        }
        ComponentKind::Badge { text, .. } => check_text(&at("text"), lit(i, text)?, TEXT_MAX),
        ComponentKind::Disclosure { summary, .. } => check_text(&at("summary"), lit(i, summary)?, TEXT_MAX),
        ComponentKind::Button { label, .. } => check_text(&at("label"), lit(i, label)?, LABEL_MAX),
        ComponentKind::DiffText { segments } => {
            let all: String = segments.iter().map(|s| s.text.as_str()).collect();
            check_text(&at("segments"), &all, WHY_MAX)
        }
        ComponentKind::Column { .. } | ComponentKind::Row { .. } | ComponentKind::Card { .. } => Ok(()),
        // Refusés avant : `check_available` et `agent_allowed`.
        ComponentKind::TextField { .. }
        | ComponentKind::RichTextField { .. }
        | ComponentKind::Stepper { .. }
        | ComponentKind::Slider { .. }
        | ComponentKind::ColorField { .. }
        | ComponentKind::Select { .. }
        | ComponentKind::ChoiceChips { .. }
        | ComponentKind::SlideThumb { .. }
        | ComponentKind::Prompt { .. }
        | ComponentKind::Timeline { .. }
        | ComponentKind::Receipt { .. } => refuse(format!("components[{i}]: component not available in render")),
    }
}

/// `render` ne propose que les composants de sa description : les champs vont dans `show_params`.
fn check_available(i: usize, c: &Component, name: &str) -> Result<()> {
    let field = matches!(
        c.kind,
        ComponentKind::TextField { .. }
            | ComponentKind::RichTextField { .. }
            | ComponentKind::Stepper { .. }
            | ComponentKind::Slider { .. }
            | ComponentKind::ColorField { .. }
            | ComponentKind::Select { .. }
            | ComponentKind::ChoiceChips { .. }
    );
    ensure!(
        !field,
        AgentFormSnafu {
            reason: format!("components[{i}]: {name} is not available in render (use show_params for settings)")
        }
    );
    Ok(())
}

fn parse(i: usize, v: &Value) -> Result<Component> {
    ensure!(
        !has_path(v),
        AgentFormSnafu { reason: format!("components[{i}]: values must be literal (no {{path}} bindings)") }
    );
    let c: Component = match serde_json::from_value(v.clone()) {
        Ok(c) => c,
        Err(e) => return refuse(format!("components[{i}]: {e}")),
    };
    let back = serde_json::to_value(&c).unwrap_or(Value::Null);
    if let Some(extra) = unknown_key(v, &back) {
        return refuse(format!("components[{i}]: unknown property {extra:?}"));
    }
    if !c.agent_allowed() {
        let name = back.get("component").and_then(Value::as_str).unwrap_or_default();
        return refuse(format!("components[{i}]: {name} is reserved to Oku Slides"));
    }
    // Le composant relu : un `{path, …}` à clés en trop devient `{path}` et se voit ici.
    ensure!(
        !has_path(&back),
        AgentFormSnafu { reason: format!("components[{i}]: values must be literal (no {{path}} bindings)") }
    );
    let name = back.get("component").and_then(Value::as_str).unwrap_or_default();
    check_available(i, &c, name)?;
    check_host_styles(i, &c)?;
    for a in c.kind.actions() {
        let ComponentKind::Button { label: Dynamic::Literal(label), .. } = &c.kind else {
            return refuse(format!("components[{i}]: only buttons may carry an action"));
        };
        let prompt = match a.event.context.get("prompt") {
            Some(Dynamic::Literal(Value::String(p))) if a.event.context.len() == 1 && a.event.name == "suggest" => p,
            _ => return refuse(format!("components[{i}]: only suggest actions with a literal prompt are allowed")),
        };
        // Ce que l'on voit est ce qui part : le bouton porte exactement la consigne qu'il envoie.
        ensure!(
            label == prompt,
            AgentFormSnafu { reason: format!("components[{i}]: a suggest button must send its own label as prompt") }
        );
    }
    check_content(i, &c)?;
    Ok(c)
}

/// Vérifie que les composants forment un arbre enraciné en `root` : chaque
/// composant a au plus un parent et n'est visité qu'une fois (pas de partage,
/// pas de cycle), la profondeur est bornée, aucun composant n'est orphelin.
/// Parcours itératif en largeur : au plus `components.len()` visites, quelle
/// que soit la longueur des listes `children` (pas d'explosion combinatoire).
fn check_tree(components: &[Component]) -> Result<()> {
    let mut by_id: HashMap<&str, &Component> = HashMap::with_capacity(components.len());
    for c in components {
        if by_id.insert(c.id.as_str(), c).is_some() {
            return refuse(format!("duplicate component id {:?}", c.id));
        }
    }
    ensure!(by_id.contains_key("root"), AgentFormSnafu { reason: "no component with id \"root\"".to_owned() });
    let mut seen: HashSet<&str> = HashSet::with_capacity(components.len());
    let mut level: Vec<&str> = vec!["root"];
    let mut depth = 0;
    while !level.is_empty() {
        depth += 1;
        ensure!(
            depth <= RENDER_DEPTH,
            AgentFormSnafu { reason: format!("deeper than {RENDER_DEPTH} levels (level {depth}, at {:?})", level[0]) }
        );
        let mut next = Vec::new();
        for id in level {
            if !seen.insert(id) {
                return refuse(format!("component {id:?} is used more than once (the components must form a tree)"));
            }
            let Some(c) = by_id.get(id) else { return refuse(format!("unknown child {id:?}")) };
            next.extend(c.kind.children());
        }
        level = next;
    }
    if let Some(orphan) = components.iter().find(|c| !seen.contains(c.id.as_str())) {
        return refuse(format!("component {:?} is not reachable from \"root\"", orphan.id));
    }
    Ok(())
}

/// La surface d'une composition libre, ou pourquoi elle est refusée.
pub fn render(id: &str, spec: &Render) -> Result<Surface> {
    // Le nombre d'abord : on ne sérialise pas une liste démesurée pour la mesurer.
    ensure!(
        spec.components.len() <= RENDER_MAX,
        AgentFormSnafu { reason: format!("at most {RENDER_MAX} components, got {}", spec.components.len()) }
    );
    let bytes = serde_json::to_vec(spec.components.as_slice()).map_or(usize::MAX, |b| b.len());
    ensure!(
        bytes <= RENDER_BYTES,
        AgentFormSnafu { reason: format!("at most {RENDER_BYTES} bytes of components, got {bytes}") }
    );
    let components: Vec<Component> =
        spec.components.iter().enumerate().map(|(i, v)| parse(i, v)).collect::<Result<_>>()?;
    check_tree(&components)?;
    let mut s = surface(id, Place::Thread, None, components, json!({}));
    s.live = false;
    Ok(s)
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod render_tests;
