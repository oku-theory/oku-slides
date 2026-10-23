use std::path::{Path, PathBuf};

use oku_slides_core::{PackFiles, PackKind};
use speculoos::prelude::*;
use uuid::Uuid;

use oku_slides_i18n::Localizer;

use crate::deck_packs::deck_packs_tests::{files, pin_op, pinned_version, received_file};
use crate::pack::PackManifest;
use crate::test_support::{adopt_deck_only_pack, fresh_dir, logged, pin, session_on};
use crate::{Request, Session};

#[test]
fn open_adds_received_versions_silently_without_changing_current() {
    let (me, them) = (fresh_dir("oku-slides-receive-me-"), fresh_dir("oku-slides-receive-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "P", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "P", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let path = received_file(them.path(), &v1);
    let mut s = session_on(me.path());
    s.library.add_version(id, &v0).unwrap();
    s.open(&path).unwrap();
    assert_that!(s.library.versions(id).len()).is_equal_to(2);
    assert_that!(s.library.current(id)).is_equal_to(Some(v0.version()));
    // t-355 : la version reçue est marquée, la mienne non.
    assert_that!((s.library.is_received(id, &v0.version()), s.library.is_received(id, &v1.version())))
        .is_equal_to((false, true));
    assert_that!(pinned_version(&s, id)).is_equal_to(Some(v1.version()));
}

#[test]
fn open_with_an_unknown_pack_creates_it_with_the_received_version_current() {
    let (me, them) = (fresh_dir("oku-slides-receive-me-"), fresh_dir("oku-slides-receive-them-"));
    let id = Uuid::new_v4();
    let v = files(id, "Nouveau", None, "2026-10-01T10:00:00Z");
    let mut s = session_on(me.path());
    s.open(&received_file(them.path(), &v)).unwrap();
    assert_that!(s.library.current(id)).is_equal_to(Some(v.version()));
    assert_that!(s.pack_updates()).is_equal_to(0);
}

/// Une version enregistrée par l'autre session à partir du fichier `base`, son deck passé à `pack`.
fn received_edit(them: &Path, base: &Path, pack: &PackFiles) -> PathBuf {
    let mut other = session_on(them);
    other.open(base).unwrap();
    let id = PackManifest::from_files(pack).unwrap().id;
    other.library.add_version(id, pack).unwrap();
    other.apply("Pack", vec![pin_op(id, pack)], None).unwrap();
    let path = them.join("suite.oku-slides.pdf");
    other.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap();
    path
}

#[test]
fn merge_adds_received_versions_silently() {
    let (me, them) = (fresh_dir("oku-slides-receive-me-"), fresh_dir("oku-slides-receive-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "P", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "P", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let base = received_file(them.path(), &v0);
    let mut s = session_on(me.path());
    s.open(&base).unwrap();
    let path = received_edit(them.path(), &base, &v1);
    s.handle(Request::Merge { path }).unwrap();
    assert_that!(s.library.versions(id).len()).is_equal_to(2);
    assert_that!(s.library.current(id)).is_equal_to(Some(v0.version()));
    // t-355 : les deux versions sont arrivées avec des decks reçus.
    assert_that!((s.library.is_received(id, &v0.version()), s.library.is_received(id, &v1.version())))
        .is_equal_to((true, true));
    // Le deck fusionné épingle v1, plus récente que ma version courante : aucune mise à jour.
    assert_that!(s.pack_updates()).is_equal_to(0);
}

#[test]
fn open_of_a_deck_only_pack_adds_nothing_to_the_library() {
    let (me, them) = (fresh_dir("oku-slides-receive-me-"), fresh_dir("oku-slides-receive-them-"));
    let mut other = session_on(them.path());
    let r = adopt_deck_only_pack(&mut other);
    let path = them.path().join("importe.oku-slides.pdf");
    other.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap();
    let mut s = session_on(me.path());
    let packs = s.library.packs(PackKind::Widgets);
    s.open(&path).unwrap();
    assert_that!(pin(&s, "deck")).is_equal_to(Some(r.clone()));
    assert_that!(s.library.versions(r.id).is_empty()).is_true();
    assert_that!(s.library.packs(PackKind::Widgets)).is_equal_to(&packs);
    assert_that!(s.pack_updates()).is_equal_to(0);
}

fn deck_pack(s: &mut Session) -> serde_json::Value {
    let snap: serde_json::Value = serde_json::from_str(&s.handle(Request::Snapshot).unwrap()).unwrap();
    snap["deck_pack"].clone()
}

#[test]
fn snapshot_deck_pack_is_false_for_a_library_pack() {
    let dir = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(dir.path());
    assert_that!(deck_pack(&mut s)).is_equal_to(serde_json::Value::Bool(false));
}

#[test]
fn snapshot_deck_pack_is_true_once_the_deck_takes_a_deck_only_pack() {
    let dir = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(dir.path());
    adopt_deck_only_pack(&mut s);
    assert_that!(deck_pack(&mut s)).is_equal_to(serde_json::Value::Bool(true));
}

#[test]
fn is_deck_only_with_an_unreadable_scope_is_false_and_logged() {
    let mut manifest = serde_json::Map::new();
    manifest.insert("scope".into(), serde_json::json!("ailleurs"));
    let files = PackFiles::new(PackKind::Widgets, manifest, std::collections::BTreeMap::new());
    let mut deck_only = true;
    let log = logged(|| deck_only = super::is_deck_only(&files));
    assert_that!(deck_only).is_false();
    assert_that!(log.as_str()).contains("WARN");
}

/// Un thème de la bibliothèque d'avant les espaces de noms est écarté, et dit une fois par processus.
#[test]
fn notice_skipped_themes_counts_the_unreadable_library_themes_once() {
    let data = fresh_dir("oku-slides-themes-skipped-");
    crate::test_support::write_raw_theme(&data.path().join("themes"), crate::test_support::OLD_THEME_JSON);
    let mut s = session_on(data.path());
    let expected = s.localizer.tr_with("notice-themes-skipped", &[("count", 1.into())]);
    assert_that!(s.doc.notice.clone()).is_equal_to(Some(expected.clone()));
    s.doc.notice = None;
    s.notice_skipped_themes();
    assert_that!(s.doc.notice.clone()).is_none();
    let de = Localizer::new(Some("de")).tr_with("notice-themes-skipped", &[("count", 1.into())]);
    assert_that!(de)
        .is_not_equal_to(Localizer::new(Some("en")).tr_with("notice-themes-skipped", &[("count", 1.into())]));
}

/// L'avis des thèmes écartés suit le document qui remplace le deck vierge (reprise d'un onglet) ; vu, il ne revient pas.
#[test]
fn notice_skipped_themes_follows_the_document_loaded_over_the_blank_deck_once() {
    let data = fresh_dir("oku-slides-themes-skipped-load-");
    crate::test_support::write_raw_theme(&data.path().join("themes"), crate::test_support::OLD_THEME_JSON);
    let mut s = session_on(data.path());
    let expected = s.localizer.tr_with("notice-themes-skipped", &[("count", 1.into())]);
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/roadmap.json");
    s.open(&fixture).unwrap();
    assert_that!(s.doc.notice.clone()).is_equal_to(Some(expected));
    // Vu par l'interface : le snapshot le vide ; un autre document ne le reprend pas.
    s.handle(Request::Snapshot).unwrap();
    s.open(&fixture).unwrap();
    assert_that!(s.doc.notice.clone()).is_none();
    let other = session_on(data.path());
    assert_that!(other.doc.notice.clone()).is_none();
}
