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

/// Le widget d'élément `liste` des tests : une liste `etapes` aux colonnes
/// `date` et `texte` (`plain`), `note` (`rich`) et `fait` (`flag`).
pub(crate) const LISTE: &str = r#"{
    "name": "liste",
    "category": "element",
    "params": [{
        "name": "etapes",
        "type": "list",
        "item": [
            { "name": "date", "type": "plain" },
            { "name": "texte", "type": "plain", "default": "Nouvelle étape" },
            { "name": "note", "type": "rich" },
            { "name": "fait", "type": "flag" }
        ],
        "default": [{ "date": "2019" }]
    }]
}"#;

/// Le registre de base, plus le widget [`LISTE`].
pub(crate) fn with_list() -> std::sync::Arc<WidgetRegistry> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    let mut probe: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} : {e}"));
    let liste: serde_json::Value = serde_json::from_str(LISTE).unwrap_or_else(|e| panic!("liste : {e}"));
    probe["base"].as_array_mut().unwrap_or_else(|| panic!("sonde : base attendu")).push(liste);
    std::sync::Arc::new(WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("sonde : {e}")))
}
