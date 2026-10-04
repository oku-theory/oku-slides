//! t-328 : un deck reçu avec le même id de pack mais un autre contenu ne
//! change jamais la version courante de la bibliothèque.
//!
//! Avant t-328, un pack était un dossier nommé par son slug : recevoir un
//! deck dont le pack portait le même nom revenait à remplacer le sien.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oku_slides_core::{Op, PackFiles, PackRef, PackVersion};
use oku_slides_session::{DEFAULT_PACK, Library, PackManifest, Request, Storage, builtin_versions};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common::{data_dir, session_in};

fn library(data: &Path) -> Library {
    let storage = Storage::local().unwrap_or_else(|e| panic!("{e}"));
    Library::new(Some(data.join("templates")), storage)
}

fn version(id: Uuid, name: &str, parent: Option<&PackVersion>, created: &str) -> PackFiles {
    let m = PackManifest {
        id,
        name: name.into(),
        description: String::new(),
        parent: parent.cloned(),
        created: created.into(),
        templates: Vec::new(),
        examples: Default::default(),
    };
    m.to_files(BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"))
}

/// Un fichier enregistré par quelqu'un d'autre, dont le deck embarque `pack`.
fn received(them: &Path, id: Uuid, pack: &PackFiles) -> PathBuf {
    library(them).add_version(id, pack).unwrap_or_else(|e| panic!("{e}"));
    let mut s = session_in(them);
    let r = PackRef { id, version: pack.version() };
    s.apply("Pack", vec![Op::SetTemplates { templates: Some(r) }], None).unwrap_or_else(|e| panic!("{e}"));
    let path = them.join("recu.oku-slides.pdf");
    s.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap_or_else(|e| panic!("{e}"));
    path
}

#[test]
fn received_deck_with_the_same_pack_id_never_changes_the_current_version() {
    let (me, them) = (data_dir(), data_dir());
    let id = Uuid::new_v4();
    let mine = version(id, "Charte", None, "2026-10-01T10:00:00Z");
    // La sienne est plus récente et porte le même nom : rien n'en fait la mienne.
    let theirs = version(id, "Charte", None, "2026-10-05T10:00:00Z");
    let path = received(&them, id, &theirs);
    library(&me).add_version(id, &mine).unwrap_or_else(|e| panic!("{e}"));
    let mut s = session_in(&me);
    s.open(&path).unwrap_or_else(|e| panic!("{e}"));
    s.open(&path).unwrap_or_else(|e| panic!("{e}"));
    s.handle(Request::Merge { path: path.clone() }).unwrap_or_else(|e| panic!("{e}"));
    let lib = library(&me);
    assert_that!(lib.current(id)).is_equal_to(Some(mine.version()));
    assert_that!(lib.versions(id).len()).is_equal_to(2);
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(theirs.version()));
}

#[test]
fn received_version_of_a_builtin_pack_never_replaces_the_builtin_current() {
    let (me, them) = (data_dir(), data_dir());
    let builtin = builtin_versions()[0].version.clone();
    let theirs = version(DEFAULT_PACK, "Essentiel retouché", Some(&builtin), "2026-10-05T10:00:00Z");
    let path = received(&them, DEFAULT_PACK, &theirs);
    let mut s = session_in(&me);
    s.open(&path).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(library(&me).current(DEFAULT_PACK)).is_equal_to(Some(builtin));
}
