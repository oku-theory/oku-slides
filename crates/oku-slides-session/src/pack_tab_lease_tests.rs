use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::library::{DraftWrite, draft_lock};
use crate::pack_lease::pack_leased;
use crate::pack_mode_lease::Settled;
use crate::test_support::{adopt_deck_only_pack, fresh_dir, manifest_ns, rename_title, session_on};
use crate::test_support_packs::{assert_unleased, draft_of, editing, opener_on, pack_tab_from, scope_of};
use crate::{BASE_PACK, Request, SessionErrorKind};

/// Whether `s`'s mode holds a lease of `pack`.
fn holds(s: &Session, pack: Uuid) -> bool {
    matches!(&s.mode, Mode::Pack { lease: Some(l), .. } if l.pack() == pack)
}

#[test]
fn deck_workshop_of_a_library_pack_takes_its_lease() {
    let data = fresh_dir("oku-slides-lease-tab-");
    let (mut s, _deck, acme) = opener_on(data.path());
    s.edit_pack(acme).unwrap();
    assert_that!(holds(&s, acme)).is_true();
    assert_that!(pack_leased(&scope_of(&s), acme))
        .is_equal_to(Some(label(&s.document_name(crate::test_support::stash_of(&s)))));
}

#[test]
fn deck_workshop_close_pack_frees_the_lease() {
    let data = fresh_dir("oku-slides-lease-tab-");
    let (mut s, _deck, acme) = opener_on(data.path());
    s.edit_pack(acme).unwrap();
    s.close_pack().unwrap();
    assert_unleased(&scope_of(&s), acme);
}

#[test]
fn deck_workshop_switch_to_another_library_pack_moves_the_lease() {
    let data = fresh_dir("oku-slides-lease-tab-");
    let (mut s, _deck, acme) = opener_on(data.path());
    s.edit_pack(acme).unwrap();
    s.edit_pack(BASE_PACK).unwrap();
    assert_that!(holds(&s, BASE_PACK)).is_true();
    assert_unleased(&scope_of(&s), acme);
}

#[test]
fn deck_workshop_switch_to_a_deck_only_pack_frees_the_lease() {
    let data = fresh_dir("oku-slides-lease-tab-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let d = adopt_deck_only_pack(&mut s);
    s.edit_pack(acme).unwrap();
    s.edit_pack(d.id).unwrap();
    assert_that!(matches!(&s.mode, Mode::Pack { lease: None, .. })).is_true();
    assert_unleased(&scope_of(&s), acme);
}

#[test]
fn deck_workshop_refused_switch_keeps_the_old_lease() {
    let data = fresh_dir("oku-slides-lease-tab-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let mut other = session_on(data.path());
    other.edit_pack(BASE_PACK).unwrap();
    s.edit_pack(acme).unwrap();
    assert_that!(s.edit_pack(BASE_PACK).unwrap_err().kind()).is_equal_to(SessionErrorKind::PackOpenInTab);
    assert_that!(holds(&s, acme)).is_true();
    assert_that!(editing(&s)).is_equal_to(Some((acme, PackHost::Deck)));
}

#[test]
fn deck_workshop_of_a_library_without_dirs_takes_no_lease() {
    let mut s = crate::test_support::session();
    s.library = crate::PackLibrary::new(None, None, s.themes.clone(), s.storage.clone());
    s.edit_pack(BASE_PACK).unwrap();
    assert_that!(editing(&s)).is_equal_to(Some((BASE_PACK, PackHost::Deck)));
    assert_that!(matches!(&s.mode, Mode::Pack { lease: None, .. })).is_true();
}

#[test]
fn deck_workshop_enter_failure_after_the_lease_frees_it() {
    let data = fresh_dir("oku-slides-lease-tab-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let scope = scope_of(&s);
    // Q requires a namespace no pack provides: its closure fails after the lease is taken.
    let q = Uuid::new_v4();
    let manifest = manifest_ns(q, "qux", &["nulle-part"]);
    crate::test_support::add_version(&s.library, &manifest, &[]);
    let stored = s.pack_base(q).unwrap();
    let settled = s.settle_lease(q, PackHost::Deck, &stored, None).unwrap();
    assert_that!(matches!(settled, Settled::Hold(_))).is_true();
    let probe = PackLease::try_take(&draft_lock(), &scope, q, "q", "probe");
    assert_that!(probe.map(|_| ()).unwrap_err().kind()).is_equal_to(SessionErrorKind::PackOpenInTab);
    drop(settled);
    assert_unleased(&scope, q);
    s.handle(Request::EditPack { pack: acme }).unwrap();
    let err = s.handle(Request::EditPack { pack: q }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownNamespace);
    assert_unleased(&scope, q);
    assert_that!(holds(&s, acme)).is_true();
}

#[rstest]
#[case::deck(PackHost::Deck)]
#[case::tab(PackHost::Tab)]
fn save_pack_as_new_moves_the_lease_to_the_copy(#[case] host: PackHost) {
    let data = fresh_dir("oku-slides-lease-tab-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let scope = scope_of(&s);
    let mut t = match host {
        PackHost::Deck => {
            s.edit_pack(acme).unwrap();
            s
        }
        PackHost::Tab => pack_tab_from(&mut s, data.path(), acme),
    };
    assert_that!(pack_leased(&scope, acme).is_some()).is_true();
    t.save_pack(true).unwrap();
    let (copy, _) = editing(&t).unwrap();
    assert_unleased(&scope, acme);
    assert_that!(holds(&t, copy)).is_true();
    assert_that!(pack_leased(&scope, copy).is_some()).is_true();
}

#[test]
fn save_pack_as_new_of_a_deck_only_pack_takes_no_lease() {
    let data = fresh_dir("oku-slides-lease-tab-");
    let (mut s, _deck, _acme) = opener_on(data.path());
    let d = adopt_deck_only_pack(&mut s);
    s.edit_pack(d.id).unwrap();
    s.save_pack(true).unwrap();
    let (copy, _) = editing(&s).unwrap();
    assert_that!(copy).is_not_equal_to(d.id);
    assert_that!(matches!(&s.mode, Mode::Pack { lease: None, .. })).is_true();
    assert_unleased(&scope_of(&s), copy);
}

#[test]
fn pack_to_pack_switch_keeps_the_host() {
    let data = fresh_dir("oku-slides-lease-tab-");
    let (mut s, _deck, acme) = opener_on(data.path());
    s.edit_pack(acme).unwrap();
    s.edit_pack(BASE_PACK).unwrap();
    assert_that!(editing(&s)).is_equal_to(Some((BASE_PACK, PackHost::Deck)));
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
    let err = s.edit_pack(acme).unwrap_err();
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
