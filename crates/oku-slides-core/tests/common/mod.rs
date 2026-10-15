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

/// Le registre de base, plus les widgets de liste : `liste` (une liste `etapes`
/// aux colonnes `date` et `texte` `plain`, `note` `rich`), `bornes` (une liste
/// bornée de 1 à 2 items), `plan` (étapes `titre`, à liste imbriquée `taches` de 3 items
/// au plus, colonne `texte`).
pub fn list_registry() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    let mut probe: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} : {e}"));
    let widgets = serde_json::json!([{
        "name": "liste",
        "category": "element",
        "params": [{
            "name": "etapes",
            "type": "list",
            "item": [
                { "name": "date", "type": "plain" },
                { "name": "texte", "type": "plain", "default": "Nouvelle étape" },
                { "name": "note", "type": "rich" }
            ],
            "default": [{ "date": "2019" }]
        }]
    }, {
        "name": "bornes",
        "category": "element",
        "params": [{ "name": "bornee", "type": "list", "min": 1, "max": 2, "item": [{ "name": "t", "type": "plain" }] }]
    }, {
        "name": "plan",
        "category": "element",
        "params": [{
            "name": "etapes",
            "type": "list",
            "item": [
                { "name": "titre", "type": "plain" },
                { "name": "taches", "type": "list", "max": 3, "item": [{ "name": "texte", "type": "plain", "default": "Tâche" }],
                  "default": [{ "texte": "Première" }] }
            ],
            "default": [{ "titre": "Lancement", "taches": [{ "texte": "Cadrer" }] }]
        }]
    }]);
    let Some(base) = probe["base"].as_array_mut() else { panic!("sonde : base attendu") };
    base.extend(widgets.as_array().cloned().unwrap_or_default());
    let probe = serde_json::from_value(probe).unwrap_or_else(|e| panic!("sonde : {e}"));
    WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("sonde : {e}"))
}
