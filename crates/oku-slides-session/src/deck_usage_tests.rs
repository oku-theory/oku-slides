use speculoos::prelude::*;
use time::macros::datetime;
use uuid::Uuid;

use super::*;
use crate::test_support::{fresh_dir, logged, rename_title, session, session_with, settings_on, touch_elsewhere};

/// A valid usage record of turn `n` (core's own helper is `cfg(test)` of core).
pub(crate) fn record(n: u128) -> UsageRecord {
    UsageRecord {
        turn: Uuid::from_u128(n),
        at: datetime!(2026-10-30 14:02:11 UTC),
        tab: oku_slides_core::UsageTab::Deck,
        agent: "claude".into(),
        model: Some("claude-opus-5-5".into()),
        tokens: Some(oku_slides_core::UsageTokens {
            total: 110,
            input: 100,
            output: 10,
            thought: None,
            cache_read: Some(5),
            cache_write: None,
        }),
        cost: Some(oku_slides_core::UsageCost { amount: 0.04, currency: "USD".into() }),
        extra: serde_json::Map::new(),
    }
}

/// The ledger of the file at `path`, as a reader sees it.
pub(crate) fn ledger_in(path: &std::path::Path) -> UsageLedger {
    let s = session();
    crate::read_document(&s.storage, path, s.source_limits, &s.registry)
        .unwrap_or_else(|e| panic!("{}: {}", path.display(), e.one_line()))
        .sources
        .usage
}

/// `s` saved as `name` in a fresh folder: the folder (kept alive) and the file.
fn saved(s: &mut Session, name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = fresh_dir("oku-slides-usage-");
    let path = dir.path().join(format!("{name}{}", crate::EXTENSION));
    s.save(Some(&path)).unwrap_or_else(|e| panic!("save: {}", e.one_line()));
    (dir, path)
}

#[test]
fn record_usage_in_deck_mode_adds_without_dirtying() {
    let mut s = session();
    assert_that!(s.record_usage(record(1))).is_true();
    assert_that!(s.usage_ledger().map(UsageLedger::len)).is_equal_to(Some(1));
    assert_that!(s.unsaved()).is_false();
    assert_that!(s.doc.usage_unsaved).is_true();
}

#[test]
fn record_usage_same_turn_twice_adds_once() {
    let mut s = session();
    s.record_usage(record(1));
    assert_that!(s.record_usage(record(1))).is_false();
    assert_that!(s.usage_ledger().map(UsageLedger::len)).is_equal_to(Some(1));
}

#[test]
fn record_usage_on_a_full_ledger_refuses_and_warns_once() {
    let mut s = session();
    let max = u128::try_from(oku_slides_core::USAGE_MAX_RECORDS).unwrap();
    for n in 0..max {
        s.record_usage(record(n));
    }
    assert_that!(s.record_usage(record(max))).is_false();
    assert_that!(s.doc.usage_full_warned).is_true();
    // A second refusal: still refused, the flag kept, nothing added.
    assert_that!(s.record_usage(record(max + 1))).is_false();
    assert_that!(s.doc.usage_full_warned).is_true();
    assert_that!(s.usage_ledger().map(UsageLedger::len)).is_equal_to(Some(oku_slides_core::USAGE_MAX_RECORDS));
}

#[test]
fn record_usage_in_history_mode_lands_in_the_stashed_deck() {
    let mut s = session();
    s.history_open().unwrap_or_else(|e| panic!("history: {}", e.one_line()));
    assert_that!(s.record_usage(record(1))).is_true();
    assert_that!(s.usage_ledger().map(UsageLedger::len)).is_equal_to(Some(1));
    s.history_close().unwrap_or_else(|e| panic!("close history: {}", e.one_line()));
    assert_that!(s.doc.sources.usage.len()).is_equal_to(1);
}

#[test]
fn record_usage_in_help_mode_records_nothing() {
    let mut s = session();
    s.open_help("fr", None).unwrap_or_else(|e| panic!("help: {}", e.one_line()));
    assert_that!(s.record_usage(record(1))).is_false();
    assert_that!(s.usage_ledger()).is_none();
}

#[test]
fn record_usage_writes_the_resume_ledger_at_once() {
    let dir = fresh_dir("oku-slides-usage-resume-");
    let mut s = session();
    assert_that!(s.resume(dir.path().to_owned())).is_true();
    s.record_usage(record(1));
    s.flush_resume();
    let bytes = std::fs::read(dir.path().join("deck").join(oku_slides_core::USAGE_FILE)).unwrap();
    assert_that!(UsageLedger::from_jsonl(&bytes).0.len()).is_equal_to(1);
}

#[test]
fn record_usage_survives_a_resume() {
    let dir = fresh_dir("oku-slides-usage-resume-");
    let mut s = session();
    assert_that!(s.resume(dir.path().to_owned())).is_true();
    s.record_usage(record(1));
    s.flush_resume();
    drop(s);
    let mut back = session();
    assert_that!(back.resume(dir.path().to_owned())).is_true();
    assert_that!(back.usage_ledger().map(UsageLedger::len)).is_equal_to(Some(1));
    assert_that!(back.doc.usage_unsaved).is_true();
    assert_that!(back.unsaved()).is_false();
}

#[test]
fn record_usage_in_history_mode_survives_a_resume() {
    let dir = fresh_dir("oku-slides-usage-resume-");
    let mut s = session();
    assert_that!(s.resume(dir.path().to_owned())).is_true();
    s.history_open().unwrap_or_else(|e| panic!("history: {}", e.one_line()));
    s.record_usage(record(1));
    s.flush_resume();
    drop(s);
    let mut back = session();
    assert_that!(back.resume(dir.path().to_owned())).is_true();
    assert_that!(back.usage_ledger().map(UsageLedger::len)).is_equal_to(Some(1));
}

#[test]
fn resume_of_a_file_changed_on_disk_keeps_the_unsaved_records() {
    let dir = fresh_dir("oku-slides-usage-resume-");
    let mut s = session();
    let (_folder, path) = saved(&mut s, "union");
    assert_that!(s.resume(dir.path().to_owned())).is_true();
    s.record_usage(record(1));
    s.flush_resume();
    drop(s);
    // Another copy of the app writes the file: a new hash, its own turn, not ours.
    let mut other = session();
    other.open(&path).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    other.record_usage(record(2));
    other.apply("Thème", vec![crate::test_support::set_theme_op("pro")], None).unwrap();
    other.save(None).unwrap_or_else(|e| panic!("save: {}", e.one_line()));
    let mut back = session();
    assert_that!(back.resume(dir.path().to_owned())).is_true();
    let turns: Vec<Uuid> = back.usage_ledger().into_iter().flat_map(UsageLedger::records).map(|r| r.turn).collect();
    assert_that!(turns).is_equal_to(vec![Uuid::from_u128(1), Uuid::from_u128(2)]);
    assert_that!(back.doc.usage_unsaved).is_true();
}

#[test]
fn resume_of_a_file_holding_every_record_is_not_flagged_unsaved() {
    let dir = fresh_dir("oku-slides-usage-resume-");
    let mut s = session();
    let (_folder, path) = saved(&mut s, "held");
    assert_that!(s.resume(dir.path().to_owned())).is_true();
    s.record_usage(record(1));
    s.flush_resume();
    drop(s);
    // Another copy saved the same turn, and a change of its own.
    let mut other = session();
    other.open(&path).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    other.record_usage(record(1));
    other.apply("Thème", vec![crate::test_support::set_theme_op("pro")], None).unwrap();
    other.save(None).unwrap_or_else(|e| panic!("save: {}", e.one_line()));
    let mut back = session();
    assert_that!(back.resume(dir.path().to_owned())).is_true();
    assert_that!(back.usage_ledger().map(UsageLedger::len)).is_equal_to(Some(1));
    assert_that!(back.doc.usage_unsaved).is_false();
}

#[test]
fn save_writes_the_ledger_and_clears_usage_unsaved() {
    let mut s = session();
    s.record_usage(record(1));
    let (_folder, path) = saved(&mut s, "saved");
    assert_that!(s.doc.usage_unsaved).is_false();
    assert_that!(ledger_in(&path).len()).is_equal_to(1);
}

/// `record(n)` without model nor cost: a shorter line for the same turn.
fn short_record(n: u128) -> UsageRecord {
    UsageRecord { model: None, cost: None, ..record(n) }
}

#[test]
fn record_usage_forged_shorter_record_replaces_and_flags_unsaved() {
    let mut s = session();
    s.record_usage(record(1));
    s.doc.usage_unsaved = false;
    assert_that!(s.record_usage(short_record(1))).is_false();
    assert_that!(s.doc.usage_unsaved).is_true();
    let held: Vec<_> = s.usage_ledger().into_iter().flat_map(UsageLedger::records).cloned().collect();
    assert_that!(held).is_equal_to(vec![short_record(1)]);
}

#[test]
fn record_usage_forged_longer_record_keeps_the_held_one_unflagged() {
    let mut s = session();
    s.record_usage(short_record(1));
    s.doc.usage_unsaved = false;
    assert_that!(s.record_usage(record(1))).is_false();
    assert_that!(s.doc.usage_unsaved).is_false();
    let held: Vec<_> = s.usage_ledger().into_iter().flat_map(UsageLedger::records).cloned().collect();
    assert_that!(held).is_equal_to(vec![short_record(1)]);
}

/// A session whose `[usage] store_in_deck` is `store`, on the roadmap fixture.
fn storing(store: bool) -> (tempfile::TempDir, Session) {
    let data = fresh_dir("oku-slides-usage-setting-");
    let mut settings = settings_on(data.path());
    settings.usage.store_in_deck = store;
    let mut s = session_with(settings);
    let fixture = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"));
    s.open(&fixture).unwrap_or_else(|e| panic!("fixture: {}", e.one_line()));
    (data, s)
}

#[test]
fn record_usage_with_store_in_deck_off_records_nothing() {
    let (_data, mut s) = storing(false);
    assert_that!(s.record_usage(record(1))).is_false();
    assert_that!(s.usage_ledger().map(UsageLedger::len)).is_equal_to(Some(0));
    assert_that!(s.doc.usage_unsaved).is_false();
}

#[test]
fn save_with_store_in_deck_off_writes_no_usage_entry() {
    let mut on = session();
    on.record_usage(record(1));
    let (_folder, path) = saved(&mut on, "shared");
    let (_data, mut off) = storing(false);
    off.open(&path).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    let (_copy, copy) = saved(&mut off, "copy");
    assert_that!(ledger_in(&copy).is_empty()).is_true();
}

#[test]
fn save_pending_usage_clean_deck_writes_its_file() {
    let mut s = session();
    let (_folder, path) = saved(&mut s, "pending");
    s.record_usage(record(1));
    s.save_pending_usage();
    assert_that!(ledger_in(&path).len()).is_equal_to(1);
    assert_that!(s.doc.usage_unsaved).is_false();
    assert_that!(s.doc.saved_hash.clone()).is_equal_to(Some(oku_slides_core::asset_id(&std::fs::read(&path).unwrap())));
}

#[test]
fn save_pending_usage_dirty_deck_writes_nothing() {
    let mut s = session();
    let (_folder, path) = saved(&mut s, "dirty");
    s.record_usage(record(1));
    s.apply("x", vec![rename_title("Changé")], None).unwrap_or_else(|e| panic!("edit: {}", e.one_line()));
    let before = std::fs::read(&path).unwrap();
    s.save_pending_usage();
    assert_that!(std::fs::read(&path).unwrap()).is_equal_to(before);
    assert_that!(s.doc.usage_unsaved).is_true();
}

#[test]
fn save_pending_usage_file_changed_on_disk_writes_nothing() {
    let mut s = session();
    let (_folder, path) = saved(&mut s, "changed");
    s.record_usage(record(1));
    touch_elsewhere(&path);
    let before = std::fs::read(&path).unwrap();
    s.save_pending_usage();
    assert_that!(std::fs::read(&path).unwrap()).is_equal_to(before);
    assert_that!(s.doc.usage_unsaved).is_true();
}

#[test]
fn save_pending_usage_store_in_deck_turned_off_leaves_the_file_bytes() {
    let mut s = session();
    s.record_usage(record(1));
    let (_folder, path) = saved(&mut s, "turned-off");
    s.record_usage(record(2));
    s.doc_defaults.store_usage = false;
    let before = std::fs::read(&path).unwrap();
    s.save_pending_usage();
    assert_that!(std::fs::read(&path).unwrap()).is_equal_to(before);
    assert_that!(ledger_in(&path).len()).is_equal_to(1);
    assert_that!(s.doc.usage_unsaved).is_false();
}

#[test]
fn save_pending_usage_deleted_file_is_not_recreated() {
    let mut s = session();
    let (_folder, path) = saved(&mut s, "deleted");
    s.record_usage(record(1));
    std::fs::remove_file(&path).unwrap();
    let log = logged(|| s.save_pending_usage());
    assert_that!(path.exists()).is_false();
    assert_that!(s.doc.usage_unsaved).is_true();
    assert_that!(log).contains("agent usage dropped: the file was deleted");
}

#[test]
fn save_pending_usage_deck_never_saved_as_pdf_warns_the_drop() {
    let mut s = session();
    s.record_usage(record(1));
    let log = logged(|| s.save_pending_usage());
    assert_that!(log).contains("agent usage dropped");
    assert_that!(s.doc.usage_unsaved).is_true();
}

#[test]
fn history_open_with_unsaved_usage_counts_as_unsaved() {
    let mut s = session();
    let (_folder, _path) = saved(&mut s, "history-unsaved");
    s.history_open().unwrap_or_else(|e| panic!("history: {}", e.one_line()));
    assert_that!(s.unsaved()).is_false();
    s.record_usage(record(1));
    assert_that!(s.unsaved()).is_true();
}

#[test]
fn save_pending_usage_in_history_mode_writes_nothing_and_warns() {
    let mut s = session();
    let (_folder, path) = saved(&mut s, "history");
    s.history_open().unwrap_or_else(|e| panic!("history: {}", e.one_line()));
    s.record_usage(record(1));
    let before = std::fs::read(&path).unwrap();
    let log = logged(|| s.save_pending_usage());
    assert_that!(std::fs::read(&path).unwrap()).is_equal_to(before);
    assert_that!(log).contains("agent usage not saved: history is open");
}
