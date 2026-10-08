//! Tableaux du deck avant Typst : forme canonique, ou rien si invalides.
//!
//! Un deck ouvert n'est jamais refusé (`validate` signale seulement) : sans
//! cette passe, un tableau mal formé ou énorme ferait échouer ou traîner la
//! compilation de tout le deck. Typst affiche le substitut à la place.

use oku_slides_core::{HostType, WidgetRegistry};
use serde_json::Value as Json;

use crate::charts::for_each_element;

/// Ramène chaque paramètre `host("table")` à sa forme canonique, ou à `null`.
pub(crate) fn sanitize(deck: &mut Json, registry: &WidgetRegistry) {
    for_each_element(deck, &mut |e| {
        let kind = e.get("kind").and_then(Json::as_str).unwrap_or_default().to_owned();
        let Some(param) = registry.get(&kind).and_then(|w| w.host_param(HostType::Table)) else { return };
        let Some(Json::Object(params)) = e.get_mut("params") else { return };
        let Some(value) = params.get_mut(&param.name) else { return };
        *value = match oku_slides_chart::parse_table(value) {
            // Sérialiser une `TableOption` lue ne peut pas échouer ; au pire, le substitut.
            Ok(table) => serde_json::to_value(table).unwrap_or(Json::Null),
            Err(reason) => {
                tracing::warn!(reason = %reason, "tableau non rendu");
                Json::Null
            }
        };
    });
}

#[cfg(test)]
#[path = "tables_tests.rs"]
mod tables_tests;
