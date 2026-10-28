use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use oku_slides_core::{Element, Namespace};
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::super::packs::workspace_packs_fixtures::{
    Packs, assert_outcome, deck_packs, deck_packs_resumed, within, within_secs,
};
use super::super::{Slot, relock};
use super::BEFORE_OWNER_LOCK;
use crate::agent::PackGate;
use crate::deck_pack_receive::Received;
use crate::test_support::session_on;
use crate::test_support_packs::{CARD_SRC, ok};
use crate::{Mode, Request, Session, TabDocument, Workspace};

/// The answer a test waits for: ten seconds, then a deadlock.
const WAIT: Duration = Duration::from_secs(10);

/// The first widget of deck-only `pack` in `t-1`'s deck.
fn first_widget(f: &Packs, pack: Uuid) -> String {
    f.with("t-1", |s| {
        let pin = s.user_deck().all_pins().find(|(_, _, p)| p.id == pack).map(|(_, _, p)| p.version.clone());
        let files = pin.and_then(|v| s.doc.sources.packs.get(&v).cloned()).unwrap_or_else(|| panic!("embedded"));
        let widget = files.widgets().keys().find(|w| !w.is_common()).map(|w| w.as_str().to_owned());
        widget.unwrap_or_else(|| panic!("a widget"))
    })
}

/// The tabs `tab`'s ready session is linked to; empty when it sleeps.
fn links_in(f: &Packs, tab: &str) -> Vec<String> {
    let session = Workspace::lock(&f.ws).session(tab).unwrap_or_else(|e| panic!("{tab}: {}", e.one_line()));
    match &*relock(&session) {
        Slot::Ready(s) => s.deck_links.keys().cloned().collect(),
        Slot::Dormant(_) | Slot::Closed(_) => Vec::new(),
    }
}

/// Counts the forwards that reach the owner's lock from now on.
fn count_forwards() -> Arc<AtomicUsize> {
    let count = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&count);
    *BEFORE_OWNER_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::new(move || {
        seen.fetch_add(1, Ordering::SeqCst);
    }));
    count
}

/// Holds `tab`'s session on another thread until the returned sender fires; `released` is set first.
fn hold(f: &Packs, tab: &str, released: Arc<AtomicBool>) -> mpsc::Sender<()> {
    let session = Workspace::lock(&f.ws).session(tab).unwrap_or_else(|e| panic!("{tab}: {}", e.one_line()));
    let (held, holding) = mpsc::channel();
    let (release, wait) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        let guard = relock(&session);
        let _ = held.send(());
        let _ = wait.recv_timeout(WAIT);
        released.store(true, Ordering::SeqCst);
        drop(guard);
    });
    holding.recv_timeout(WAIT).unwrap_or_else(|_| panic!("session not held"));
    release
}

/// A pack tab's request answered on another thread within ten seconds: whether it succeeded.
fn answers(f: &Packs, tab: &str) -> bool {
    let (ws, tab) = (Arc::clone(&f.ws), tab.to_owned());
    let (done, answered) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = done.send(Workspace::request(&ws, &tab, Request::Snapshot).0.is_ok());
    });
    answered.recv_timeout(WAIT).unwrap_or(false)
}

/// Moves the link of `tab` in owner `t-1` to expect another version: its next send is refused.
fn misdirect_link(f: &Packs, tab: &str) {
    let ns = f.acme_ns.clone();
    f.with("t-1", |s| {
        let other = s.user_deck().widgets[&ns].version.clone();
        s.deck_links.get_mut(tab).unwrap_or_else(|| panic!("a link")).expect = other;
    });
}

/// The notice `tab` shows.
fn notice_of(f: &Packs, tab: &str) -> Option<String> {
    f.with(tab, |s| s.doc.notice.clone())
}

#[test]
fn open_pack_of_a_deck_only_pack_opens_a_deck_pack_tab_after_its_owner() {
    within(|| {
        let (f, pin) = deck_packs();
        let other = f.new_tab();
        f.activate("t-1");
        let p = f.deck_pack_tab(pin.id);
        assert_that!(f.window(0)).is_equal_to(vec!["t-1".to_owned(), p.clone(), other]);
        assert_that!(f.document(&p))
            .is_equal_to(Some(TabDocument::DeckPack { deck_tab: "t-1".to_owned(), pack: pin.id }));
        assert_that!(f.active(0)).is_equal_to(&p);
        assert_that!(f.with("t-1", |s| matches!(s.mode, Mode::Deck))).is_true();
    });
}

#[test]
fn open_pack_twice_focuses_existing_tab() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        f.activate("t-1");
        let count = f.tab_count();
        assert_outcome(f.open("t-1", pin.id, None), &format!("Focused({p:?})"));
        assert_that!((f.active(0), f.tab_count())).is_equal_to((p, count));
    });
}

#[test]
fn open_pack_from_the_owner_with_a_widget_focuses_it_on_that_widget() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        f.activate("t-1");
        let widget = first_widget(&f, pin.id);
        assert_outcome(f.open("t-1", pin.id, Some(&widget)), &format!("Focused({p:?})"));
        assert_that!(f.with(&p, |s| s.ui.widget.clone())).is_equal_to(Some(widget));
    });
}

#[test]
fn open_pack_from_the_deck_pack_tab_opens_the_widget_there() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        let widget = first_widget(&f, pin.id);
        assert_outcome(f.open(&p, pin.id, Some(&widget)), "Here");
        assert_that!(f.with(&p, |s| s.ui.widget.clone())).is_equal_to(Some(widget));
    });
}

#[test]
fn deck_pack_save_reaches_owner_through_the_workspace() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        drop(Workspace::take_touched(&f.ws));
        f.save_in(&p, "Importé 2");
        assert_that!(Workspace::take_touched(&f.ws)).is_equal_to(vec!["t-1".to_owned()]);
        assert_that!(Workspace::take_touched(&f.ws)).is_empty();
        let pinned = f.pinned("t-1", pin.id);
        assert_that!((pinned.is_some(), pinned != Some(pin.version.clone()))).is_equal_to((true, true));
        assert_that!((f.dirty("t-1"), f.unsent(&p))).is_equal_to((true, false));
    });
}

#[test]
fn deck_pack_forward_wakes_a_dormant_owner() {
    within_secs(60, || {
        let (f, pin) = deck_packs_resumed();
        let p = f.deck_pack_tab(pin.id);
        let f = f.restart(|_, _| {});
        f.edit_in(&p, "Importé 2");
        assert_that!(f.is_dormant("t-1")).is_true();
        f.save_pack(&p, false).unwrap_or_else(|e| panic!("save: {}", e.one_line()));
        assert_that!(f.pinned("t-1", pin.id)).is_not_equal_to(Some(pin.version.clone()));
    });
}

#[test]
fn resumed_deck_pack_tab_save_reaches_owner() {
    within_secs(60, || {
        let (f, pin) = deck_packs_resumed();
        let p = f.deck_pack_tab(pin.id);
        f.save_in(&p, "Importé 2");
        let first = f.pinned("t-1", pin.id);
        let f = f.restart(|_, _| {});
        f.save_in(&p, "Importé 3");
        let second = f.pinned("t-1", pin.id);
        assert_that!((second.is_some(), second != first, f.unsent(&p))).is_equal_to((true, true, false));
    });
}

#[test]
fn deck_pack_copy_through_the_workspace_retargets_the_tab() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        f.edit_in(&p, "Copie");
        f.save_pack(&p, true).unwrap_or_else(|e| panic!("copy: {}", e.one_line()));
        let Some(TabDocument::DeckPack { pack: copy, .. }) = f.document(&p) else { panic!("a deck-pack tab") };
        assert_that!(copy).is_not_equal_to(pin.id);
        let before = f.pinned("t-1", copy);
        assert_that!(before).is_some();
        f.save_in(&p, "Copie 2");
        assert_that!(f.pinned("t-1", copy)).is_not_equal_to(before);
    });
}

#[test]
fn delivered_copy_after_a_lost_acknowledgement_retargets_the_tab() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        f.edit_in(&p, "Copie");
        let taken = Workspace::with_quiet(&f.ws, &p, |s| {
            ok(s.save_pack(true), "copy");
            s.mark_due();
            s.take_deck_pack_send()
        });
        let send = taken.0.ok().flatten().unwrap_or_else(|| panic!("a due send"));
        // The owner applies the copy; its acknowledgement never reaches the tab (a crash, a failed sync).
        let received = Workspace::with_quiet(&f.ws, "t-1", |o| o.receive_deck_pack(&p, &send)).0;
        assert_that!(received.ok().and_then(Result::ok)).is_equal_to(Some(Received::Applied));
        assert_that!(Workspace::with_quiet(&f.ws, &p, |s| s.park_deck_pack(send.seq)).0).is_ok();
        f.with(&p, Session::mark_due);
        let Some(TabDocument::DeckPack { pack: copy, .. }) = f.document(&p) else { panic!("a deck-pack tab") };
        assert_that!((copy != pin.id, f.pinned("t-1", copy).is_some(), f.unsent(&p))).is_equal_to((true, true, false));
    });
}

#[test]
fn agent_open_pack_of_a_deck_only_pack_counts_as_a_new_tab() {
    within(|| {
        let (f, pin) = deck_packs();
        let count = f.tab_count();
        let mut asked = Vec::new();
        let r = f.open_by_agent("t-1", pin.id, &mut |g| {
            asked.push(g);
            false
        });
        assert_outcome(r, "Refused");
        assert_that!((asked, f.tab_count())).is_equal_to((vec![PackGate::NewTab], count));
        assert_that!(f.owner_write("t-1")).is_ok();
    });
}

#[test]
fn failed_deck_pack_build_unlinks_the_owner() {
    within(|| {
        let (f, pin) = deck_packs();
        let count = f.tab_count();
        super::super::tabs::FAIL_DECK_PACK_BUILD.store(true, Ordering::SeqCst);
        assert_that!(f.open("t-1", pin.id, None)).is_err();
        assert_that!(f.tab_count()).is_equal_to(count);
        assert_that!(f.owner_write("t-1")).is_ok();
    });
}

#[test]
fn owner_closed_during_deck_pack_build_inserts_no_tab() {
    within(|| {
        let (f, pin) = deck_packs();
        let other = f.new_tab();
        let ws = Arc::clone(&f.ws);
        let hook = move || {
            let close = serde_json::from_value(json!({"cmd": "close", "tab": "t-1", "discard": true}));
            let closed = Workspace::apply(&ws, close.unwrap_or_else(|e| panic!("request: {e}")));
            closed.unwrap_or_else(|e| panic!("close: {}", e.one_line()));
        };
        *super::super::tabs::DURING_DECK_PACK_BUILD.lock().unwrap_or_else(std::sync::PoisonError::into_inner) =
            Some(Box::new(hook));
        assert_that!(f.open("t-1", pin.id, None)).is_err();
        assert_that!(Workspace::lock(&f.ws).tab_ids()).is_equal_to(vec![other]);
    });
}

#[test]
fn two_owners_of_one_pack_get_two_tabs() {
    within(|| {
        let (f, pin) = deck_packs();
        let copy = f.deck.path().join(format!("copie{}", crate::EXTENSION));
        f.with("t-1", |s| ok(s.save_copy(&copy), "save copy"));
        let second = Workspace::open_beside(&f.ws, "t-1", &copy).unwrap_or_else(|e| panic!("{}", e.one_line()));
        let first_tab = f.deck_pack_tab(pin.id);
        let second_tab = match f.open(&second, pin.id, None) {
            Ok(super::PackOpened::Opened(id)) => id,
            other => panic!("a second deck-pack tab: {other:?}"),
        };
        assert_that!(second_tab).is_not_equal_to(&first_tab);
        assert_that!(f.document(&second_tab))
            .is_equal_to(Some(TabDocument::DeckPack { deck_tab: second, pack: pin.id }));
    });
}

#[test]
fn forward_waits_for_a_held_owner() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        f.edit_in(&p, "Importé 2");
        drop(Workspace::take_touched(&f.ws));
        let (hooked, before_owner) = mpsc::channel();
        *BEFORE_OWNER_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::new(move || {
            let _ = hooked.send(());
        }));
        let released = Arc::new(AtomicBool::new(false));
        let release = hold(&f, "t-1", Arc::clone(&released));
        let (ws, tab, seen) = (Arc::clone(&f.ws), p.clone(), Arc::clone(&released));
        let (done, saved) = mpsc::channel();
        std::thread::spawn(move || {
            let ok = Workspace::request(&ws, &tab, Request::SavePack { as_new: false }).0.is_ok();
            let _ = done.send((ok, seen.load(Ordering::SeqCst), Workspace::take_touched(&ws)));
        });
        before_owner.recv_timeout(WAIT).unwrap_or_else(|_| panic!("the send was never taken"));
        assert_that!(answers(&f, &p)).is_true();
        let _ = release.send(());
        let outcome = saved.recv_timeout(WAIT).unwrap_or_else(|_| panic!("the save never returned"));
        assert_that!(outcome).is_equal_to((true, true, vec!["t-1".to_owned()]));
    });
}

#[test]
fn refused_send_is_not_forwarded_again() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        misdirect_link(&f, &p);
        let forwards = count_forwards();
        f.save_in(&p, "Importé 2");
        assert_that!((forwards.load(Ordering::SeqCst), f.unsent(&p))).is_equal_to((1, true));
        let release = hold(&f, "t-1", Arc::new(AtomicBool::new(false)));
        assert_that!(answers(&f, &p)).is_true();
        let _ = release.send(());
        assert_that!(forwards.load(Ordering::SeqCst)).is_equal_to(1);
    });
}

#[test]
fn forward_during_owner_turn_is_pulled_at_end_turn() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        f.with("t-1", |s| s.begin_turn(1, "mise en page"));
        f.save_in(&p, "Importé 2");
        assert_that!((f.pinned("t-1", pin.id), notice_of(&f, &p))).is_equal_to((Some(pin.version.clone()), None));
        f.with("t-1", crate::Session::end_turn);
        assert_that!(f.pinned("t-1", pin.id)).is_not_equal_to(Some(pin.version.clone()));
        assert_that!(f.unsent(&p)).is_false();
    });
}

#[test]
fn internal_calls_never_forward() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        f.edit_in(&p, "Importé 2");
        let saved = Workspace::with_quiet(&f.ws, &p, |s| {
            let saved = s.save_pack(false).map(|_| ());
            s.mark_due();
            saved
        });
        assert_that!(saved.0.and_then(|r| r)).is_ok();
        let forwards = count_forwards();
        f.activate("t-1");
        assert_outcome(f.open("t-1", pin.id, None), &format!("Focused({p:?})"));
        let due = Workspace::with_quiet(&f.ws, &p, |s| s.take_deck_pack_send().is_some()).0;
        assert_that!((forwards.load(Ordering::SeqCst), due.ok())).is_equal_to((0, Some(true)));
    });
}

/// Whether `v` holds key `key` at any depth.
fn has_key(v: &Value, key: &str) -> bool {
    match v {
        Value::Object(map) => map.iter().any(|(k, v)| k == key || has_key(v, key)),
        Value::Array(items) => items.iter().any(|v| has_key(v, key)),
        _ => false,
    }
}

#[test]
fn forward_is_persisted_on_both_sides() {
    within_secs(60, || {
        let (f, pin) = deck_packs_resumed();
        let p = f.deck_pack_tab(pin.id);
        f.save_in(&p, "Importé 2");
        let forwarded = f.pinned("t-1", pin.id);
        let f = f.restart(|resume, _| {
            let state = std::fs::read(resume.join("tabs").join(&p).join("state.json")).unwrap();
            let state: Value = serde_json::from_slice(&state).unwrap();
            assert_that!(has_key(&state, "unforwarded")).is_false();
        });
        assert_that!(f.pinned("t-1", pin.id)).is_equal_to(forwarded);
    });
}

#[test]
fn links_reach_the_owner_through_carry_and_notices() {
    within_secs(60, || {
        let (f, pin) = deck_packs_resumed();
        let p = f.deck_pack_tab(pin.id);
        let other = f.new_tab();
        let element: Element = f.with("t-1", |s| {
            let found = s.deck().slides.iter().find_map(|x| x.locate("c-1").map(|l| l.element.clone()));
            found.unwrap_or_else(|| panic!("c-1"))
        });
        let f = f.restart(|_, _| {});
        f.send(json!({"cmd": "carry_assets", "from": other, "to": "t-1", "element": element}));
        assert_that!(links_in(&f, "t-1")).is_equal_to(vec![p.clone()]);
        let f = f.restart(|_, _| {});
        let session = Workspace::lock(&f.ws).session("t-1").unwrap_or_else(|e| panic!("{}", e.one_line()));
        Workspace::post_notice(&f.ws, "t-1", &session, "avis".to_owned());
        assert_that!(links_in(&f, "t-1")).is_equal_to(vec![p]);
        assert_that!(f.owner_write("t-1").map_err(|e| e.kind()))
            .is_equal_to(Err(crate::SessionErrorKind::PackOpenInTab));
    });
}

#[test]
fn drain_touched_emits_once_then_runs_after_per_owner() {
    within(|| {
        let (f, _) = deck_packs();
        Workspace::lock(&f.ws).touched = vec!["t-1".to_owned(), "t-3".to_owned()];
        let (mut emitted, mut after) = (0, Vec::new());
        Workspace::drain_touched(&f.ws, || emitted += 1, |o| after.push(o.to_owned()));
        assert_that!((emitted, after)).is_equal_to((1, vec!["t-1".to_owned(), "t-3".to_owned()]));
        let mut again = 0;
        Workspace::drain_touched(&f.ws, || again += 1, |_| panic!("no owner left"));
        assert_that!(again).is_equal_to(0);
    });
}

#[test]
fn owner_absorb_on_save_then_refuses_the_next_send() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        let path = f.with("t-1", |s| s.doc.path.clone()).unwrap_or_else(|| panic!("a file"));
        let mut other = session_on(f.data.path());
        ok(other.open(&path), "open elsewhere");
        ok(other.set_widget(Some(&Namespace::deck()), "card", CARD_SRC.into()), "deck-only write elsewhere");
        other.end_widget_session();
        ok(other.save(None), "save elsewhere");
        crate::test_support::touch_elsewhere(&path);
        let saved = Workspace::request(&f.ws, "t-1", Request::Save { path: None, overwrite: false }).0;
        assert_that!(saved).is_ok();
        assert_that!(f.pinned("t-1", pin.id)).is_not_equal_to(Some(pin.version.clone()));
        f.save_in(&p, "Importé 2");
        assert_that!((f.unsent(&p), notice_of(&f, &p).is_some())).is_equal_to((true, true));
    });
}

#[test]
fn pack_opener_of_a_deck_pack_tab_is_none() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        assert_that!(Workspace::pack_opener(&f.ws, &p).ok().flatten()).is_none();
    });
}

#[test]
fn note_refuses_a_changed_deck_tab() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        let mut w = Workspace::lock(&f.ws);
        let cached = w.tabs[&p].info.clone();
        let mut info = cached.clone();
        info.document = TabDocument::DeckPack { deck_tab: "t-9".to_owned(), pack: pin.id };
        info.title = "Autre".to_owned();
        assert_that!(w.note(&p, info)).is_false();
        assert_that!(w.tabs[&p].info).is_equal_to(cached);
    });
}

#[test]
fn note_keeps_the_pack_without_an_accepted_copy() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        let mut w = Workspace::lock(&f.ws);
        let mut info = w.tabs[&p].info.clone();
        info.document = TabDocument::DeckPack { deck_tab: "t-1".to_owned(), pack: Uuid::new_v4() };
        info.title = "Copie".to_owned();
        assert_that!(w.note(&p, info)).is_true();
        let kept = (w.tabs[&p].info.document.clone(), w.tabs[&p].info.title.clone());
        assert_that!(kept)
            .is_equal_to((TabDocument::DeckPack { deck_tab: "t-1".to_owned(), pack: pin.id }, "Copie".to_owned()));
    });
}
