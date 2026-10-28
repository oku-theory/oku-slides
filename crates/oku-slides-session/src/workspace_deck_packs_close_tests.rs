use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};
use speculoos::prelude::*;

use super::super::deck_packs::BEFORE_OWNER_LOCK;
use super::super::packs::workspace_packs_fixtures::{Packs, deck_packs, deck_packs_resumed, within, within_secs};
use super::FLUSH_ALL_SCANS;
use crate::{Result, Session, SessionErrorKind, Workspace, WorkspaceOutcome};

/// `apply` on a JSON request.
fn apply(f: &Packs, v: Value) -> Result<WorkspaceOutcome> {
    Workspace::apply(&f.ws, serde_json::from_value(v).unwrap_or_else(|e| panic!("request: {e}")))
}

/// Closes `tab`, keeping or discarding its work.
fn close(f: &Packs, tab: &str, discard: bool) -> Result<WorkspaceOutcome> {
    apply(f, json!({"cmd": "close", "tab": tab, "discard": discard}))
}

/// The outcome's variant name, or the error's kind.
fn kind(r: &Result<WorkspaceOutcome>) -> String {
    match r {
        Ok(WorkspaceOutcome::LastWindow(_)) => "LastWindow".to_owned(),
        Ok(WorkspaceOutcome::DeckChanged { tab, .. }) => format!("DeckChanged({tab})"),
        Ok(WorkspaceOutcome::Applied(_)) => "Applied".to_owned(),
        Ok(other) => format!("{other:?}"),
        Err(e) => format!("{:?}", e.kind()),
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

/// Every window's label and tabs, in order.
fn windows(f: &Packs) -> Vec<(String, Vec<String>)> {
    Workspace::lock(&f.ws).layout.windows.iter().map(|w| (w.label.clone(), w.tabs.clone())).collect()
}

/// Moves `tab` to a new window: its label.
fn to_new_window(f: &Packs, tab: &str) -> String {
    f.send(json!({"cmd": "move_tab", "tab": tab}));
    windows(f).into_iter().find(|(_, tabs)| tabs.iter().any(|t| t == tab)).map(|(label, _)| label).unwrap()
}

/// Whether the workspace still has tab `tab`.
fn has(f: &Packs, tab: &str) -> bool {
    Workspace::lock(&f.ws).tabs.contains_key(tab)
}

#[test]
fn deck_pack_tab_closes_with_owner_deck() {
    within(|| {
        let (f, pin) = deck_packs();
        let other = f.new_tab();
        let p = f.deck_pack_tab(pin.id);
        assert_that!(kind(&close(&f, "t-1", false))).is_equal_to("Applied".to_owned());
        assert_that!((has(&f, "t-1"), has(&f, &p), f.window(0))).is_equal_to((false, false, vec![other]));
    });
}

#[test]
fn closing_owner_flushes_a_dirty_deck_pack_tab_first() {
    within(|| {
        let (f, pin) = deck_packs();
        f.new_tab();
        let p = f.deck_pack_tab(pin.id);
        f.edit_in(&p, "Importé 2");
        assert_that!(kind(&close(&f, "t-1", false))).is_equal_to("Unsaved".to_owned());
        assert_that!((has(&f, "t-1"), has(&f, &p), f.dirty("t-1"))).is_equal_to((true, true, true));
        assert_that!(f.pinned("t-1", pin.id)).is_not_equal_to(Some(pin.version.clone()));
    });
}

#[test]
fn closing_owner_with_discard_closes_both_without_flushing() {
    within(|| {
        let (f, pin) = deck_packs();
        f.new_tab();
        let p = f.deck_pack_tab(pin.id);
        f.edit_in(&p, "Importé 2");
        let forwards = count_forwards();
        assert_that!(kind(&close(&f, "t-1", true))).is_equal_to("Applied".to_owned());
        assert_that!((has(&f, "t-1"), has(&f, &p), forwards.load(Ordering::SeqCst))).is_equal_to((false, false, 0));
    });
}

#[test]
fn closing_owner_and_links_that_are_all_tabs_is_last_window() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        assert_that!(kind(&close(&f, "t-1", false))).is_equal_to("LastWindow".to_owned());
        assert_that!((has(&f, "t-1"), has(&f, &p))).is_equal_to((true, true));
    });
}

#[test]
fn close_window_emptying_every_window_is_last_window() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        to_new_window(&f, &p);
        let before = windows(&f);
        let r = apply(&f, json!({"cmd": "close_window", "window": "w-1"}));
        assert_that!(kind(&r)).is_equal_to("LastWindow".to_owned());
        assert_that!(windows(&f)).is_equal_to(before);
    });
}

#[test]
fn close_window_closes_linked_tabs_in_other_windows() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        let other = f.new_tab();
        let linked = to_new_window(&f, &p);
        let kept = to_new_window(&f, &other);
        assert_that!(close_window(&f, "w-1")).is_ok();
        assert_that!((has(&f, "t-1"), has(&f, &p))).is_equal_to((false, false));
        let labels: Vec<String> = windows(&f).into_iter().map(|(label, _)| label).collect();
        assert_that!(labels.contains(&linked)).is_false();
        assert_that!(labels).is_equal_to(vec![kept]);
    });
}

/// Closes window `window`, keeping its work.
fn close_window(f: &Packs, window: &str) -> Result<WorkspaceOutcome> {
    apply(f, json!({"cmd": "close_window", "window": window}))
}

#[test]
fn close_window_flushes_linked_tabs_first() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        let other = f.new_tab();
        let linked = to_new_window(&f, &p);
        f.send(json!({"cmd": "move_tab", "tab": other, "window": linked}));
        f.edit_in(&p, "Importé 2");
        assert_that!(kind(&close_window(&f, "w-1"))).is_equal_to("Unsaved".to_owned());
        assert_that!((f.unsent(&p), f.dirty("t-1"))).is_equal_to((false, true));
        assert_that!(f.pinned("t-1", pin.id)).is_not_equal_to(Some(pin.version.clone()));
    });
}

#[test]
fn closing_a_deck_pack_tab_alone_unlinks_and_activates_its_owner() {
    within(|| {
        let (f, pin) = deck_packs();
        f.new_tab();
        let p = f.deck_pack_tab(pin.id);
        f.edit_in(&p, "Importé 2");
        drop(Workspace::take_touched(&f.ws));
        assert_that!(kind(&close(&f, &p, false))).is_equal_to("DeckChanged(t-1)".to_owned());
        assert_that!((has(&f, &p), f.active(0))).is_equal_to((false, "t-1".to_owned()));
        assert_that!(f.owner_write("t-1")).is_ok();
    });
}

#[test]
fn closing_a_just_opened_deck_pack_tab_drops_its_pending_link() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        assert_that!(kind(&close(&f, &p, false))).is_equal_to("Applied".to_owned());
        assert_that!(f.owner_write("t-1")).is_ok();
    });
}

#[test]
fn settle_flushes_deck_pack_tabs_before_closing() {
    within_secs(60, || {
        let (f, pin) = deck_packs_resumed();
        let p = f.deck_pack_tab(pin.id);
        f.edit_in(&p, "Importé 2");
        Workspace::settle(&f.ws);
        let f = f.restart(|_, _| {});
        assert_that!(f.pinned("t-1", pin.id)).is_not_equal_to(Some(pin.version.clone()));
    });
}

#[test]
fn second_settle_forwards_nothing() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        f.edit_in(&p, "Importé 2");
        Workspace::settle(&f.ws);
        let (forwards, scans) = (count_forwards(), FLUSH_ALL_SCANS.load(Ordering::SeqCst));
        Workspace::settle(&f.ws);
        // The second settle stops at the closed check, before scanning any tab.
        let after = (forwards.load(Ordering::SeqCst), FLUSH_ALL_SCANS.load(Ordering::SeqCst));
        assert_that!((scans, after)).is_equal_to((1, (0, 1)));
    });
}

#[test]
fn discarded_close_that_is_last_window_is_not_flushed_at_quit() {
    within_secs(60, || {
        let (f, pin) = deck_packs_resumed();
        let p = f.deck_pack_tab(pin.id);
        f.edit_in(&p, "Importé 2");
        assert_that!(kind(&close(&f, "t-1", true))).is_equal_to("LastWindow".to_owned());
        let f = f.restart(|_, _| {});
        assert_that!(f.pinned("t-1", pin.id)).is_equal_to(Some(pin.version.clone()));
    });
}

#[test]
fn discarded_last_close_drops_an_unforwarded_save() {
    within_secs(60, || {
        let (f, pin) = deck_packs_resumed();
        let p = f.deck_pack_tab(pin.id);
        f.with("t-1", |s| s.begin_turn(1, "mise en page"));
        f.save_in(&p, "Importé 2");
        assert_that!(f.unsent(&p)).is_true();
        assert_that!(kind(&close(&f, "t-1", true))).is_equal_to("LastWindow".to_owned());
        let f = f.restart(|_, _| {});
        // A public call on the tab would forward a save resumed as unforwarded.
        f.with(&p, |_| ());
        assert_that!((f.pinned("t-1", pin.id), f.unsent(&p))).is_equal_to((Some(pin.version.clone()), false));
    });
}

#[test]
fn discard_at_quit_is_forgotten_when_its_tab_closes() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        assert_that!(kind(&close(&f, "t-1", true))).is_equal_to("LastWindow".to_owned());
        f.new_tab();
        assert_that!(close(&f, &p, true)).is_ok();
        assert_that!(Workspace::lock(&f.ws).discard_at_quit.contains(&p)).is_false();
    });
}

#[test]
fn closing_owner_after_a_park_during_history_forwards_it() {
    within(|| {
        let (f, pin) = deck_packs();
        f.new_tab();
        let p = f.deck_pack_tab(pin.id);
        f.with("t-1", Session::history_open).unwrap_or_else(|e| panic!("history: {}", e.one_line()));
        f.save_in(&p, "Importé 2");
        assert_that!(f.unsent(&p)).is_true();
        let restored = Workspace::with_quiet(&f.ws, "t-1", Session::history_restore).0;
        assert_that!(restored.and_then(|r| r)).is_ok();
        assert_that!(kind(&close(&f, "t-1", false))).is_equal_to("Unsaved".to_owned());
        assert_that!((f.unsent(&p), f.pinned("t-1", pin.id) != Some(pin.version.clone()))).is_equal_to((false, true));
        assert_that!(kind(&close(&f, "t-1", true))).is_equal_to("Applied".to_owned());
        assert_that!((has(&f, "t-1"), has(&f, &p))).is_equal_to((false, false));
    });
}

#[test]
fn ensure_closable_refuses_an_unsent_deck_pack_tab() {
    within(|| {
        let (f, pin) = deck_packs();
        let p = f.deck_pack_tab(pin.id);
        let ns = f.acme_ns.clone();
        f.with("t-1", |s| {
            let other = s.user_deck().widgets[&ns].version.clone();
            s.deck_links.get_mut(&p).unwrap_or_else(|| panic!("a link")).expect = other;
        });
        f.save_in(&p, "Importé 2");
        assert_that!(f.unsent(&p)).is_true();
        let refused = Workspace::lock(&f.ws).ensure_closable(&p, false).map_err(|e| e.kind());
        assert_that!(refused).is_equal_to(Err(SessionErrorKind::Unsaved));
    });
}
