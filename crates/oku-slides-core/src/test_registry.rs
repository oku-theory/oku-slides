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

/// Le widget d'élément `bornes` des tests : une liste bornée (1 à 2 items) et une liste à colonne `asset`.
pub(crate) const BORNES: &str = r#"{
    "name": "bornes",
    "category": "element",
    "params": [
        { "name": "bornee", "type": "list", "min": 1, "max": 2, "item": [{ "name": "t", "type": "plain" }] },
        { "name": "images", "type": "list", "item": [{ "name": "img", "type": "asset" }] }
    ]
}"#;

/// Le widget de slide `chrono` des tests : une liste `etapes` à deux colonnes `rich`.
pub(crate) const CHRONO: &str = r#"{
    "name": "chrono",
    "category": "slide",
    "params": [{
        "name": "etapes",
        "type": "list",
        "item": [{ "name": "note", "type": "rich" }, { "name": "suite", "type": "rich" }]
    }]
}"#;

/// Le widget d'élément `mesures` des tests : une liste `lignes` aux colonnes `niveau` (`choice`) et `valeur` (`number`).
pub(crate) const MESURES: &str = r#"{
    "name": "mesures",
    "category": "element",
    "params": [{
        "name": "lignes",
        "type": "list",
        "item": [
            { "name": "niveau", "type": "choice", "values": ["bas", "haut"] },
            { "name": "valeur", "type": "number" }
        ]
    }]
}"#;

/// Le registre de base, plus les widgets [`LISTE`], [`BORNES`], [`CHRONO`] et [`MESURES`].
pub(crate) fn with_list() -> std::sync::Arc<WidgetRegistry> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    let mut probe: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} : {e}"));
    let base = probe["base"].as_array_mut().unwrap_or_else(|| panic!("sonde : base attendu"));
    for widget in [LISTE, BORNES, CHRONO, MESURES] {
        base.push(serde_json::from_str(widget).unwrap_or_else(|e| panic!("{widget} : {e}")));
    }
    std::sync::Arc::new(WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("sonde : {e}")))
}
