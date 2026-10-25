//! t-1093 : enregistrer un pack écrivait sa version dans la bibliothèque avant d'épingler.
//!
//! Le deck mis de côté épingle déjà l'espace de noms du pack édité sous l'autre sorte (pack de
//! layouts reçu avec le fichier) : l'épinglage refuse (`NamespaceTaken`), mais la version était
//! déjà écrite, et chaque essai en ajoutait une.

use std::collections::BTreeMap;
use std::path::Path;

use oku_slides_core::{Op, PackKind, PackPin, PackScope};
use oku_slides_session::{PackLibrary, PackManifest, SessionErrorKind, Storage};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common::{data_dir, fixture_path, no_user_themes, ns, session_in};

fn library(data: &Path) -> PackLibrary {
    let storage = Storage::local().unwrap_or_else(|e| panic!("{e}"));
    PackLibrary::new(Some(data.join("widgets")), Some(data.join("layouts")), no_user_themes(), storage)
}

/// Un manifeste vide du pack `id` sous l'espace de noms `namespace`.
fn manifest(id: Uuid, namespace: &str) -> PackManifest {
    PackManifest {
        id,
        name: namespace.into(),
        namespace: ns(namespace),
        requires: vec![],
        description: String::new(),
        parent: None,
        created: "2026-10-01T00:00:00Z".into(),
        scope: PackScope::Library,
        profiles: Vec::new(),
        examples: BTreeMap::default(),
        styles: BTreeMap::default(),
        tokens: Vec::new(),
    }
}

/// Un fichier enregistré sous `them`, dont le deck épingle le pack de layouts `acme`.
fn file_pinning_layouts(them: &Path) -> std::path::PathBuf {
    let m = manifest(Uuid::new_v4(), "acme");
    let files = m.to_files(PackKind::Layouts, BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"));
    library(them).add_version(m.id, &files).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let mut s = session_in(them);
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("ouverture : {}", e.one_line()));
    let pin = PackPin { id: m.id, version: files.version(), requires: vec![] };
    let op = Op::AddPack { kind: PackKind::Layouts, namespace: ns("acme"), pin };
    s.apply("acme", vec![op], None).unwrap_or_else(|e| panic!("épinglage : {}", e.one_line()));
    let path = them.join("acme.oku-slides.pdf");
    s.save(Some(&path)).unwrap_or_else(|e| panic!("enregistrement : {}", e.one_line()));
    path
}

#[test]
fn issue_1093_pack_save_conflict_writes_no_version() {
    let (me, them) = (data_dir(), data_dir());
    let path = file_pinning_layouts(&them);
    let mine = manifest(Uuid::new_v4(), "acme");
    let files = mine.to_files(PackKind::Widgets, BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"));
    library(&me).add_version(mine.id, &files).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let mut s = session_in(&me);
    s.open(&path).unwrap_or_else(|e| panic!("ouverture : {}", e.one_line()));
    s.edit_pack(mine.id).unwrap_or_else(|e| panic!("atelier : {}", e.one_line()));
    let saved = s.save_pack(false).map_err(|e| e.kind());
    assert_that!(saved).is_equal_to(Err(SessionErrorKind::NamespaceTaken));
    assert_that!(library(&me).versions(mine.id).len()).is_equal_to(1);
}
