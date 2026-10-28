use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::SessionErrorKind;
use crate::library::DraftWrite;
use crate::pack_lease::pack_leased;
use crate::test_support::{adopt_deck_only_pack, fresh_dir, rename_title, session_on};
use crate::test_support_packs::{
    assert_unleased, deck_pack_tab_of, draft_of, editing, opener_on, pack_tab_from, scope_of,
};

/// Whether `s`'s mode holds a lease of `pack`.
fn holds(s: &Session, pack: Uuid) -> bool {
    matches!(&s.mode, Mode::Pack { lease: Some(l), .. } if l.pack() == pack)
}

#[test]
fn save_pack_as_new_moves_the_lease_to_the_copy() {
    let data = fresh_dir("oku-slides-lease-tab-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let scope = scope_of(&s);
    let mut t = pack_tab_from(&mut s, data.path(), acme);
    assert_that!(pack_leased(&scope, acme).is_some()).is_true();
    t.save_pack(true).unwrap();
    let copy = editing(&t).unwrap();
    assert_unleased(&scope, acme);
    assert_that!(holds(&t, copy)).is_true();
    assert_that!(pack_leased(&scope, copy).is_some()).is_true();
}

#[test]
fn save_pack_as_new_of_a_deck_only_pack_takes_no_lease() {
    let data = fresh_dir("oku-slides-lease-tab-");
    let (mut owner, _deck, _acme) = opener_on(data.path());
    let d = adopt_deck_only_pack(&mut owner);
    let mut s = deck_pack_tab_of(&mut owner, data.path(), d.id);
    s.save_pack(true).unwrap();
    let copy = editing(&s).unwrap();
    assert_that!(copy).is_not_equal_to(d.id);
    assert_that!(matches!(&s.mode, Mode::Pack { lease: None, .. })).is_true();
    assert_unleased(&scope_of(&s), copy);
}

#[test]
fn another_deck_adopting_the_pack_tab_draft_makes_its_next_save_a_child() {
    let data = fresh_dir("oku-slides-lease-tab-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let mut t = pack_tab_from(&mut s, data.path(), acme);
    let draft = t.save_pack(false).unwrap();
    let ns = s.user_deck().widgets.iter().find(|(_, p)| p.id == acme).map(|(ns, _)| ns.clone()).unwrap();
    s.pack_use_mine(oku_slides_core::PackKind::Widgets, &ns).unwrap();
    assert_that!(draft_of(&s, acme, &draft)).is_equal_to(DraftWrite::Child(draft.clone()));
    t.apply("x", vec![rename_title("acme 2")], None).unwrap();
    let next = t.save_pack(false).unwrap();
    let files = t.library.version(acme, &next).unwrap();
    assert_that!(crate::pack::PackManifest::from_files(&files).unwrap().parent).is_equal_to(Some(draft));
}

#[test]
fn pack_tab_lease_refuses_another_session_while_it_lasts() {
    let data = fresh_dir("oku-slides-lease-tab-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let _t = pack_tab_from(&mut s, data.path(), acme);
    let err = session_on(data.path()).open_pack_tab_here(acme, None).unwrap_err();
    assert_that!((err.kind(), err.lease_holder())).is_equal_to((SessionErrorKind::PackOpenInTab, Some("acme")));
}

#[test]
fn pack_tab_lease_frees_when_the_session_drops() {
    let data = fresh_dir("oku-slides-lease-tab-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let t = pack_tab_from(&mut s, data.path(), acme);
    drop(t);
    assert_unleased(&scope_of(&s), acme);
}
