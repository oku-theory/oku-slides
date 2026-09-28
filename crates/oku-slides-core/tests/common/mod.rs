//! Registre de base pour les tests d'intégration du cœur
//! (`fixtures/widgets/base-registry.json`). Copie de `src/test_registry.rs`,
//! que les tests d'intégration ne voient pas (code `#[cfg(test)]`).
//!
//! Chaque binaire de test n'en utilise qu'une partie.
#![allow(dead_code)]

use oku_slides_core::WidgetRegistry;

pub fn registry() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    let probe = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} : {e}"));
    WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("{path} : {e}"))
}

/// Le registre de base, partagé comme le prennent le replica et les sources.
pub fn shared() -> std::sync::Arc<WidgetRegistry> {
    std::sync::Arc::new(registry())
}
