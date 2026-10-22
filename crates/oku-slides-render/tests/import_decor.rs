//! Le décor que génère l'import hi-fi se compile dans un widget de slide de
//! pack. Le code compilé est celui des instantanés de `oku-slides-import`
//! (`src/hifi/typst/snapshots`) : lus ici comme des fichiers, sans dépendre
//! de la crate d'import. Ses images passent par `ctx.load`, que le rendu
//! donne aussi aux widgets de slide.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use oku_slides_core::{Deck, Namespace, PackKind, WidgetFile};
use oku_slides_render::{BuiltinPack, Fonts, PackSet, PackWidgets, Renderer, load_registry};
use speculoos::prelude::*;

/// PNG 1 × 1.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0B, 0x49,
    0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x60, 0x00, 0x02, 0x00, 0x00, 0x05, 0x00, 0x01, 0x7A, 0x5E, 0xAB, 0x3F, 0x00,
    0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// L'asset des instantanés synthétiques de l'import.
const ASSET: &str = "sha256:aa";

/// Les jetons de couleur qu'ajoute un thème généré par l'import (thème Office).
fn theme() -> serde_json::Map<String, serde_json::Value> {
    let tokens = serde_json::json!({ "color": {
        "accent-2": "#ed7d31", "accent-3": "#a5a5a5", "accent-4": "#ffc000", "accent-5": "#5b9bd5",
        "accent-6": "#70ad47", "link": "#0563c1", "link-visited": "#954f72",
    } });
    let serde_json::Value::Object(tokens) = tokens else { unreachable!() };
    tokens
}

/// Un pack de layouts d'un widget de slide `decor` qui dessine `code` (mode code, `t` et `ctx` en portée).
fn decor_pack(code: &str) -> PackWidgets {
    let source = format!(
        "#import \"/oku-slides/lib.typ\": widget, blank-slide, color-of, font-of\n\
         #let widget = widget(\"decor\", category: \"slide\", (args, ctx) => blank-slide(id: ctx.id, {{\n\
         let t = ctx.tokens\n{code}}}))\n"
    );
    let name = WidgetFile::parse("decor").unwrap_or_else(|| panic!("nom de widget"));
    let namespace = Namespace::parse("decor").unwrap_or_else(|| panic!("espace de noms"));
    PackWidgets::new("decor", namespace, PackKind::Layouts, BTreeMap::from([(name, source)]))
        .unwrap_or_else(|e| panic!("{e}"))
}

/// Le SVG de l'unique slide `decor` rendue avec le pack de `code`.
fn render(code: &str) -> String {
    let fonts = Fonts::default();
    let set = PackSet::new(vec![decor_pack(code), BuiltinPack::Base.widgets().unwrap_or_else(|e| panic!("{e}"))])
        .unwrap_or_else(|e| panic!("{e}"));
    let registry = load_registry(&fonts, &set).unwrap_or_else(|e| panic!("sonde : {e}"));
    let mut r = Renderer::new(fonts).unwrap_or_else(|e| panic!("{e}"));
    r.set_widgets(&set, Arc::new(registry)).unwrap_or_else(|e| panic!("{e}"));
    r.set_theme(Some(theme())).unwrap_or_else(|e| panic!("{e}"));
    let deck = Deck::from_json(r#"{"version": 8,"slides":[{"id":"s","kind":"decor/decor"}]}"#)
        .unwrap_or_else(|e| panic!("{e}"));
    let assets = BTreeMap::from([(ASSET.to_owned(), PNG.to_vec())]);
    let compiled = r.render(&deck, &assets).unwrap_or_else(|e| panic!("rendu : {e}"));
    compiled.svg(0).unwrap_or_else(|e| panic!("{e}"))
}

/// Le contenu d'un instantané `insta` (après son en-tête).
fn snapshot_body(text: &str) -> &str {
    text.splitn(3, "---\n").nth(2).unwrap_or_else(|| panic!("instantané sans en-tête"))
}

#[test]
fn slide_widget_loads_a_deck_asset_through_ctx_load() {
    let svg =
        render(&format!("place(box(width: 100pt, height: 100pt, (ctx.load)(asset: \"{ASSET}\", fit: \"stretch\")))\n"));
    assert_that!(svg).contains("<image");
}

#[test]
fn import_decor_snapshots_compile_in_a_slide_widget() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../oku-slides-import/src/hifi/typst/snapshots");
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} : {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "snap"))
        .collect();
    paths.sort();
    assert_that!(paths.len()).is_greater_than(10);
    for path in paths {
        let text = fs::read_to_string(&path).unwrap();
        let svg = std::panic::catch_unwind(|| render(snapshot_body(&text)))
            .unwrap_or_else(|_| panic!("{} ne se compile pas", path.display()));
        if text.contains(ASSET) {
            asserting(&format!("{} dessine son image", path.display())).that(&svg).contains("<image");
        }
    }
}
