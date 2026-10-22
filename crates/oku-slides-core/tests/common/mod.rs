//! Registre de base pour les tests d'intégration du cœur
//! (`fixtures/widgets/builtin-registry.json`). Copie de `src/test_registry.rs`,
//! que les tests d'intégration ne voient pas (code `#[cfg(test)]`).
//!
//! Chaque binaire de test n'en utilise qu'une partie.
#![allow(dead_code)]

use oku_slides_core::WidgetRegistry;

/// La sonde du fichier des packs intégrés, plus le pack de test `test` : son widget d'élément
/// `frame-box` a un slot libre `children` (cadres sous un élément).
fn probe() -> serde_json::Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/builtin-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    let mut probe: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} : {e}"));
    let widget = serde_json::json!({
        "name": "frame-box",
        "category": "element",
        "params": [{ "name": "children", "type": "slot", "mode": "free" }]
    });
    let Some(packs) = probe["packs"].as_array_mut() else { panic!("sonde : packs attendu") };
    packs.push(serde_json::json!({ "namespace": "test", "kind": "widgets", "widgets": [widget] }));
    probe
}

/// Le kind du widget `frame-box` du pack de test.
pub const FRAME_BOX_KIND: &str = "test/frame-box";

pub fn registry() -> WidgetRegistry {
    WidgetRegistry::from_probe(probe()).unwrap_or_else(|e| panic!("sonde : {e}"))
}

/// Le registre intégré, partagé comme le prennent le replica et les sources.
pub fn shared() -> std::sync::Arc<WidgetRegistry> {
    std::sync::Arc::new(registry())
}

/// Le registre intégré, plus les widgets de liste du pack `test-el` : `liste` (une liste `etapes`
/// aux colonnes `date` et `texte` `plain`, `note` `rich`), `bornes` (une liste
/// bornée de 1 à 2 items), `plan` (étapes `titre`, à liste imbriquée `taches` de 3 items
/// au plus, colonne `texte`).
pub fn list_registry() -> WidgetRegistry {
    let mut probe = probe();
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
    let Some(packs) = probe["packs"].as_array_mut() else { panic!("sonde : packs attendu") };
    packs.push(serde_json::json!({ "namespace": "test-el", "kind": "widgets", "widgets": widgets }));
    let probe = serde_json::from_value(probe).unwrap_or_else(|e| panic!("sonde : {e}"));
    WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("sonde : {e}"))
}
