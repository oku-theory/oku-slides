//! Registre de base pour les tests du cœur (`fixtures/widgets/base-registry.json`).

use crate::WidgetRegistry;

/// Le widget d'élément `test/frame-box` : un slot libre, pour les tests qui ont besoin de cadres sous un élément.
pub(crate) const FRAME_BOX: &str = r#"{
    "name": "frame-box",
    "category": "element",
    "params": [{ "name": "children", "type": "slot", "mode": "free" }]
}"#;

/// Le kind du widget [`FRAME_BOX`] dans le pack de test.
pub(crate) const FRAME_BOX_KIND: &str = "test/frame-box";

/// La sonde du fichier de base, plus le pack de test `test` (widget [`FRAME_BOX`]).
fn probe() -> serde_json::Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    let mut probe: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} : {e}"));
    let widget: serde_json::Value = serde_json::from_str(FRAME_BOX).unwrap_or_else(|e| panic!("{FRAME_BOX} : {e}"));
    let packs = probe["packs"].as_array_mut().unwrap_or_else(|| panic!("sonde : packs attendu"));
    packs.push(serde_json::json!({ "namespace": "test", "widgets": [widget] }));
    probe
}

pub(crate) fn registry() -> WidgetRegistry {
    WidgetRegistry::from_probe(probe()).unwrap_or_else(|e| panic!("sonde : {e}"))
}

/// Une référence de thème de test : version `<c>` répété 64 fois, id dérivé.
pub(crate) fn theme_ref(c: &str) -> crate::ThemeRef {
    let version = crate::PackVersion::from_hex(&c.repeat(64)).unwrap_or_else(|| panic!("hex invalide : {c}"));
    crate::ThemeRef { id: uuid::Uuid::from_u128(u128::from(c.as_bytes()[0])), version }
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

/// Le widget d'élément `plan` des tests : une liste `etapes` (colonne `titre`) dont chaque
/// item porte une liste imbriquée `taches` (`texte`, `fait`, `note` riche), bornée à 3 items (t-495).
pub(crate) const PLAN: &str = r#"{
    "name": "plan",
    "category": "element",
    "params": [{
        "name": "etapes",
        "type": "list",
        "item-label": "étape",
        "item": [
            { "name": "titre", "type": "plain", "default": "Étape" },
            { "name": "taches", "type": "list", "item-label": "tâche", "max": 3,
              "item": [{ "name": "texte", "type": "plain", "default": "Tâche" }, { "name": "fait", "type": "flag" },
                       { "name": "note", "type": "rich" }],
              "default": [{ "texte": "Première tâche" }] }
        ],
        "default": [{ "titre": "Lancement", "taches": [{ "texte": "Cadrer" }, { "texte": "Planifier" }] }]
    }]
}"#;

/// Le registre de base, plus les widgets [`LISTE`], [`BORNES`], [`CHRONO`], [`MESURES`] et [`PLAN`].
pub(crate) fn with_list() -> std::sync::Arc<WidgetRegistry> {
    let mut probe = probe();
    let base = probe["base"].as_array_mut().unwrap_or_else(|| panic!("sonde : base attendu"));
    for widget in [LISTE, BORNES, CHRONO, MESURES, PLAN] {
        base.push(serde_json::from_str(widget).unwrap_or_else(|e| panic!("{widget} : {e}")));
    }
    std::sync::Arc::new(WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("sonde : {e}")))
}
