//! Schémas compacts : description réduite à sa première phrase, sans descriptions imbriquées.

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

/// Retire récursivement le mot-clé `description` de chaque schéma.
///
/// Les clés d'un objet `properties` sont des noms d'arguments : on les garde, même
/// une propriété nommée `description`, et on nettoie seulement le schéma de chacune.
fn strip(v: &mut Value) {
    match v {
        Value::Object(map) => {
            map.remove("description");
            for (key, child) in map.iter_mut() {
                match child {
                    Value::Object(props) if key == "properties" => props.values_mut().for_each(strip),
                    _ => strip(child),
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(strip),
        _ => {}
    }
}

/// Réduit la description à sa première phrase et retire celles du schéma.
pub(crate) fn compact(spec: &ToolSpec) -> ToolSpec {
    let description = match spec.description.find(". ") {
        Some(i) => spec.description[..=i].to_owned(),
        None => spec.description.clone(),
    };
    let mut schema = spec.schema.clone();
    strip(&mut schema);
    ToolSpec { name: spec.name.clone(), description, schema }
}

/// Les outils compacts de la route et de `AskReport`, dans l'ordre de `all`.
pub(crate) fn exposed(all: &[ToolSpec], route: &[Family]) -> Vec<ToolSpec> {
    all.iter()
        .filter(|s| families_of(&s.name).iter().any(|f| *f == Family::AskReport || route.contains(f)))
        .map(compact)
        .collect()
}

#[cfg(test)]
#[path = "compact_tests.rs"]
mod compact_tests;
