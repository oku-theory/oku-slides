//! Registre de base pour les tests du cœur (`fixtures/widgets/base-registry.json`).

use crate::WidgetRegistry;

pub(crate) fn registry() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    let probe = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} : {e}"));
    WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("{path} : {e}"))
}

/// Le registre de base, partagé comme le prennent le replica et les sources.
pub(crate) fn shared() -> std::sync::Arc<WidgetRegistry> {
    std::sync::Arc::new(registry())
}
