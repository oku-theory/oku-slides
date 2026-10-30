//! `pack_updates` recalculé par chaque chemin qui change les épinglages ou la bibliothèque (t-1134).

use oku_slides_core::{Namespace, PackKind, PackPin};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::deck_packs::deck_packs_tests::{files, namespace_of_pack, opened_older_than_library, pin_op};
use crate::deck_widgets::deck_widgets_tests::fixture_pack;
use crate::test_support::{fresh_dir, opened_on, session_on};
use crate::{BASE_PACK, Request, Session};

/// v0 ← v1 du pack `id` dans la bibliothèque de `s`, v1 courante : v0 et v1.
fn library_v0_v1(s: &Session, id: Uuid) -> (oku_slides_core::PackFiles, oku_slides_core::PackFiles) {
    let v0 = files(id, "P", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "P", Some(&v0.version()), "2026-10-02T10:00:00Z");
    s.library.add_version(id, &v0).unwrap();
    s.library.add_version(id, &v1).unwrap();
    s.library.set_current(id, &v1.version()).unwrap();
    (v0, v1)
}

#[test]
fn pack_updates_generic_apply_of_an_older_pin_counts_one() {
    let data = fresh_dir("oku-slides-updates-apply-");
    let mut s = opened_on(data.path());
    let id = Uuid::new_v4();
    let (v0, _v1) = library_v0_v1(&s, id);
    s.apply("Pack", vec![pin_op(id, &v0)], None).unwrap();
    assert_that!(s.pack_updates()).is_equal_to(1);
}

#[test]
fn pack_updates_history_restore_before_use_mine_counts_one_again() {
    let (_a, _b, mut s, theirs, _mine) = opened_older_than_library();
    s.pack_use_mine(PackKind::Widgets, &namespace_of_pack(&theirs)).unwrap();
    assert_that!(s.pack_updates()).is_equal_to(0);
    s.history_open().unwrap();
    // La dernière version est celle de « ma version » ; l'avant-dernière, celle d'avant.
    let before = s.history_versions().unwrap().len() - 2;
    s.history_seek(before).unwrap();
    s.history_restore().unwrap();
    assert_that!(s.pack_updates()).is_equal_to(1);
}

#[test]
fn pack_updates_merge_bringing_an_older_pin_counts_one() {
    let (me, them) = (fresh_dir("oku-slides-updates-me-"), fresh_dir("oku-slides-updates-them-"));
    let mut s = opened_on(me.path());
    let id = Uuid::new_v4();
    let (v0, _v1) = library_v0_v1(&s, id);
    let mine = me.path().join("deck.oku-slides.pdf");
    s.handle(Request::Save { path: Some(mine.clone()), overwrite: false }).unwrap();
    // Une autre copie du même deck, qui épingle v0.
    let mut other = session_on(them.path());
    other.library.add_version(id, &v0).unwrap();
    other.open(&mine).unwrap();
    other.apply("Pack", vec![pin_op(id, &v0)], None).unwrap();
    let theirs = them.path().join("copie.oku-slides.pdf");
    other.handle(Request::Save { path: Some(theirs.clone()), overwrite: false }).unwrap();
    assert_that!(s.pack_updates()).is_equal_to(0);
    s.merge_file(&theirs).unwrap();
    assert_that!(s.pack_updates()).is_equal_to(1);
}

/// La version de `base` livrée avant la courante de `s` : la parente de celle-ci.
fn previous_builtin_base(s: &Session) -> oku_slides_core::PackVersion {
    let current = s.library.current(BASE_PACK).unwrap();
    let shipped = crate::builtin_versions().iter().find(|b| b.manifest.id == BASE_PACK && b.version == current);
    shipped.and_then(|b| b.manifest.parent.clone()).unwrap()
}

/// Un fichier d'avant la version livrée de `base` : il épingle la précédente, embarquée, et s'ouvre.
#[test]
fn pack_updates_opening_a_file_on_the_previous_builtin_base_counts_one() {
    let (me, them) = (fresh_dir("oku-slides-updates-me-"), fresh_dir("oku-slides-updates-them-"));
    let mut other = opened_on(them.path());
    let old = fixture_pack("packs/base-626985fb");
    assert_that!(old.version()).is_equal_to(previous_builtin_base(&other));
    let version = other.doc.sources.add_pack(old);
    let mut deck = other.deck().clone();
    let base = Namespace::parse("base").unwrap();
    deck.widgets.insert(base, PackPin { id: BASE_PACK, version, requires: Vec::new() });
    other.doc.sources.apply_silent(deck).unwrap();
    let path = them.path().join("ancien.oku-slides.pdf");
    other.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap();
    let mut s = session_on(me.path());
    s.open(&path).unwrap();
    assert_that!(s.pack_updates()).is_equal_to(1);
}
