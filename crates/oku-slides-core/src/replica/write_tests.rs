//! `sync_text` through each of its tiers: per character, per line, whole text.

use loro::LoroDoc;
use rstest::rstest;
use speculoos::prelude::*;

use super::{CHAR_DIFF_MAX_CHARS, DIFF_TIMEOUT_MS, sync_text};

/// A budget no diff reaches: the tier is chosen by size alone.
const UNBOUNDED: f64 = f64::INFINITY;

fn ok<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e:?}"))
}

/// A doc whose text `t` holds `text`, committed.
fn doc_with(text: &str) -> LoroDoc {
    let doc = LoroDoc::new();
    ok(doc.get_text("t").insert(0, text));
    doc.commit();
    doc
}

/// `lines` lines made by `line`, CRLF-terminated, with multibyte characters.
fn many_lines(lines: usize, line: impl Fn(usize) -> String) -> String {
    (0..lines).map(|i| line(i) + "\r\n").collect()
}

/// A text past [`CHAR_DIFF_MAX_CHARS`] characters: the per-line tier.
fn large(line: impl Fn(usize) -> String) -> String {
    let text = many_lines(CHAR_DIFF_MAX_CHARS / 8, line);
    assert_that!(text.chars().count()).is_greater_than(CHAR_DIFF_MAX_CHARS);
    text
}

#[rstest]
#[case::char_tier("é 😀 a\r\nb\r\n", "é 🎉 a\r\nB\r\nc😀\r\n", UNBOUNDED)]
#[case::char_tier_production_budget("x\r\n", "😀\r\ny\r\n", DIFF_TIMEOUT_MS)]
#[case::line_tier(&large(|i| format!("l{i} é😀")), &large(|i| format!("l{i} {}", if i % 7 == 0 { "🎉" } else { "é😀" })), UNBOUNDED)]
#[case::zero_budget_large(&large(|i| format!("o{i} 😀")), &large(|i| format!("n{i} é")), 0.0)]
// Below 128² compared units Loro diffs without reading the clock: a zero budget still diffs.
#[case::zero_budget_small_diff_never_times_out("😀\r\nα", "β\r\n😀", 0.0)]
// A pure delete never reads the clock either.
#[case::zero_budget_pure_delete_never_times_out("😀\r\n", "", 0.0)]
fn sync_text_any_tier_ends_equal_to_the_target(#[case] from: &str, #[case] to: &str, #[case] budget_ms: f64) {
    let doc = doc_with(from);
    let t = doc.get_text("t");
    ok(sync_text(&t, to, budget_ms));
    doc.commit();
    assert_that!(t.to_string()).is_equal_to(to.to_owned());
}

#[test]
fn sync_text_unchanged_writes_nothing() {
    let doc = doc_with("😀\r\n");
    let vv = doc.oplog_vv();
    ok(sync_text(&doc.get_text("t"), "😀\r\n", 0.0));
    doc.commit();
    assert_that!(doc.oplog_vv()).is_equal_to(vv);
}

/// `edit` applied to the text of a fork of `base`, `to` synced on `base`, then both merged.
fn merged(base: &str, to: &str, budget_ms: f64, edit: impl Fn(&loro::LoroText)) -> String {
    let a = doc_with(base);
    let b = a.fork();
    edit(&b.get_text("t"));
    b.commit();
    ok(sync_text(&a.get_text("t"), to, budget_ms));
    a.commit();
    ok(a.import(&ok(b.export(loro::ExportMode::all_updates()))));
    a.get_text("t").to_string()
}

#[test]
fn sync_text_char_tier_keeps_a_concurrent_edit_in_the_same_line() {
    let out = merged("alpha 😀 omega\r\n", "ALPHA 😀 omega\r\n", UNBOUNDED, |t| {
        ok(t.insert(t.len_unicode() - 2, "!"));
    });
    assert_that!(out).is_equal_to("ALPHA 😀 omega!\r\n".to_owned());
}

#[test]
fn sync_text_line_tier_keeps_a_concurrent_edit_on_another_line() {
    let base = large(|i| format!("line {i} é😀"));
    let to = base.replacen("line 10 é😀\r\n", "line ten 🎉\r\n", 1);
    let out = merged(&base, &to, UNBOUNDED, |t| {
        ok(t.insert(t.len_unicode(), "last 😀\r\n"));
    });
    assert_that!(out).is_equal_to(format!("{to}last 😀\r\n"));
}

/// Atom ops written by syncing a large text to the same text with every other line changed.
fn ops_for_every_other_line(budget_ms: f64) -> (usize, usize) {
    let base = large(|i| format!("l{i} é😀"));
    let to = large(|i| if i % 2 == 0 { format!("n{i} 🎉") } else { format!("l{i} é😀") });
    let doc = doc_with(&base);
    let before = doc.len_ops();
    ok(sync_text(&doc.get_text("t"), &to, budget_ms));
    doc.commit();
    assert_that!(doc.get_text("t").to_string()).is_equal_to(&to);
    (doc.len_ops() - before, base.chars().count() + to.chars().count())
}

#[test]
fn sync_text_line_tier_keeps_unchanged_lines() {
    let (written, whole) = ops_for_every_other_line(UNBOUNDED);
    assert_that!(written).is_less_than(whole);
}

/// The splice tier ran: every old char is deleted and every new one inserted.
///
/// The line diff here compares ~8 Ki lines with a change every other line, millions of steps;
/// Loro gives up once its millisecond clock (`get_sys_timestamp`) has ticked (`> 0.0`).
#[test]
fn sync_text_zero_budget_splices_the_whole_text() {
    let (written, whole) = ops_for_every_other_line(0.0);
    assert_that!(written).is_equal_to(whole);
}
