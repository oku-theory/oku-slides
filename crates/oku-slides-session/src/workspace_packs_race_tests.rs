use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::json;
use speculoos::prelude::*;

use super::workspace_packs_fixtures::{Packs, outcome, packs, within};
use super::{Opener, opening_entry, opening_waiters};
use crate::agent::PackGate;
use crate::pack_lease::PackLease;
use crate::test_support::{apply_ws, fresh_dir, settings_on};
use crate::test_support_packs::assert_unleased;
use crate::workspace::{Created, Opening, Pending, TabPlace};
use crate::{BASE_PACK, Fonts, Platform, Session, SessionErrorKind, TabDocument, Workspace};

const WAIT: Duration = Duration::from_secs(10);

/// The pack tabs of the workspace.
fn pack_tabs(f: &Packs) -> usize {
    Workspace::lock(&f.ws).tabs.values().filter(|t| matches!(t.info.document, TabDocument::Pack { .. })).count()
}

/// Opens `acme` from `t-1` on another thread for an agent whose gate says it entered, then waits.
///
/// Returns the "entered" receiver, the release sender and the receiver of `(outcome, gate calls)`.
fn held_open(f: &Packs) -> (mpsc::Receiver<()>, mpsc::Sender<()>, mpsc::Receiver<(String, usize)>) {
    let (ws, acme) = (Arc::clone(&f.ws), f.acme);
    let (entered_tx, entered) = mpsc::channel();
    let (release, release_rx) = mpsc::channel::<()>();
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        let mut asked = 0;
        let mut gate = |_: PackGate| {
            asked += 1;
            let _ = entered_tx.send(());
            let _ = release_rx.recv_timeout(WAIT);
            true
        };
        let r = outcome(Workspace::open_pack_with(&ws, "t-1", acme, None, Opener::Agent(&mut gate), Created::No));
        let _ = done.send((r, asked));
    });
    (entered, release, finished)
}

/// Opens `pack` from `tab` as the user on another thread; the receiver of its outcome.
fn open_on_a_thread(f: &Packs, tab: &str, pack: uuid::Uuid) -> mpsc::Receiver<String> {
    let (ws, tab) = (Arc::clone(&f.ws), tab.to_owned());
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = done.send(outcome(Workspace::open_pack_with(&ws, &tab, pack, None, Opener::Ui, Created::No)));
    });
    finished
}

#[test]
fn open_pack_twice_with_a_widget_on_one_thread_never_deadlocks() {
    let f = packs();
    let (ws, acme) = (Arc::clone(&f.ws), f.acme);
    let outcomes = within(move || {
        let open = || outcome(Workspace::open_pack_with(&ws, "t-1", acme, Some("card"), Opener::Ui, Created::No));
        (open(), open())
    });
    assert_that!(outcomes).is_equal_to(("Opened(\"t-2\")".to_owned(), "Focused(\"t-2\")".to_owned()));
}

#[test]
fn open_pack_twice_with_a_widget_on_two_threads_never_deadlocks() {
    let f = packs();
    let (done, finished) = mpsc::channel();
    for _ in 0..2 {
        let (ws, acme, done) = (Arc::clone(&f.ws), f.acme, done.clone());
        std::thread::spawn(move || {
            let _ =
                done.send(outcome(Workspace::open_pack_with(&ws, "t-1", acme, Some("card"), Opener::Ui, Created::No)));
        });
    }
    let mut got: Vec<String> = (0..2).map(|_| finished.recv_timeout(WAIT).expect("deadlock")).collect();
    got.sort();
    assert_that!(got).is_equal_to(vec!["Focused(\"t-2\")".to_owned(), "Opened(\"t-2\")".to_owned()]);
}

/// A's open of P is held in its gate; B's open of P waits on P's opening lock, then focuses A's tab.
#[test]
fn open_pack_from_two_decks_at_once_opens_one_pack_tab() {
    let f = packs();
    let b = f.new_tab();
    let (entered, release, a_done) = held_open(&f);
    entered.recv_timeout(WAIT).expect("A's gate entered");
    let b_done = open_on_a_thread(&f, &b, f.acme);
    let deadline = Instant::now() + WAIT;
    while opening_waiters(f.acme) != 1 {
        assert_that!(b_done.try_recv().is_err()).is_true();
        assert_that!(Instant::now() < deadline).is_true();
        std::thread::sleep(Duration::from_millis(2));
    }
    release.send(()).unwrap();
    let (a, asked) = a_done.recv_timeout(WAIT).expect("A answered");
    let b = b_done.recv_timeout(WAIT).expect("B answered");
    assert_that!((a, b, asked, pack_tabs(&f))).is_equal_to((
        "Opened(\"t-3\")".to_owned(),
        "Focused(\"t-3\")".to_owned(),
        1,
        1,
    ));
}

#[test]
fn open_pack_of_another_pack_never_waits_on_a_busy_open() {
    let f = packs();
    let b = f.new_tab();
    let (entered, release, a_done) = held_open(&f);
    entered.recv_timeout(WAIT).expect("A's gate entered");
    let other = open_on_a_thread(&f, &b, BASE_PACK).recv_timeout(WAIT);
    let a_still_held = a_done.try_recv().is_err();
    release.send(()).unwrap();
    a_done.recv_timeout(WAIT).expect("A answered");
    assert_that!((other.is_ok_and(|o| o.starts_with("Opened")), a_still_held)).is_equal_to((true, true));
}

/// A ready pack tab of P whose cached document lags: the lease refusal finds it live.
#[test]
fn open_pack_refused_by_a_lagging_pack_tab_focuses_it() {
    let f = packs();
    let p = f.pack_tab();
    f.activate("t-1");
    Workspace::lock(&f.ws).tabs.get_mut(&p).unwrap().info.document = TabDocument::Deck { path: None, deck_uid: None };
    let mut asked = Vec::new();
    let r = f.open_by_agent("t-1", f.acme, &mut |g| {
        asked.push(g);
        true
    });
    assert_that!((outcome(r), asked)).is_equal_to((format!("Focused({p:?})"), vec![PackGate::NewTab]));
}

#[test]
fn open_pack_refused_by_another_workspace_reports_the_holder() {
    let f = packs();
    let other_data = fresh_dir("oku-slides-ws-packs-other-");
    let mut settings = settings_on(other_data.path());
    settings.paths.widgets = Some(f.data.path().join("widgets"));
    settings.paths.layouts = Some(f.data.path().join("layouts"));
    settings.paths.themes = Some(f.data.path().join("themes"));
    let platform = Platform::with_settings(settings).unwrap();
    let other = Mutex::new(Workspace::new(Fonts::default(), &platform, None).unwrap());
    Workspace::open_pack_with(&other, "t-1", f.acme, None, Opener::Ui, Created::No).unwrap();
    let held = TabDocument::Pack { pack: f.acme };
    let holder = Workspace::lock(&other).tabs.values().find(|t| t.info.document == held).map(|t| t.info.title.clone());
    let holder = holder.unwrap_or_else(|| panic!("the other workspace's pack tab"));
    let err = f.open("t-1", f.acme, None).unwrap_err();
    assert_that!((err.kind(), err.lease_holder()))
        .is_equal_to((SessionErrorKind::PackOpenInTab, Some(holder.as_str())));
}

/// A pack tab of `acme` built by hand for `window`, after `anchor`, not yet finished.
fn built(f: &Packs, window: &str, anchor: &str) -> (Pending, Session) {
    let seed = f.with("t-1", |s| s.take_preview(f.acme)).unwrap();
    let lease = PackLease::acquire(&f.scope, f.acme, "acme", "acme").unwrap();
    let opening = Opening::Pack { pack: f.acme, widget: None, created: Created::No };
    let pending = Workspace::lock(&f.ws).pending(Some(window.into()), opening, TabPlace::After(anchor.into()));
    let session = pending.build_pack(*seed, lease).unwrap();
    (pending, session)
}

/// Finishes `pending`: the new tab's id.
fn finish(f: &Packs, pending: Pending, session: Session) -> String {
    Workspace::finish(&f.ws, pending, session).map_or_else(|e| panic!("{}", e.one_line()), |(_, id)| id)
}

#[test]
fn open_pack_finish_appends_when_the_anchor_left_the_window() {
    let f = packs();
    let x = f.new_tab();
    let (pending, session) = built(&f, "w-1", "t-1");
    f.send(json!({"cmd": "move_tab", "tab": "t-1"}));
    let id = finish(&f, pending, session);
    assert_that!((f.window(0), f.opener_of(&id))).is_equal_to((vec![x, id.clone()], Some("t-1".to_owned())));
}

#[test]
fn open_pack_finish_when_the_anchor_closed_appends_without_opener() {
    let f = packs();
    let x = f.new_tab();
    let y = f.new_tab();
    f.send(json!({"cmd": "reorder", "tab": "t-1", "index": 1}));
    let (pending, session) = built(&f, "w-1", "t-1");
    f.close("t-1");
    let id = finish(&f, pending, session);
    assert_that!((f.window(0), f.opener_of(&id))).is_equal_to((vec![x, y, id.clone()], None));
}

#[test]
fn open_pack_finish_when_the_window_closed_meanwhile_lands_in_the_focused_window() {
    let f = packs();
    f.send(json!({"cmd": "new_window"}));
    let (pending, session) = built(&f, "w-2", "t-2");
    f.send(json!({"cmd": "close_window", "window": "w-2", "discard": ["t-2"]}));
    let id = finish(&f, pending, session);
    assert_that!((f.window(0), f.opener_of(&id))).is_equal_to((vec!["t-1".to_owned(), id.clone()], None));
}

#[test]
fn open_pack_finish_refuses_a_closed_workspace() {
    let f = packs();
    let (pending, session) = built(&f, "w-1", "t-1");
    Workspace::settle(&f.ws);
    let refused = Workspace::finish(&f.ws, pending, session).is_err();
    assert_unleased(&f.scope, f.acme);
    assert_that!((refused, pack_tabs(&f))).is_equal_to((true, 0));
}

#[test]
fn open_pack_whose_requesting_tab_closed_during_the_gate_is_refused() {
    let f = packs();
    f.new_tab();
    let (ws, closing_ws, acme) = (Arc::clone(&f.ws), Arc::clone(&f.ws), f.acme);
    let r = within(move || {
        let mut gate = |_: PackGate| {
            let ws = Arc::clone(&closing_ws);
            let closed =
                std::thread::spawn(move || apply_ws(&ws, json!({"cmd": "close", "tab": "t-1", "discard": true})));
            closed.join().is_ok_and(|r| r.is_ok())
        };
        outcome(Workspace::open_pack_with(&ws, "t-1", acme, None, Opener::Agent(&mut gate), Created::No))
    });
    assert_unleased(&f.scope, f.acme);
    assert_that!((r, pack_tabs(&f), opening_entry(f.acme))).is_equal_to(("UnknownTab".to_owned(), 0, false));
}
