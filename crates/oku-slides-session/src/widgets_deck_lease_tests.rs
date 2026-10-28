use oku_slides_core::{Category, Namespace, WidgetFile};
use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::Session;
use crate::library::DraftWrite;
use crate::pack::PackManifest;
use crate::pack_lease::PackLease;
use crate::test_support::{adopt_deck_only_pack, fresh_dir};
use crate::test_support_packs::{CARD_V2, opener_on, pack_tab_from, scope_of, snapshot_json};
use crate::{Localize, SessionErrorKind};

/// The namespace `s`'s deck pins `pack` under.
fn ns_of(s: &Session, pack: Uuid) -> Namespace {
    s.user_deck().widgets.iter().find(|(_, p)| p.id == pack).map(|(ns, _)| ns.clone()).unwrap()
}

/// The version `s`'s deck pins `pack` at.
fn pinned(s: &Session, pack: Uuid) -> oku_slides_core::PackVersion {
    s.user_deck().widgets.values().find(|p| p.id == pack).map(|p| p.version.clone()).unwrap()
}

/// A deck-mode write `deck_widget_write_to_a_leased_pack_is_refused` tries.
#[derive(Debug, Clone, Copy)]
enum DeckWrite {
    Set,
    SetByAgent,
    Remove,
    Rename,
}

#[rstest]
#[case::set(DeckWrite::Set)]
#[case::set_by_agent(DeckWrite::SetByAgent)]
#[case::remove(DeckWrite::Remove)]
#[case::rename(DeckWrite::Rename)]
fn deck_widget_write_to_a_leased_pack_is_refused(#[case] write: DeckWrite) {
    let data = fresh_dir("oku-slides-deck-lease-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let _tab = pack_tab_from(&mut s, data.path(), acme);
    let ns = ns_of(&s, acme);
    let (before, deck, steps) = (snapshot_json(&s), s.user_deck().clone(), s.doc.sources.undo_steps());
    let err = match write {
        DeckWrite::Set => s.set_widget(Some(&ns), "card", CARD_V2.into()).map(drop),
        DeckWrite::SetByAgent => s.set_widget_by_agent(Some(&ns), "card", CARD_V2.into()).map(drop),
        DeckWrite::Remove => s.remove_widget(Some(&ns), "card").map(drop),
        DeckWrite::Rename => s.rename_widget(Some(&ns), "card", "carte").map(drop),
    }
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackOpenInTab);
    assert_that!(err.localize(&s.localizer)).is_equal_to(
        "Le pack acme est en cours d'édition dans l'onglet « acme » : modifiez-le là, ou fermez d'abord cet onglet."
            .to_owned(),
    );
    assert_that!((snapshot_json(&s), s.user_deck().clone(), s.doc.sources.undo_steps()))
        .is_equal_to((before, deck, steps));
}

/// A deck pinning `acme` whose write goes past the early check while a lease is taken: the
/// session, its folders, `acme`, the lease, the version written and its parent.
struct LateLease {
    s: Session,
    _data: tempfile::TempDir,
    _deck: tempfile::TempDir,
    acme: Uuid,
    lease: Option<PackLease>,
    written: oku_slides_core::PackVersion,
    parent: oku_slides_core::PackVersion,
}

/// `commit_deck_widgets` writes `card` to `acme` while another editor holds the lease.
fn late_lease() -> LateLease {
    let data = fresh_dir("oku-slides-deck-lease-");
    let (mut s, deck, acme) = opener_on(data.path());
    let parent = pinned(&s, acme);
    let lease = PackLease::acquire(&scope_of(&s), acme, "acme", "Autre").unwrap();
    let t = s.deck_widget_target(Some(&ns_of(&s, acme)), Some(Category::Element)).unwrap();
    let manifest = PackManifest::from_files(&t.files).unwrap();
    let mut widgets = t.files.widgets().clone();
    widgets.insert(WidgetFile::parse("card").unwrap(), CARD_V2.into());
    s.commit_deck_widgets(&t, manifest, widgets).unwrap();
    let written = pinned(&s, acme);
    LateLease { s, _data: data, _deck: deck, acme, lease: Some(lease), written, parent }
}

#[test]
fn deck_widget_late_lease_skips_the_library_write() {
    let l = late_lease();
    assert_that!(l.s.library.version(l.acme, &l.written)).is_none();
}

#[test]
fn deck_widget_late_lease_sets_the_not_saved_notice() {
    let l = late_lease();
    let expected = l.s.localizer.tr_with("session-widgets-not-saved", &[("pack", "acme".into())]);
    assert_that!(l.s.doc.notice.as_deref()).is_equal_to(Some(expected.as_str()));
}

#[test]
fn deck_widget_late_lease_keeps_the_version_in_the_deck() {
    let l = late_lease();
    assert_that!(l.written).is_not_equal_to(l.parent.clone());
    assert_that!(l.s.doc.sources.packs.contains_key(&l.written)).is_true();
}

#[test]
fn deck_widget_late_lease_keeps_the_lineage() {
    let mut l = late_lease();
    l.lease = None;
    let ns = ns_of(&l.s, l.acme);
    l.s.set_widget(Some(&ns), "card", CARD_V2.replace("strong", "underline")).unwrap();
    let next = pinned(&l.s, l.acme);
    let files = l.s.library.version(l.acme, &next).unwrap();
    assert_that!(PackManifest::from_files(&files).unwrap().parent).is_equal_to(Some(l.parent));
}

#[test]
fn deck_widget_late_lease_never_adopts_under_drafts() {
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let l = late_lease();
        done.send(l.s.library.version(l.acme, &l.written).is_none()).unwrap();
    });
    let skipped = finished.recv_timeout(std::time::Duration::from_secs(60)).expect("no deadlock");
    assert_that!(skipped).is_true();
}

#[test]
fn deck_widget_late_lease_version_never_reaches_the_library() {
    let mut l = late_lease();
    let current = l.s.library.current(l.acme);
    l.s.end_widget_session();
    assert_that!(l.s.library.version(l.acme, &l.written)).is_none();
    assert_that!(l.s.library.current(l.acme)).is_equal_to(current);
}

#[test]
fn deck_widget_write_after_the_lease_drops_is_current() {
    let mut l = late_lease();
    l.lease = None;
    let ns = ns_of(&l.s, l.acme);
    l.s.set_widget(Some(&ns), "card", CARD_V2.replace("strong", "underline")).unwrap();
    let next = pinned(&l.s, l.acme);
    assert_that!(l.s.library.current(l.acme)).is_equal_to(Some(next.clone()));
    assert_that!(l.s.library.draft_write(&crate::library::draft_lock(), l.acme, Some(&next)))
        .is_equal_to(DraftWrite::Replace(next));
}

#[test]
fn deck_widget_read_of_a_leased_pack_is_allowed() {
    let data = fresh_dir("oku-slides-deck-lease-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let _tab = pack_tab_from(&mut s, data.path(), acme);
    let ns = ns_of(&s, acme);
    assert_that!(s.widget_status(Some(&ns)).is_ok()).is_true();
    assert_that!(s.widget_source(Some(&ns), "card").is_ok()).is_true();
}

#[test]
fn deck_widget_write_to_a_deck_only_pack_ignores_leases() {
    let data = fresh_dir("oku-slides-deck-lease-");
    let (mut s, _deck, _acme) = opener_on(data.path());
    let d = adopt_deck_only_pack(&mut s);
    let _lease = PackLease::acquire(&scope_of(&s), d.id, "d", "Autre").unwrap();
    s.set_widget(Some(&Namespace::deck()), "card", CARD_V2.into()).unwrap();
    assert_that!(s.widget_source(Some(&Namespace::deck()), "card").unwrap()).is_equal_to(CARD_V2.to_owned());
}
