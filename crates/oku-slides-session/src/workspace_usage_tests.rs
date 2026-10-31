use std::sync::Arc;

use serde_json::json;
use speculoos::prelude::*;

use super::super::packs::workspace_packs_fixtures::{Packs, deck_packs, packs};
use crate::agent::{Sessions, TabOpen, TabSession};
use crate::deck_usage::deck_usage_tests::{ledger_in, record};
use crate::test_support::{logged, rename_title, touch_elsewhere};
use crate::theme::PAPIER_THEME;
use crate::{DocumentKind, Session, TabDocument, Workspace};

/// `tab`'s agent view of the fixture's workspace.
fn tab_session(f: &Packs, tab: &str) -> TabSession {
    let sessions: Arc<dyn Sessions> = f.ws.clone();
    TabSession::new(tab, sessions)
}

/// The number of turns `t-1`'s deck holds.
fn deck_turns(f: &Packs) -> usize {
    f.with("t-1", |s| s.usage_ledger().map_or(0, oku_slides_core::UsageLedger::len))
}

#[test]
fn usage_owner_of_a_deck_tab_is_itself() {
    let f = packs();
    assert_that!(Workspace::usage_owner(&f.ws, "t-1")).is_equal_to(Some("t-1".to_owned()));
}

#[test]
fn record_usage_from_user_opened_pack_tab_lands_in_opener_deck() {
    let f = packs();
    let pack_tab = f.pack_tab();
    tab_session(&f, &pack_tab).record_usage(record(1));
    assert_that!(deck_turns(&f)).is_equal_to(1);
    assert_that!(f.with(&pack_tab, |s| s.usage_ledger().is_none())).is_true();
}

#[test]
fn record_usage_from_agent_opened_pack_tab_lands_in_requesting_deck() {
    let f = packs();
    let tab = match f.open_by_agent("t-1", f.acme, &mut |_| true) {
        Ok(TabOpen::Opened { tab, .. }) => tab,
        other => panic!("a new pack tab: {other:?}"),
    };
    let agent_tab = tab_session(&f, &tab);
    agent_tab.record_usage(record(1));
    assert_that!(deck_turns(&f)).is_equal_to(1);
    let ledger = agent_tab.usage_ledger().unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(ledger.map(|l| l.len())).is_equal_to(Some(1));
}

#[test]
fn record_usage_from_user_opened_theme_tab_lands_in_opener_deck() {
    let f = packs();
    f.send(json!({"cmd": "open_theme", "tab": "t-1", "theme": PAPIER_THEME}));
    let theme_tab = f.active(0);
    assert_that!(f.document(&theme_tab)).is_equal_to(Some(TabDocument::Theme { theme: PAPIER_THEME }));
    tab_session(&f, &theme_tab).record_usage(record(1));
    assert_that!(deck_turns(&f)).is_equal_to(1);
}

#[test]
fn record_usage_from_help_tab_records_nothing() {
    let f = packs();
    f.send(json!({"cmd": "open_help", "window": "w-1"}));
    let help = f.active(0);
    assert_that!(f.document(&help).map(|d| d.kind())).is_equal_to(Some(DocumentKind::Help));
    assert_that!(Workspace::usage_owner(&f.ws, &help)).is_none();
    tab_session(&f, &help).record_usage(record(1));
    assert_that!(deck_turns(&f)).is_equal_to(0);
}

#[test]
fn usage_ledger_of_deck_pack_tab_reads_owner_deck() {
    let (f, pin) = deck_packs();
    let deck_pack = f.deck_pack_tab(pin.id);
    tab_session(&f, "t-1").record_usage(record(1));
    let ledger = tab_session(&f, &deck_pack).usage_ledger().unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(ledger.map(|l| l.len())).is_equal_to(Some(1));
}

#[test]
fn record_usage_from_theme_tab_opened_from_pack_tab_lands_in_deck() {
    let f = packs();
    let pack_tab = f.pack_tab();
    f.send(json!({"cmd": "open_theme", "tab": pack_tab, "theme": PAPIER_THEME}));
    let theme_tab = f.active(0);
    assert_that!(f.document(&theme_tab)).is_equal_to(Some(TabDocument::Theme { theme: PAPIER_THEME }));
    assert_that!(f.opener_of(&theme_tab)).is_equal_to(Some(pack_tab));
    tab_session(&f, &theme_tab).record_usage(record(1));
    assert_that!(deck_turns(&f)).is_equal_to(1);
}

#[test]
fn usage_owner_opener_cycle_between_non_deck_tabs_is_none() {
    let f = packs();
    let pack_tab = f.pack_tab();
    f.send(json!({"cmd": "open_theme", "tab": pack_tab, "theme": PAPIER_THEME}));
    let theme_tab = f.active(0);
    {
        let mut w = Workspace::lock(&f.ws);
        if let Some(t) = w.tabs.get_mut(&pack_tab) {
            t.opener = Some(theme_tab.clone());
        }
    }
    assert_that!(Workspace::usage_owner(&f.ws, &pack_tab)).is_none();
    assert_that!(Workspace::usage_owner(&f.ws, &theme_tab)).is_none();
}

/// The file of `t-1`.
fn deck_path(f: &Packs) -> std::path::PathBuf {
    f.with("t-1", |s| s.doc.path.clone()).unwrap_or_else(|| panic!("a file"))
}

#[test]
fn close_clean_deck_with_unsaved_usage_writes_the_file() {
    let (f, _pin) = deck_packs();
    let path = deck_path(&f);
    tab_session(&f, "t-1").record_usage(record(1));
    f.close_keeping("t-1").unwrap_or_else(|e| panic!("close: {}", e.one_line()));
    assert_that!(ledger_in(&path).len()).is_equal_to(1);
}

#[test]
fn close_with_file_changed_on_disk_leaves_the_file_alone() {
    let (f, _pin) = deck_packs();
    let path = deck_path(&f);
    tab_session(&f, "t-1").record_usage(record(1));
    touch_elsewhere(&path);
    let before = std::fs::read(&path).unwrap();
    let log = logged(|| {
        f.close_keeping("t-1").unwrap_or_else(|e| panic!("close: {}", e.one_line()));
    });
    assert_that!(std::fs::read(&path).unwrap()).is_equal_to(before);
    assert_that!(log).contains("agent usage dropped: the file changed on disk");
}

#[test]
fn close_deck_with_history_open_and_unsaved_usage_is_refused_and_keeps_it() {
    let (f, _pin) = deck_packs();
    let path = deck_path(&f);
    f.with("t-1", Session::history_open).unwrap_or_else(|e| panic!("history: {}", e.one_line()));
    tab_session(&f, "t-1").record_usage(record(1));
    let err = f.close_keeping("t-1").err().unwrap_or_else(|| panic!("the close is refused"));
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::Unsaved);
    assert_that!(deck_turns(&f)).is_equal_to(1);
    assert_that!(ledger_in(&path).is_empty()).is_true();
}

/// The UI's close path with history open: history closed first, then the close.
#[test]
fn close_after_history_close_writes_the_usage_recorded_in_history() {
    let (f, _pin) = deck_packs();
    let path = deck_path(&f);
    f.with("t-1", Session::history_open).unwrap_or_else(|e| panic!("history: {}", e.one_line()));
    tab_session(&f, "t-1").record_usage(record(1));
    f.with("t-1", Session::history_close).unwrap_or_else(|e| panic!("close history: {}", e.one_line()));
    assert_that!(f.with("t-1", |s| s.tab_info().dirty)).is_false();
    f.close_keeping("t-1").unwrap_or_else(|e| panic!("close: {}", e.one_line()));
    assert_that!(ledger_in(&path).len()).is_equal_to(1);
}

#[test]
fn close_deck_with_busy_session_skips_the_save_without_waiting() {
    let (f, _pin) = deck_packs();
    let path = deck_path(&f);
    tab_session(&f, "t-1").record_usage(record(1));
    let slot = Workspace::lock(&f.ws).session("t-1").unwrap_or_else(|e| panic!("{}", e.one_line()));
    let guard = slot.lock().unwrap();
    let log = logged(|| {
        assert_that!(f.close_keeping("t-1")).is_err();
    });
    drop(guard);
    assert_that!(log).contains("the session is busy");
    assert_that!(ledger_in(&path).is_empty()).is_true();
}

#[test]
fn close_dirty_deck_is_refused_and_writes_nothing() {
    let (f, _pin) = deck_packs();
    let path = deck_path(&f);
    tab_session(&f, "t-1").record_usage(record(1));
    f.with("t-1", |s| s.apply("x", vec![rename_title("Changé")], None))
        .unwrap_or_else(|e| panic!("edit: {}", e.one_line()));
    let before = std::fs::read(&path).unwrap();
    assert_that!(f.close_keeping("t-1")).is_err();
    assert_that!(std::fs::read(&path).unwrap()).is_equal_to(before);
}

#[test]
fn close_discarding_writes_no_usage() {
    let (f, _pin) = deck_packs();
    let path = deck_path(&f);
    tab_session(&f, "t-1").record_usage(record(1));
    f.close("t-1");
    assert_that!(ledger_in(&path).is_empty()).is_true();
}

#[test]
fn close_window_with_clean_deck_writes_its_usage() {
    let (f, _pin) = deck_packs();
    let path = deck_path(&f);
    let other = f.new_tab();
    f.send(json!({"cmd": "move_tab", "tab": other}));
    tab_session(&f, "t-1").record_usage(record(1));
    f.send(json!({"cmd": "close_window", "window": "w-1"}));
    assert_that!(ledger_in(&path).len()).is_equal_to(1);
}
