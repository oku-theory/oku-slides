use std::path::{Path, PathBuf};

use oku_slides_core::{Op, PackFiles, PackRef, PackVersion};
use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::pack::PackManifest;
use crate::test_support::{fresh_dir, manifest, session_on};
use crate::{Request, Session};

fn files(id: Uuid, name: &str, parent: Option<&PackVersion>, created: &str) -> PackFiles {
    manifest(id, name, parent, created).to_files(std::collections::BTreeMap::new()).unwrap()
}

/// Un fichier enregistré par une autre session (dossier `them`), dont le deck embarque `pack`.
fn received_file(them: &Path, pack: &PackFiles) -> PathBuf {
    let mut other = session_on(them);
    let id = PackManifest::from_files(pack).unwrap().id;
    other.library.add_version(id, pack).unwrap();
    let r = PackRef { id, version: pack.version() };
    other.apply("Pack", vec![Op::SetTemplates { templates: Some(r) }], None).unwrap();
    let path = them.join("recu.oku-slides.pdf");
    other.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap();
    path
}

fn notice(s: &mut Session) -> serde_json::Value {
    let snap: serde_json::Value = serde_json::from_str(&s.handle(Request::Snapshot).unwrap()).unwrap();
    snap["pack_notice"].clone()
}

#[test]
fn open_adds_received_versions_silently_without_changing_current() {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "P", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "P", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let path = received_file(them.path(), &v1);
    let mut s = session_on(me.path());
    s.library.add_version(id, &v0).unwrap();
    s.open(&path).unwrap();
    assert_that!(s.library.versions(id).len()).is_equal_to(2);
    assert_that!(s.library.current(id)).is_equal_to(Some(v0.version()));
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(v1.version()));
}

#[test]
fn open_with_an_unknown_pack_creates_it_with_the_received_version_current() {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v = files(id, "Nouveau", None, "2026-10-01T10:00:00Z");
    let mut s = session_on(me.path());
    s.open(&received_file(them.path(), &v)).unwrap();
    assert_that!(s.library.current(id)).is_equal_to(Some(v.version()));
    assert_that!(notice(&mut s).is_null()).is_true();
}

/// v0 ← v1 ; v0 ← v2. `mine` : ma version courante ; `theirs` : celle du fichier.
#[rstest]
#[case::newer(0, 1, "descendant", "plus récente")]
#[case::older(1, 0, "ancestor", "plus ancienne")]
#[case::parallel(1, 2, "sibling", "en parallèle")]
fn open_with_another_version_shows_the_banner_with_its_relation(
    #[case] mine: usize,
    #[case] theirs: usize,
    #[case] relation: &str,
    #[case] words: &str,
) {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "Charte", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "Charte", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let v2 = files(id, "Charte", Some(&v0.version()), "2026-10-03T10:00:00Z");
    let v = [v0, v1, v2];
    let mut s = session_on(me.path());
    s.library.add_version(id, &v[0]).unwrap();
    s.library.add_version(id, &v[1]).unwrap();
    s.library.set_current(id, &v[mine].version()).unwrap();
    s.open(&received_file(them.path(), &v[theirs])).unwrap();
    let n = notice(&mut s);
    assert_that!(n["relation"].as_str()).is_equal_to(Some(relation));
    assert_that!(n["message"].as_str().unwrap_or_default()).contains(words);
    assert_that!(n["message"].as_str().unwrap_or_default()).contains("Charte");
    assert_that!(s.library.current(id)).is_equal_to(Some(v[mine].version()));
}

#[test]
fn open_with_the_current_version_shows_no_banner() {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v = files(id, "P", None, "2026-10-01T10:00:00Z");
    let mut s = session_on(me.path());
    s.library.add_version(id, &v).unwrap();
    s.open(&received_file(them.path(), &v)).unwrap();
    assert_that!(notice(&mut s).is_null()).is_true();
}

/// Ma version courante v0, le fichier en v1 : la session ouverte, et les deux versions.
fn opened_on_newer() -> (tempfile::TempDir, tempfile::TempDir, Session, PackFiles, PackFiles) {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "P", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "P", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let path = received_file(them.path(), &v1);
    let mut s = session_on(me.path());
    s.library.add_version(id, &v0).unwrap();
    s.open(&path).unwrap();
    (me, them, s, v0, v1)
}

#[test]
fn pack_use_mine_switches_the_deck_and_undo_brings_theirs_back() {
    let (_me, _them, mut s, v0, v1) = opened_on_newer();
    s.handle(Request::PackUseMine).unwrap();
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(v0.version()));
    assert_that!(s.sources_for_tests().packs.contains_key(&v0.version())).is_true();
    assert_that!(notice(&mut s).is_null()).is_true();
    s.handle(Request::Undo).unwrap();
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(v1.version()));
}

#[test]
fn pack_make_current_makes_theirs_current_and_closes_the_banner() {
    let (_me, _them, mut s, _v0, v1) = opened_on_newer();
    let id = s.deck().templates.as_ref().map(|r| r.id).unwrap();
    s.handle(Request::PackMakeCurrent).unwrap();
    assert_that!(s.library.current(id)).is_equal_to(Some(v1.version()));
    assert_that!(notice(&mut s).is_null()).is_true();
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(v1.version()));
}

#[test]
fn pack_notice_close_hides_the_banner_for_the_session() {
    let (_me, them, mut s, _v0, _v1) = opened_on_newer();
    s.handle(Request::PackNoticeClose).unwrap();
    assert_that!(notice(&mut s).is_null()).is_true();
    s.open(&them.path().join("recu.oku-slides.pdf")).unwrap();
    assert_that!(notice(&mut s).is_null()).is_true();
}

#[test]
fn pack_use_mine_is_refused_while_editing_templates() {
    let (_me, _them, mut s, _v0, _v1) = opened_on_newer();
    s.edit_templates(crate::pack::DEFAULT_PACK).unwrap();
    let err = s.handle(Request::PackUseMine).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::EditingTemplates);
}

/// Une version enregistrée par l'autre session à partir du fichier `base`, son deck passé à `pack`.
fn received_edit(them: &Path, base: &Path, pack: &PackFiles) -> PathBuf {
    let mut other = session_on(them);
    other.open(base).unwrap();
    let id = PackManifest::from_files(pack).unwrap().id;
    other.library.add_version(id, pack).unwrap();
    let r = PackRef { id, version: pack.version() };
    other.apply("Pack", vec![Op::SetTemplates { templates: Some(r) }], None).unwrap();
    let path = them.join("suite.oku-slides.pdf");
    other.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap();
    path
}

#[test]
fn merge_adds_received_versions_silently_without_banner() {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
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
    assert_that!(notice(&mut s).is_null()).is_true();
}

#[test]
fn pack_notice_after_the_deck_changes_pack_is_gone_and_make_current_does_nothing() {
    let (_me, _them, mut s, v0, _v1) = opened_on_newer();
    let id = s.deck().templates.as_ref().map(|r| r.id).unwrap();
    let mine = PackRef { id, version: v0.version() };
    s.apply("Pack", vec![Op::SetTemplates { templates: Some(mine) }], None).unwrap();
    assert_that!(notice(&mut s).is_null()).is_true();
    s.handle(Request::PackMakeCurrent).unwrap();
    assert_that!(s.library.current(id)).is_equal_to(Some(v0.version()));
}

#[test]
fn pack_make_current_is_refused_while_editing_templates() {
    let (_me, _them, mut s, v0, _v1) = opened_on_newer();
    let id = s.deck().templates.as_ref().map(|r| r.id).unwrap();
    s.edit_templates(crate::pack::DEFAULT_PACK).unwrap();
    let err = s.handle(Request::PackMakeCurrent).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::EditingTemplates);
    assert_that!(s.library.current(id)).is_equal_to(Some(v0.version()));
}

/// t-328 : une version reçue que la bibliothèque n'a pas pu ajouter n'empêche
/// pas l'ouverture ; le bandeau ne propose pas d'en faire ma version courante.
#[test]
fn open_when_adding_the_received_version_fails_opens_and_offers_no_make_current() {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "Charte", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "Charte", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let path = received_file(them.path(), &v1);
    let mut s = session_on(me.path());
    s.library.add_version(id, &v0).unwrap();
    // Un fichier à la place du dossier de v1 : son écriture échoue.
    let versions = me.path().join("templates").join(id.to_string()).join("versions");
    std::fs::write(versions.join(v1.version().hex()), b"").unwrap();
    s.open(&path).unwrap();
    assert_that!(s.library.version(id, &v1.version())).is_none();
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(v1.version()));
    let n = notice(&mut s);
    assert_that!(n["message"].as_str().unwrap_or_default()).contains("Charte");
    assert_that!(n["use_mine"].is_string()).is_true();
    assert_that!(n["make_current"].is_null()).is_true();
}

#[test]
fn pack_notice_view_offers_make_current_when_the_library_has_their_version() {
    let (_me, _them, mut s, _v0, _v1) = opened_on_newer();
    assert_that!(notice(&mut s)["make_current"].as_str()).is_equal_to(Some("Faire de la sienne ma version courante"));
}
