use std::path::Path;

use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::pack::DEFAULT_PACK;
use crate::test_support::{add_version, fresh_dir, manifest};

fn library(dir: &Path) -> PackLibrary {
    PackLibrary::new(Some(dir.to_owned()), Storage::local().unwrap())
}

/// Packs lus sur le disque par cette instance depuis sa création.
fn scans(lib: &PackLibrary) -> usize {
    lib.index.lock().unwrap().scans
}

#[test]
fn index_read_twice_scans_the_pack_once() {
    let dir = fresh_dir("oku-slides-library-index-");
    let id = Uuid::new_v4();
    let a = add_version(&library(dir.path()), &manifest(id, "A", None, "2026-10-01T10:00:00Z"), &[]);
    let lib = library(dir.path());
    let read_all = || {
        assert_that!(lib.current(id)).is_equal_to(Some(a.version.clone()));
        assert_that!(lib.packs().len()).is_equal_to(1 + builtin_versions().len());
        assert_that!(lib.version(id, &a.version)).is_some();
    };
    read_all();
    read_all();
    assert_that!(scans(&lib)).is_equal_to(1 + builtin_versions().len());
}

#[test]
fn index_add_version_is_seen_without_rescanning() {
    let dir = fresh_dir("oku-slides-library-index-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let a = add_version(&lib, &manifest(id, "A", None, "2026-10-01T10:00:00Z"), &[]);
    let b = add_version(&lib, &manifest(id, "B", Some(&a.version), "2026-10-02T10:00:00Z"), &[]);
    lib.set_current(id, &b.version).unwrap();
    assert_that!(lib.versions(id).len()).is_equal_to(2);
    assert_that!(lib.current(id)).is_equal_to(Some(b.version));
    assert_that!(lib.packs().iter().any(|p| p.id == id)).is_true();
    assert_that!(scans(&lib)).is_equal_to(1 + builtin_versions().len());
}

#[test]
fn index_clone_shares_the_index() {
    let dir = fresh_dir("oku-slides-library-index-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    lib.current(id);
    let a = add_version(&lib.clone(), &manifest(id, "A", None, "2026-10-01T10:00:00Z"), &[]);
    assert_that!(lib.current(id)).is_equal_to(Some(a.version));
    // L'ajout lit les packs intégrés (espaces de noms pris), une fois, dans l'index partagé.
    assert_that!(scans(&lib)).is_equal_to(1 + builtin_versions().len());
}

#[test]
fn index_write_by_another_instance_rebuilds_it() {
    let dir = fresh_dir("oku-slides-library-index-");
    let (lib, other) = (library(dir.path()), library(dir.path()));
    let id = Uuid::new_v4();
    let a = add_version(&lib, &manifest(id, "A", None, "2026-10-01T10:00:00Z"), &[]);
    let b = add_version(&other, &manifest(id, "B", Some(&a.version), "2026-10-02T10:00:00Z"), &[]);
    assert_that!(lib.versions(id).len()).is_equal_to(2);
    other.set_current(id, &b.version).unwrap();
    assert_that!(lib.current(id)).is_equal_to(Some(b.version));
    other.delete(id).unwrap();
    assert_that!(lib.current(id)).is_none();
    assert_that!(lib.packs().iter().any(|p| p.id == id)).is_false();
}

#[test]
fn index_unchanged_generation_is_not_rebuilt() {
    let dir = fresh_dir("oku-slides-library-index-");
    let (lib, other) = (library(dir.path()), library(dir.path()));
    let id = Uuid::new_v4();
    add_version(&other, &manifest(id, "A", None, "2026-10-01T10:00:00Z"), &[]);
    lib.current(id);
    lib.current(id);
    assert_that!(scans(&lib)).is_equal_to(1);
}

/// Jamais cru pour la sécurité : les octets d'une version sont vérifiés à son chargement.
#[test]
fn index_version_altered_after_indexing_is_rejected_on_load() {
    let dir = fresh_dir("oku-slides-library-index-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let a = add_version(&lib, &manifest(id, "A", None, "2026-10-01T10:00:00Z"), &[]);
    let b = add_version(&lib, &manifest(id, "B", Some(&a.version), "2026-10-02T10:00:00Z"), &[]);
    let forged = manifest(id, "Forgé", None, "2026-10-01T10:00:00Z").to_files(Default::default()).unwrap();
    let path = dir.path().join(id.to_string()).join("versions").join(a.version.hex()).join("widgets.json");
    std::fs::write(path, forged.pack_json().unwrap()).unwrap();
    assert_that!(lib.version(id, &a.version)).is_none();
    // La courante altérée : la plus récente, intacte, la remplace.
    assert_that!(lib.current_version(id).map(|s| s.version)).is_equal_to(Some(b.version));
}

/// Après une suite d'écritures, l'index dit ce que dirait une instance neuve.
#[test]
fn index_after_writes_matches_a_fresh_instance() {
    let dir = fresh_dir("oku-slides-library-index-");
    let lib = library(dir.path());
    let builtin = crate::test_support::builtin(crate::pack::DEFAULT_PACK).version.clone();
    let (x, y) = (Uuid::new_v4(), Uuid::new_v4());
    let x1 = add_version(&lib, &manifest(x, "X", None, "2026-10-01T10:00:00Z"), &[("kpi", "K")]);
    let next = manifest(x, "X2", None, "2026-10-02T10:00:00Z").to_files(Default::default()).unwrap();
    lib.replace_version(x, &x1.version, &next).unwrap();
    add_version(&lib, &manifest(y, "Y", None, "2026-10-01T10:00:00Z"), &[]);
    lib.delete(y).unwrap();
    let mine = add_version(&lib, &manifest(DEFAULT_PACK, "Mien", Some(&builtin), "2026-10-03T10:00:00Z"), &[]);
    lib.set_current(DEFAULT_PACK, &mine.version).unwrap();
    let fresh = library(dir.path());
    assert_that!(lib.packs()).is_equal_to(fresh.packs());
    for id in [x, y, DEFAULT_PACK] {
        let listed = |l: &PackLibrary| l.versions(id).into_iter().map(|s| s.version).collect::<Vec<_>>();
        assert_that!(listed(&lib)).is_equal_to(listed(&fresh));
    }
}

#[test]
fn index_after_a_refused_deck_only_version_keeps_the_pack_unchanged() {
    let dir = fresh_dir("oku-slides-library-index-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let a = add_version(&lib, &manifest(id, "A", None, "2026-10-01T10:00:00Z"), &[]);
    let mut deck_only = manifest(id, "B", Some(&a.version), "2026-10-02T10:00:00Z");
    deck_only.scope = oku_slides_core::PackScope::Deck;
    let files = deck_only.to_files(Default::default()).unwrap();
    assert_that!(lib.add_version(id, &files).map_err(|e| e.kind())).is_equal_to(Err(crate::SessionErrorKind::DeckOnly));
    assert_that!(lib.receive_version(id, &files).map_err(|e| e.kind()))
        .is_equal_to(Err(crate::SessionErrorKind::DeckOnly));
    let listed = lib.versions(id).into_iter().map(|s| s.version).collect::<Vec<_>>();
    assert_that!(listed).is_equal_to(vec![a.version.clone()]);
    assert_that!(lib.current(id)).is_equal_to(Some(a.version));
    assert_that!(lib.version(id, &files.version())).is_none();
}
