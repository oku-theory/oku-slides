//! t-1015 : un pack fautif n'écarte plus les widgets des autres packs du deck.
//!
//! Avant t-1015, un deck n'avait qu'un pack : un échec borné du rendu le
//! remplaçait par les widgets de base, tous ses widgets compris. Avec
//! plusieurs packs, le processus de rendu n'en accuse aucun ; la session
//! cherche le coupable par dichotomie et l'écarte seul.

use std::collections::BTreeMap;

use oku_slides_core::{Namespace, Op, PackPin, PackScope, WidgetFile};
use oku_slides_session::{PackLibrary, PackManifest, Request, Settings, Storage};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common::{data_dir, fixture_path, session_with_settings};

/// Le widget d'élément `note` : valide.
const GOOD_NOTE: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("note", params: (body: p.plain()), (args, ctx) => [#args.body])
"#;

/// Le widget d'élément `note` qui boucle au rendu (boucles imbriquées, hors des gardes de Typst).
const LOOPING_NOTE: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("note", params: (body: p.plain()),
  (args, ctx) => { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; [#n] })
"#;

/// Délai de rendu : bien plus qu'un rendu normal de la fixture.
const TIMEOUT_MS: u64 = 2000;

/// Ajoute à la bibliothèque de `data` le pack `namespace` au widget `note` de source `source` ; son op.
fn library_pack(library: &PackLibrary, namespace: &str, source: &str) -> Op {
    let namespace = Namespace::parse(namespace).unwrap_or_else(|| panic!("espace de noms"));
    let id = Uuid::new_v4();
    let manifest = PackManifest {
        id,
        name: namespace.to_string(),
        namespace: namespace.clone(),
        requires: Vec::new(),
        description: String::new(),
        parent: None,
        created: "2026-10-18T10:00:00Z".into(),
        scope: PackScope::Library,
        profiles: Vec::new(),
        examples: BTreeMap::new(),
        tokens: Vec::new(),
    };
    let note = WidgetFile::parse("note").unwrap_or_else(|| panic!("nom"));
    let files = manifest
        .to_files(oku_slides_core::PackKind::Widgets, BTreeMap::from([(note, source.to_owned())]))
        .unwrap_or_else(|e| panic!("{e}"));
    library.add_version(id, &files).unwrap_or_else(|e| panic!("{}", e.one_line()));
    Op::AddPack {
        kind: oku_slides_core::PackKind::Widgets,
        namespace,
        pin: PackPin { id, version: files.version(), requires: Vec::new() },
    }
}

/// Un élément `<namespace>/note` ajouté à `s-constat`.
fn note_of(namespace: &str) -> Op {
    serde_json::from_value(serde_json::json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": format!("n-{namespace}"), "kind": format!("{namespace}/note"), "params": { "body": "x" } }
    }))
    .unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn bounded_failure_among_three_packs_disables_only_the_looping_one() {
    let data = data_dir();
    let library = PackLibrary::new(
        Some(data.as_ref().join("widgets")),
        None,
        crate::common::no_user_themes(),
        Storage::local().unwrap_or_else(|e| panic!("{e}")),
    );
    let mut ops: Vec<Op> = [("alpha", GOOD_NOTE), ("beta", LOOPING_NOTE), ("gamma", GOOD_NOTE)]
        .iter()
        .map(|(n, src)| library_pack(&library, n, src))
        .collect();
    ops.extend(["alpha", "beta", "gamma"].map(note_of));
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("{e}"));
    settings.render.timeout_ms = TIMEOUT_MS;
    settings.render.probe_budget_ms = TIMEOUT_MS;
    let mut s = session_with_settings(data.as_ref(), settings);
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("{}", e.one_line()));
    s.apply("Packs", ops, None).unwrap_or_else(|e| panic!("modification refusée : {}", e.one_line()));

    let widgets: serde_json::Value =
        serde_json::from_str(&s.handle(Request::Widgets).unwrap_or_else(|e| panic!("{e}")))
            .unwrap_or_else(|e| panic!("{e}"));
    let present: Vec<bool> =
        ["alpha", "beta", "gamma"].iter().map(|n| widgets["widgets"].get(format!("{n}/note")).is_some()).collect();
    assert_that!(present).is_equal_to(vec![true, false, true]);
    let snapshot: serde_json::Value =
        serde_json::from_str(&s.handle(Request::Snapshot).unwrap_or_else(|e| panic!("{e}")))
            .unwrap_or_else(|e| panic!("{e}"));
    let notice = snapshot["notice"].as_str().unwrap_or_default();
    assert_that!(notice).contains("beta");
    assert_that!(notice.contains("alpha") || notice.contains("gamma")).is_false();
}
