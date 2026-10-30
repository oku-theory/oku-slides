use std::sync::Arc;

use oku_slides_core::{PackPin, PackVersion};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::PackOpened;
use super::workspace_packs_fixtures::{Packs, forge_pack_entry, packs, packs_resumed, within};
use crate::test_support::{add_pack, apply_ws, library_of, rename_title};
use crate::test_support_packs::CARD_V2;
use crate::workspace::relock;
use crate::{BASE_PACK, Request, SessionErrorKind, Workspace};

/// `tab`'s badge at its next snapshot.
fn badge(f: &Packs, tab: &str) -> u32 {
    Workspace::request(&f.ws, tab, Request::Snapshot).0.unwrap_or_else(|e| panic!("{}", e.one_line()));
    f.with(tab, |s| s.snapshot().pack_updates)
}

/// `t-1`'s deck-mode write of `card` on `acme`, by kind.
fn deck_write(f: &Packs) -> Result<(), SessionErrorKind> {
    let ns = f.acme_ns.clone();
    f.with("t-1", |s| s.set_widget(Some(&ns), "card", CARD_V2.into()).map(|_| ()).map_err(|e| e.kind()))
}

#[test]
fn closing_an_active_pack_tab_activates_its_opener() {
    let f = packs();
    let p = f.pack_tab();
    f.new_tab();
    f.activate(&p);
    f.close(&p);
    assert_that!(f.active(0)).is_equal_to("t-1".to_owned());
}

#[test]
fn closing_an_inactive_pack_tab_keeps_the_active_tab() {
    let f = packs();
    let p = f.pack_tab();
    let x = f.new_tab();
    f.close(&p);
    assert_that!(f.active(0)).is_equal_to(x);
}

#[test]
fn closing_a_pack_tab_whose_opener_moved_window_uses_the_neighbour_rule() {
    let f = packs();
    let p = f.pack_tab();
    let x = f.new_tab();
    f.send(json!({"cmd": "move_tab", "tab": "t-1"}));
    f.activate(&p);
    f.close(&p);
    assert_that!(f.active(0)).is_equal_to(x);
}

#[test]
fn closing_a_pack_tab_whose_opener_closed_uses_the_neighbour_rule() {
    let f = packs();
    let p = f.pack_tab();
    let x = f.new_tab();
    f.close("t-1");
    f.activate(&p);
    f.close(&p);
    assert_that!(f.active(0)).is_equal_to(x);
}

#[test]
fn closing_a_pack_tab_frees_the_lease() {
    let f = packs();
    let p = f.pack_tab();
    let before = deck_write(&f);
    f.close(&p);
    assert_that!((before, deck_write(&f))).is_equal_to((Err(SessionErrorKind::PackOpenInTab), Ok(())));
}

#[test]
fn stale_session_handle_of_a_closed_tab_cannot_write() {
    let f = packs();
    let p = f.pack_tab();
    let slot = Workspace::lock(&f.ws).session(&p).unwrap();
    f.close(&p);
    let ready = relock(&slot).ready_with_links(0, &[]).map(|_| ()).map_err(|e| e.kind());
    assert_that!(ready).is_equal_to(Err(SessionErrorKind::UnknownTab));
}

/// The non-discard close of a pack tab whose session is busy or poisoned answers at once.
#[rstest]
#[case::busy(false)]
#[case::poisoned(true)]
fn closing_a_busy_pack_tab_never_waits(#[case] poisoned: bool) {
    let f = packs();
    let p = f.pack_tab();
    let slot = Workspace::lock(&f.ws).session(&p).unwrap();
    if poisoned {
        let poisoner = Arc::clone(&slot);
        let _ = std::thread::spawn(move || {
            let _guard = poisoner.lock();
            panic!("deliberate panic");
        })
        .join();
        assert_that!(slot.is_poisoned()).is_true();
    }
    let guard = (!poisoned).then(|| slot.lock().unwrap());
    let (ws, tab) = (Arc::clone(&f.ws), p.clone());
    let r = within(move || apply_ws(&ws, json!({"cmd": "close", "tab": tab})).map(|_| ()).map_err(|e| e.kind()));
    drop(guard);
    assert_that!(r).is_equal_to(Err(SessionErrorKind::Unsaved));
}

#[test]
fn closing_a_pack_tab_never_waits_on_a_busy_deck() {
    let f = packs();
    let p = f.pack_tab();
    let deck = Workspace::lock(&f.ws).session("t-1").unwrap();
    let guard = deck.lock().unwrap();
    let (ws, tab) = (Arc::clone(&f.ws), p.clone());
    let r = within(move || apply_ws(&ws, json!({"cmd": "close", "tab": tab, "discard": true})).is_ok());
    drop(guard);
    assert_that!(r).is_true();
}

#[test]
fn opener_badge_lights_on_its_next_snapshot_after_the_pack_tab_saves() {
    let f = packs();
    let p = f.pack_tab();
    let before = badge(&f, "t-1");
    f.write_in(&p);
    assert_that!((before, badge(&f, "t-1"))).is_equal_to((0, 1));
}

#[test]
fn another_deck_badge_lights_on_its_next_snapshot() {
    let f = packs();
    let other = f.new_tab();
    let acme = f.acme;
    f.with(&other, |s| {
        let current = s.library.current_version(acme).unwrap();
        let pin = PackPin { id: acme, version: current.files.version(), requires: current.manifest.requires.clone() };
        s.apply("acme", vec![add_pack(&current.manifest, &pin)], None).unwrap();
    });
    let p = f.pack_tab();
    let before = badge(&f, &other);
    f.write_in(&p);
    assert_that!((before, badge(&f, &other))).is_equal_to((0, 1));
}

#[test]
fn dormant_deck_badge_lights_on_wake() {
    let f = packs_resumed();
    let p = f.pack_tab();
    let before = badge(&f, "t-1");
    let f = f.restart(|_, _| {});
    f.write_in(&p);
    assert_that!(f.is_dormant("t-1")).is_true();
    let woken = f.with("t-1", |s| s.snapshot().pack_updates);
    assert_that!((before, woken)).is_equal_to((0, 1));
}

/// O10: six dormant entries cached as pack tabs of P, whose states are decks.
#[test]
fn open_pack_wakes_at_most_four_dormant_candidates() {
    let f = packs_resumed();
    let decoys: Vec<String> = (0..6).map(|_| f.new_tab()).collect();
    let acme = f.acme;
    let f = f.restart(|_, layout| {
        for d in &decoys {
            forge_pack_entry(layout, d, acme);
        }
    });
    f.open("t-1", acme, None).unwrap();
    let woke = decoys.iter().filter(|d| !f.is_dormant(d)).count();
    assert_that!(woke).is_equal_to(4);
}

#[test]
fn saving_in_a_pack_tab_leaves_the_opener_deck_pins_dirty_and_undo_unchanged() {
    let f = packs();
    let p = f.pack_tab();
    let state = || {
        f.with("t-1", |s| {
            let (deck, snap) = (s.user_deck().clone(), s.snapshot());
            (deck.widgets, deck.layouts, s.doc.dirty, snap.undo_label.clone(), snap.redo_label.clone())
        })
    };
    let before = state();
    f.write_in(&p);
    assert_that!(state()).is_equal_to(before);
}

#[test]
fn pack_opener_offers_the_written_version() {
    let f = packs();
    let p = f.pack_tab();
    let v = f.write_in(&p);
    let (name, uid) = (f.with(&p, |s| s.tab_info().title), f.with("t-1", |s| s.doc.sources.replica().deck_uid()));
    let offer = Workspace::pack_opener(&f.ws, &p).unwrap().unwrap();
    assert_that!((offer.tab, offer.pack, offer.pack_name, offer.version, offer.deck.deck_uid)).is_equal_to((
        "t-1".to_owned(),
        f.acme,
        name,
        v,
        uid,
    ));
}

/// Why `pack_opener_is_null` answers nothing.
#[derive(Debug, Clone, Copy)]
enum Null {
    SavedAsNew,
    NoOpener,
    OpenerClosed,
    OpenerCurrent,
    NothingWritten,
    NewerElsewhere,
    OpenerIsPackTab,
}

/// A newer version of `acme` than `v`, made current by another process (the library seam).
fn newer_elsewhere(f: &Packs, v: &PackVersion) {
    let acme = f.acme;
    f.with("t-1", |s| {
        let current = s.library.current_version(acme).unwrap();
        let mut m = current.manifest.clone();
        m.parent = Some(v.clone());
        m.name = "acme elsewhere".into();
        let files = m.to_files(current.files.kind(), current.files.widgets().clone()).unwrap();
        library_of(s).add_version(acme, &files).unwrap();
        library_of(s).set_current(acme, &files.version()).unwrap();
    });
}

#[rstest]
#[case::saved_as_new(Null::SavedAsNew)]
#[case::no_opener(Null::NoOpener)]
#[case::opener_closed(Null::OpenerClosed)]
#[case::opener_current(Null::OpenerCurrent)]
#[case::nothing_written(Null::NothingWritten)]
#[case::newer_elsewhere(Null::NewerElsewhere)]
#[case::opener_is_pack_tab(Null::OpenerIsPackTab)]
fn pack_opener_is_null(#[case] why: Null) {
    let f = packs();
    let mut p = f.pack_tab();
    match why {
        Null::SavedAsNew => f.with(&p, |s| {
            s.apply("x", vec![rename_title("acme copie")], None).unwrap();
            drop(s.save_pack(true).unwrap());
        }),
        Null::NoOpener => {
            f.write_in(&p);
            Workspace::lock(&f.ws).tabs.get_mut(&p).unwrap().opener = None;
        }
        Null::OpenerClosed => {
            f.write_in(&p);
            f.close("t-1");
        }
        Null::OpenerCurrent => {
            let v = f.write_in(&p);
            let deck_uid = f.with("t-1", |s| s.doc.sources.replica().deck_uid());
            let take = Request::UsePackVersion { pack: f.acme, version: v, deck_uid };
            Workspace::request(&f.ws, "t-1", take).0.unwrap();
        }
        Null::NothingWritten => {}
        Null::NewerElsewhere => {
            let v = f.write_in(&p);
            newer_elsewhere(&f, &v);
        }
        Null::OpenerIsPackTab => {
            let Ok(PackOpened::Opened { tab: q, .. }) = f.open(&p, BASE_PACK, None) else {
                panic!("a pack tab of Base")
            };
            f.write_in(&q);
            p = q;
        }
    }
    assert_that!(Workspace::pack_opener(&f.ws, &p).unwrap().is_none()).is_true();
}

#[test]
fn pack_opener_of_a_deck_tab_is_null() {
    let f = packs();
    f.pack_tab();
    assert_that!(Workspace::pack_opener(&f.ws, "t-1").unwrap().is_none()).is_true();
}

#[test]
fn use_pack_version_on_the_opener_after_close_moves_its_pin() {
    let f = packs();
    let p = f.pack_tab();
    let v = f.write_in(&p);
    let offer = Workspace::pack_opener(&f.ws, &p).unwrap().unwrap();
    f.close(&p);
    let take = Request::UsePackVersion { pack: offer.pack, version: offer.version, deck_uid: offer.deck.deck_uid };
    Workspace::request(&f.ws, &offer.tab, take).0.unwrap();
    let ns = f.acme_ns.clone();
    assert_that!(f.with("t-1", |s| s.user_deck().widgets[&ns].version.clone())).is_equal_to(v);
}
