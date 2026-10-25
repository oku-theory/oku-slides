//! t-329 : un widget cassé d'un pack n'empêche pas de rendre les autres.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use common::{deck_json, oku_slides, workdir};
use oku_slides_core::{PackPin, WidgetFile};
use oku_slides_session::{PackLibrary, PackManifest, Storage};
use speculoos::prelude::*;

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

/// Ajoute à la bibliothèque de `dir` un pack avec les fichiers `(nom, source)`.
fn user_pack(dir: &std::path::Path, widgets: &[(&str, &str)]) -> PackPin {
    let id = uuid::Uuid::new_v4();
    let manifest = PackManifest {
        id,
        name: "Pack".to_owned(),
        namespace: oku_slides_core::Namespace::parse("pack").unwrap_or_else(|| panic!("espace de noms")),
        requires: vec![],
        description: String::new(),
        parent: None,
        created: "2026-10-01T00:00:00Z".to_owned(),
        scope: oku_slides_core::PackScope::Library,
        profiles: Vec::new(),
        examples: std::collections::BTreeMap::default(),
        styles: std::collections::BTreeMap::default(),
        tokens: Vec::new(),
    };
    let widgets = widgets
        .iter()
        .map(|(name, source)| {
            let file = WidgetFile::parse(name).unwrap_or_else(|| panic!("nom de widget invalide : {name}"));
            (file, (*source).to_owned())
        })
        .collect();
    let files =
        manifest.to_files(oku_slides_core::PackKind::Widgets, widgets).unwrap_or_else(|e| panic!("pack.json : {e}"));
    let storage = Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    let themes = oku_slides_session::ThemeLibrary::new(Some(dir.join("data").join("themes")), storage.clone());
    let library = PackLibrary::new(
        Some(dir.join("data").join("widgets")),
        Some(dir.join("data").join("layouts")),
        themes,
        storage,
    );
    library.add_version(id, &files).unwrap_or_else(|e| panic!("bibliothèque : {e}"));
    PackPin { id, version: files.version(), requires: vec![] }
}

#[test]
fn save_of_a_deck_whose_pack_has_a_broken_widget_renders_the_others() {
    let dir = workdir("widget-casse");
    let r = user_pack(&dir, &[("quote", QUOTE), ("casse", "#let widget = (")]);
    let deck = dir.join("deck.json");
    let json = deck_json(
        serde_json::json!({ "pack": r }),
        serde_json::json!([{ "id": "s", "kind": oku_slides_core::kinds::essentiel::CONTENT, "params": { "title": "T", "blocks": [
            { "id": "q", "kind": "pack/quote", "params": { "body": [{ "runs": [{ "t": "Tout coule." }] }], "source": "H" } }
        ] } }]),
    );
    std::fs::write(&deck, serde_json::to_vec(&json).unwrap()).unwrap();
    let pdf = dir.join("deck.oku-slides.pdf");
    let saved = oku_slides(&dir, "en").arg("save").arg(&deck).arg(&pdf).output().unwrap();
    assert_that!(saved.status.success()).is_true();
    assert_that!(String::from_utf8_lossy(&saved.stderr).into_owned()).contains("casse");
    let out = dir.join("out");
    let rendered = oku_slides(&dir, "en").arg("render").arg(&pdf).arg("--out").arg(&out).output().unwrap();
    assert_that!(rendered.status.success()).is_true();
}
