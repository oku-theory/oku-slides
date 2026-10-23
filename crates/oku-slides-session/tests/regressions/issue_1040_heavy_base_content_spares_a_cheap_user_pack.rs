//! t-1040 : un deck trop lourd par son contenu `base` faisait écarter le seul pack de
//! l'utilisateur, innocent.
//!
//! L'emplacement portait `base` et ce pack : le processus de rendu accusait l'unique pack hors
//! confiance, la session sautait son rendu témoin et l'écartait pour la vie du processus de
//! rendu, et le rendu refait dépassait encore le délai. Un emplacement qui porte un pack de
//! confiance n'accuse plus personne : le rendu témoin avec `base` seul échoue aussi, l'échec est
//! celui du deck.

use std::collections::BTreeMap;

use oku_slides_core::{Namespace, Op, PackPin, PackScope, WidgetFile};
use oku_slides_session::{PackLibrary, PackManifest, Request, Settings, Storage};
use serde_json::Value;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common::{data_dir, session_with_settings};
use crate::issue_1040_heavy_base_content_disables_no_pack::{TIMEOUT_MS, bounded, heavy_table_slides};

/// Le widget d'élément `note`, bon marché.
const NOTE: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("note", params: (body: p.plain()), (args, ctx) => [#args.body])
"#;

/// Ajoute à `library` le pack `leger` au widget `note` ; l'op qui l'épingle.
fn cheap_pack(library: &PackLibrary) -> Op {
    let namespace = Namespace::parse("leger").unwrap_or_else(|| panic!("espace de noms"));
    let id = Uuid::new_v4();
    let manifest = PackManifest {
        id,
        name: "Léger".into(),
        namespace: namespace.clone(),
        requires: Vec::new(),
        description: String::new(),
        parent: None,
        created: "2026-10-19T10:00:00Z".into(),
        scope: PackScope::Library,
        profiles: Vec::new(),
        examples: BTreeMap::new(),
        tokens: Vec::new(),
    };
    let note = WidgetFile::parse("note").unwrap_or_else(|| panic!("nom"));
    let files = manifest
        .to_files(oku_slides_core::PackKind::Widgets, BTreeMap::from([(note, NOTE.to_owned())]))
        .unwrap_or_else(|e| panic!("{e}"));
    library.add_version(id, &files).unwrap_or_else(|e| panic!("{}", e.one_line()));
    Op::AddPack {
        kind: oku_slides_core::PackKind::Widgets,
        namespace,
        pin: PackPin { id, version: files.version(), requires: Vec::new() },
    }
}

#[test]
fn issue_1040_deck_too_heavy_beside_a_cheap_user_pack_disables_no_pack() {
    let data = data_dir();
    let library = PackLibrary::new(
        Some(data.as_ref().join("widgets")),
        None,
        crate::common::no_user_themes(),
        Storage::local().unwrap_or_else(|e| panic!("{e}")),
    );
    let pin = cheap_pack(&library);
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("{e}"));
    settings.render.timeout_ms = TIMEOUT_MS;
    settings.render.probe_budget_ms = TIMEOUT_MS;
    let mut s = session_with_settings(data.as_ref(), settings);
    s.new_deck().unwrap_or_else(|e| panic!("{}", e.one_line()));
    s.apply("Pack", vec![pin], None).unwrap_or_else(|e| panic!("{}", e.one_line()));
    // Le contenu `base` seul dépasse le délai ; le pack `leger` n'y est pour rien.
    let err = s.apply("Lourd", heavy_table_slides(40, 100, 20), None).err();
    assert_that!(err.as_ref().and_then(bounded)).is_equal_to(Some(true));
    let widgets: Value = serde_json::from_str(&s.handle(Request::Widgets).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_that!(widgets["widgets"].get("leger/note").is_some()).is_true();
    let snapshot: Value = serde_json::from_str(&s.handle(Request::Snapshot).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_that!(snapshot["notice"].clone()).is_equal_to(Value::Null);
}
