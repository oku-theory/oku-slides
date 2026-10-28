use oku_slides_core::{Namespace, PackKind, PackVersion};
use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::library::{DraftWrite, draft_lock};
use crate::test_support::{add_pack, add_version, adopt_deck_only_pack, fresh_dir, manifest_ns, rename_title};
use crate::test_support_packs::{CARD_SRC, draft_of, opener_on, pack_tab_from};
use crate::{Localize, Request, SessionErrorKind};

/// An opener deck pinning `acme` at `v1`, a pack tab that wrote `v2` (current, newer).
struct Offer {
    s: Session,
    tab: Session,
    data: tempfile::TempDir,
    _deck: tempfile::TempDir,
    acme: Uuid,
    v1: PackVersion,
    v2: PackVersion,
}

/// The namespace `s`'s deck pins `pack` under.
fn ns_of(s: &Session, pack: Uuid) -> Namespace {
    s.user_deck().widgets.iter().find(|(_, p)| p.id == pack).map(|(ns, _)| ns.clone()).unwrap()
}

/// The version `s`'s deck pins `pack` at.
fn pinned(s: &Session, pack: Uuid) -> Option<PackVersion> {
    s.user_deck().all_pins().find(|(_, _, p)| p.id == pack).map(|(_, _, p)| p.version.clone())
}

/// The offer fixture: the tab renamed its pack and saved.
fn offer() -> Offer {
    let data = fresh_dir("oku-slides-offer-");
    let (mut s, deck, acme) = opener_on(data.path());
    let v1 = pinned(&s, acme).unwrap();
    let mut tab = pack_tab_from(&mut s, data.path(), acme);
    tab.apply("x", vec![rename_title("acme 2")], None).unwrap();
    let v2 = tab.save_pack(false).unwrap();
    assert_that!(s.library.current(acme)).is_equal_to(Some(v2.clone()));
    Offer { s, tab, data, _deck: deck, acme, v1, v2 }
}

/// The deck's identity in `s`.
fn uid(s: &Session) -> String {
    s.doc.sources.replica().deck_uid()
}

/// A library version of `pack` written beside the others: `parent`, `requires`, made current.
fn current_version(s: &Session, pack: Uuid, parent: Option<&PackVersion>, requires: &[&str]) -> PackVersion {
    let current = s.library.current_version(pack).unwrap();
    let mut m = current.manifest.clone();
    m.parent = parent.cloned();
    m.name = format!("acme {}", requires.len() + usize::from(parent.is_some()) * 10);
    m.requires = requires.iter().map(|r| crate::test_support::ns(r)).collect();
    let files = m.to_files(current.files.kind(), current.files.widgets().clone()).unwrap();
    s.library.add_version(pack, &files).unwrap();
    s.library.set_current(pack, &files.version()).unwrap();
    files.version()
}

#[test]
fn use_pack_version_success_ends_the_widget_session() {
    let mut o = offer();
    adopt_deck_only_pack(&mut o.s);
    o.s.set_widget(Some(&Namespace::deck()), "card", CARD_SRC.into()).unwrap();
    assert_that!(o.s.widget_session.is_some()).is_true();
    let uid = uid(&o.s);
    o.s.use_pack_version(o.acme, &o.v2, &uid).unwrap();
    assert_that!(o.s.widget_session.is_none()).is_true();
}

#[test]
fn opener_offer_names_a_deck_pinning_an_older_version() {
    let o = offer();
    let expected = DeckOffer { title: "opener".into(), deck_uid: uid(&o.s) };
    assert_that!(o.s.opener_offer(o.acme, &o.v2)).is_equal_to(Some(expected));
}

/// Why `opener_offer_is_none_unless_due` makes no offer.
#[derive(Debug, Clone, Copy)]
enum NotDue {
    Current,
    NotPinned,
    Newer,
    Parallel,
    DeckOnly,
    NoLongerCurrent,
    RequiresUnpinned,
}

#[rstest]
#[case::current(NotDue::Current)]
#[case::not_pinned(NotDue::NotPinned)]
#[case::newer(NotDue::Newer)]
#[case::parallel(NotDue::Parallel)]
#[case::deck_only(NotDue::DeckOnly)]
#[case::no_longer_current(NotDue::NoLongerCurrent)]
#[case::requires_unpinned(NotDue::RequiresUnpinned)]
fn opener_offer_is_none_unless_due(#[case] why: NotDue) {
    if let NotDue::Newer = why {
        let (_me, _them, s, _theirs, mine) = crate::deck_packs::deck_packs_tests::opened_on_newer();
        let id = crate::pack::PackManifest::from_files(&mine).unwrap().id;
        assert_that!(s.library.current(id)).is_equal_to(Some(mine.version()));
        assert_that!(s.opener_offer(id, &mine.version())).is_none();
        return;
    }
    let mut o = offer();
    let (pack, version) = match why {
        NotDue::Current => {
            o.s.pack_use_mine(PackKind::Widgets, &ns_of(&o.s, o.acme)).unwrap();
            (o.acme, o.v2.clone())
        }
        NotDue::NotPinned => {
            o.s.remove_deck_pack(PackKind::Widgets, &ns_of(&o.s, o.acme)).unwrap();
            (o.acme, o.v2.clone())
        }
        NotDue::Parallel => (o.acme, current_version(&o.s, o.acme, None, &[])),
        NotDue::DeckOnly => {
            let d = adopt_deck_only_pack(&mut o.s);
            let library_scoped = crate::pack::PackManifest { id: d.id, ..manifest_ns(d.id, "dlib", &[]) };
            let pin = add_version(&o.s.library, &library_scoped, &[]);
            (d.id, pin.version)
        }
        NotDue::NoLongerCurrent => {
            o.s.library.set_current(o.acme, &o.v1).unwrap();
            (o.acme, o.v2.clone())
        }
        NotDue::RequiresUnpinned => (o.acme, current_version(&o.s, o.acme, Some(&o.v2), &["socle"])),
        NotDue::Newer => unreachable!("handled above"),
    };
    assert_that!(o.s.opener_offer(pack, &version)).is_none();
}

#[test]
fn opener_offer_takes_the_first_pin_of_a_pack_pinned_twice() {
    let o = offer();
    let mut s = o.s;
    let mut deck = s.doc.sources.deck().clone();
    let first = ns_of(&s, o.acme);
    let mut current = deck.widgets[&first].clone();
    current.version = o.v2.clone();
    // A second pin, at the current version, under a namespace sorting after the first.
    deck.widgets.insert(Namespace::parse(&format!("{first}-z")).unwrap(), current);
    s.doc.sources.apply_silent(deck).unwrap();
    assert_that!(s.opener_offer(o.acme, &o.v2).is_some()).is_true();
}

#[test]
fn use_pack_version_pins_the_offered_version() {
    let mut o = offer();
    let uid = uid(&o.s);
    o.s.use_pack_version(o.acme, &o.v2, &uid).unwrap();
    assert_that!(pinned(&o.s, o.acme)).is_equal_to(Some(o.v2));
}

#[test]
fn use_pack_version_takes_one_undo_step() {
    let mut o = offer();
    let (uid, steps) = (uid(&o.s), o.s.doc.sources.undo_steps());
    o.s.use_pack_version(o.acme, &o.v2, &uid).unwrap();
    assert_that!(o.s.doc.sources.undo_steps()).is_equal_to(steps + 1);
}

#[test]
fn use_pack_version_takes_requires_from_that_version() {
    let mut o = offer();
    let old = o.s.user_deck().widgets[&ns_of(&o.s, o.acme)].requires.clone();
    let requires: &[&str] = if old.is_empty() { &["base"] } else { &[] };
    let v3 = current_version(&o.s, o.acme, Some(&o.v2), requires);
    let uid = uid(&o.s);
    o.s.use_pack_version(o.acme, &v3, &uid).unwrap();
    let now = o.s.user_deck().widgets[&ns_of(&o.s, o.acme)].requires.clone();
    assert_that!(now).is_not_equal_to(old);
    assert_that!(now).is_equal_to(requires.iter().map(|r| crate::test_support::ns(r)).collect::<Vec<_>>());
}

#[test]
fn use_pack_version_freezes_the_pinned_version() {
    let mut o = offer();
    assert_that!(draft_of(&o.s, o.acme, &o.v2)).is_equal_to(DraftWrite::Replace(o.v2.clone()));
    let uid = uid(&o.s);
    o.s.use_pack_version(o.acme, &o.v2, &uid).unwrap();
    assert_that!(o.s.library.draft_write(&draft_lock(), o.acme, Some(&o.v2))).is_equal_to(DraftWrite::Child(o.v2));
}

/// What changed before `use_pack_version_refuses_an_expired_offer` takes the offer.
#[derive(Debug, Clone, Copy)]
enum Expired {
    UpdatedMeanwhile,
    NewerCurrent,
    NoLongerPinned,
    AlreadyPinned,
    AnotherDeck,
}

/// Expires the offer of `o` by `how`; the deck identity to send.
fn expire(o: &mut Offer, how: Expired) -> String {
    let ns = ns_of(&o.s, o.acme);
    match how {
        Expired::UpdatedMeanwhile => {
            // The deck moved past the offer: it pins a child of `v2`, which is current again.
            current_version(&o.s, o.acme, Some(&o.v2), &[]);
            o.s.pack_use_mine(PackKind::Widgets, &ns).unwrap();
            o.s.library.set_current(o.acme, &o.v2).unwrap();
        }
        Expired::NewerCurrent => {
            o.tab.apply("x", vec![rename_title("acme 3")], None).unwrap();
            o.tab.save_pack(false).unwrap();
        }
        Expired::NoLongerPinned => o.s.remove_deck_pack(PackKind::Widgets, &ns).unwrap(),
        Expired::AlreadyPinned => o.s.pack_use_mine(PackKind::Widgets, &ns).unwrap(),
        Expired::AnotherDeck => return "un-autre-deck".into(),
    }
    uid(&o.s)
}

#[rstest]
#[case::updated_meanwhile(Expired::UpdatedMeanwhile)]
#[case::newer_current(Expired::NewerCurrent)]
#[case::no_longer_pinned(Expired::NoLongerPinned)]
#[case::already_pinned(Expired::AlreadyPinned)]
#[case::another_deck(Expired::AnotherDeck)]
fn use_pack_version_refuses_an_expired_offer(#[case] how: Expired) {
    let mut o = offer();
    let uid = expire(&mut o, how);
    let pins = pinned(&o.s, o.acme);
    let err = o.s.use_pack_version(o.acme, &o.v2, &uid).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackOfferExpired);
    assert_that!(err.localize(&o.s.localizer)).is_equal_to(
        "Le deck n'a pas été mis à jour : le deck ou cette version du pack a changé entre-temps.".to_owned(),
    );
    assert_that!(pinned(&o.s, o.acme)).is_equal_to(pins);
}

#[test]
fn use_pack_version_refusal_keeps_the_undo_stack() {
    let mut o = offer();
    let steps = o.s.doc.sources.undo_steps();
    assert_that!(o.s.use_pack_version(o.acme, &o.v2, "un-autre-deck").is_err()).is_true();
    assert_that!(o.s.doc.sources.undo_steps()).is_equal_to(steps);
}

#[test]
fn use_pack_version_refusal_keeps_the_widget_session() {
    let mut o = offer();
    let beta = Uuid::new_v4();
    let manifest = manifest_ns(beta, "beta", &[]);
    let pin = add_version(&o.s.library, &manifest, &[]);
    o.s.apply("beta", vec![add_pack(&manifest, &pin)], None).unwrap();
    o.s.end_widget_session();
    o.s.set_widget(Some(&crate::test_support::ns("beta")), "card", CARD_SRC.into()).unwrap();
    let draft = pinned(&o.s, beta).unwrap();
    let request = Request::UsePackVersion { pack: o.acme, version: o.v2.clone(), deck_uid: "un-autre-deck".into() };
    assert_that!(o.s.handle(request).unwrap_err().kind()).is_equal_to(SessionErrorKind::PackOfferExpired);
    assert_that!(o.s.widget_session.is_some()).is_true();
    assert_that!(draft_of(&o.s, beta, &draft)).is_equal_to(DraftWrite::Replace(draft));
}

#[test]
fn use_pack_version_with_an_unreadable_manifest_keeps_its_error() {
    let mut o = offer();
    assert_that!(o.s.opener_offer(o.acme, &o.v2).is_some()).is_true();
    // The library's index still lists the version; its files no longer match it.
    let folder = o.data.path().join("widgets").join(o.acme.to_string()).join("versions").join(o.v2.hex());
    std::fs::write(folder.join("widgets.json"), b"{}").unwrap();
    let uid = uid(&o.s);
    let err = o.s.use_pack_version(o.acme, &o.v2, &uid).unwrap_err();
    assert_that!(err.kind()).is_not_equal_to(SessionErrorKind::PackOfferExpired);
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackVersionMissing);
}
