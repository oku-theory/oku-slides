//! Pack de widgets des tests de rendu : la frise de test (`fixtures/widgets/frise-test.typ`),
//! le plan à liste imbriquée (`fixtures/widgets/plan-test.typ`) et le rendu d'un deck d'une
//! slide avec un pack.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{
    DECK_VERSION, Deck, Namespace, PackFiles, PackKind, ThemeRef, WidgetFile,
    kinds::{base, essentiel},
};
use oku_slides_render::{
    Compiled, ElementBox, Fonts, Layout, PackSet, PackWidgets, Renderer, builtin_pack_set, load_registry,
};
use serde_json::json;

/// Source du widget de pack `frise` : liste `etapes` (date, texte), quatre étapes par défaut.
pub const FRISE_TEST: &str = include_str!("../../../../fixtures/widgets/frise-test.typ");

/// Source du widget de pack `plan` : étapes (`titre`) à tâches imbriquées (`texte`, `fait`).
pub const PLAN_TEST: &str = include_str!("../../../../fixtures/widgets/plan-test.typ");

/// Espace de noms des packs de test : leurs widgets s'appellent `test/<nom>`.
pub const TEST_NS: &str = "test";

/// Le pack de test qui porte le plan.
pub fn plan_pack() -> PackWidgets {
    pack("plan-test", &[("plan", PLAN_TEST)])
}

/// Un pack `id` d'espace de noms [`TEST_NS`], aux fichiers `files` (nom, source).
pub fn pack(id: &str, files: &[(&str, &str)]) -> PackWidgets {
    pack_in(id, TEST_NS, files)
}

/// Un pack de widgets `id` d'espace de noms `namespace`, aux fichiers `files` (nom, source).
pub fn pack_in(id: &str, namespace: &str, files: &[(&str, &str)]) -> PackWidgets {
    pack_of_kind(PackKind::Widgets, id, namespace, files)
}

/// Un pack de sorte `kind`, `id` d'espace de noms `namespace`, aux fichiers `files` (nom, source).
pub fn pack_of_kind(kind: PackKind, id: &str, namespace: &str, files: &[(&str, &str)]) -> PackWidgets {
    let namespace = Namespace::parse(namespace).unwrap_or_else(|| panic!("espace de noms invalide : {namespace}"));
    let files = files
        .iter()
        .map(|(n, s)| (WidgetFile::parse(n).unwrap_or_else(|| panic!("nom de widget invalide : {n}")), (*s).to_owned()))
        .collect();
    PackWidgets::new(id, namespace, kind, files).unwrap_or_else(|e| panic!("{e}"))
}

/// Le pack de test qui porte la frise.
pub fn frise_pack() -> PackWidgets {
    pack("frise-test", &[("frise", FRISE_TEST)])
}

/// Le kind d'un élément image.
pub const IMAGE_KIND: &str = base::IMAGE;
/// Le kind d'un élément texte.
pub const TEXT_KIND: &str = base::TEXT;

/// Un rendu aux packs intégrés : `base` et `essentiel`.
pub fn renderer() -> Renderer {
    Renderer::with_builtins(Fonts::default()).unwrap_or_else(|e| panic!("{e}"))
}

/// Un rendu aux packs intégrés et à ceux de `set`.
pub fn renderer_with(set: &PackSet) -> Renderer {
    let mut packs = set.packs().to_vec();
    packs.extend(builtin_pack_set().unwrap_or_else(|e| panic!("{e}")).packs().iter().cloned());
    let set = PackSet::new(packs).unwrap_or_else(|e| panic!("{e}"));
    let registry = Arc::new(load_registry(&Fonts::default(), &set).unwrap_or_else(|e| panic!("{e}")));
    let mut r = Renderer::new(Fonts::default()).unwrap_or_else(|e| panic!("{e}"));
    r.set_widgets(&set, registry).unwrap_or_else(|e| panic!("{e}"));
    r
}

/// Rend un deck d'une slide `essentiel/content` dont les blocs sont `blocks`, avec les packs intégrés et `packs`.
pub fn compile_with_packs(packs: &[PackWidgets], blocks: serde_json::Value) -> Compiled {
    let mut r = renderer_with(&PackSet::new(packs.to_vec()).unwrap_or_else(|e| panic!("{e}")));
    let deck = json!({
        "version": DECK_VERSION, "params": { "title": "T" },
        "slides": [{ "id": "s", "kind": essentiel::CONTENT, "params": { "blocks": blocks } }]
    });
    let deck = Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("deck invalide : {e}"));
    r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"))
}

/// Comme [`compile_with_packs`], pour la seule mise en page.
pub fn render_with_packs(packs: &[PackWidgets], blocks: serde_json::Value) -> Layout {
    compile_with_packs(packs, blocks).layout().clone()
}

/// Rend un deck d'une slide `content` dont les blocs sont `blocks`, avec les widgets de `pack`.
pub fn compile_with_pack(pack: &PackWidgets, blocks: serde_json::Value) -> Compiled {
    compile_with_packs(std::slice::from_ref(pack), blocks)
}

/// Comme [`compile_with_pack`], pour la seule mise en page.
pub fn render_with_pack(pack: &PackWidgets, blocks: serde_json::Value) -> Layout {
    render_with_packs(std::slice::from_ref(pack), blocks)
}

/// Un deck d'une slide `essentiel/blank` dont le slot `elements` tient le seul `element`.
pub fn blank_with_element(element: serde_json::Value) -> Deck {
    let deck = json!({
        "version": DECK_VERSION,
        "slides": [{ "id": "s", "kind": essentiel::BLANK, "params": { "elements": [element] } }]
    });
    Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("deck invalide : {e}"))
}

/// La boîte de l'élément `id` sur la page 1.
pub fn box_of<'a>(layout: &'a Layout, id: &str) -> &'a ElementBox {
    layout.boxes_on(1).iter().find(|b| b.id == id).unwrap_or_else(|| panic!("pas de boîte {id} en page 1"))
}

/// La taille `size.<name>` des jetons par défaut, en pt.
pub fn token_size(name: &str) -> f64 {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../typst/oku-slides/src/default-tokens.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} illisible : {e}"));
    let tokens: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} invalide : {e}"));
    tokens["size"][name].as_f64().unwrap_or_else(|| panic!("size.{name} absent"))
}

/// Le `theme.json` d'un thème intégré (`papier`, `pro`), lu dans la bibliothèque Typst.
fn theme_json(name: &str) -> String {
    let path = format!("{}/../../typst/oku-slides/themes/{name}.json", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path} illisible : {e}"))
}

/// Jetons d'un thème intégré (`papier`, `pro`), lus dans la bibliothèque Typst ; `None` sans thème.
pub fn theme_tokens(name: Option<&str>) -> Option<serde_json::Map<String, serde_json::Value>> {
    let text = theme_json(name?);
    let theme: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("thème invalide : {e}"));
    theme["tokens"].as_object().cloned()
}

/// La version d'un thème intégré : son `theme.json`, sans widget.
pub fn theme_files(name: &str) -> PackFiles {
    PackFiles::from_bytes(PackKind::Widgets, theme_json(name).as_bytes(), [])
        .unwrap_or_else(|| panic!("{name} invalide"))
}

/// La référence d'un thème intégré : son id et la version de son `theme.json`.
pub fn theme_ref(name: &str) -> ThemeRef {
    let files = theme_files(name);
    let id = files.manifest()["id"].as_str().and_then(|s| s.parse().ok()).unwrap_or_else(|| panic!("{name} sans id"));
    ThemeRef { id, version: files.version() }
}

/// Le nom du thème intégré de même id que `theme` ; `None` sans thème.
pub fn theme_name(theme: Option<&ThemeRef>) -> Option<&'static str> {
    let theme = theme?;
    ["papier", "pro"].into_iter().find(|n| theme_ref(n).id == theme.id)
}
