//! The knowledge bundle in the replica: round trip, format 8, merges and hostile entries.

use std::sync::Arc;

use loro::{LoroMap, LoroText, LoroValue, ValueOrContainer};
use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

use super::{DeckReplica, KNOWLEDGE, ROOT, compact};
use crate::error::{FrontmatterReason, PathReason, ReplicaErrorKind};
use crate::knowledge::limits::{MAX_FILE_BYTES, MAX_FILES, READ_FACTOR};
use crate::knowledge::{DropCause, KnowledgeFile};
use crate::{Deck, Issue, Op, WidgetRegistry, apply_all};

const NOTE: &str = "---\ntype: Note\n---\nalpha middle omega\n";

fn ok<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e:?}"))
}

fn registry() -> Arc<WidgetRegistry> {
    crate::test_registry::shared()
}

fn roadmap() -> Deck {
    ok(Deck::from_json(include_str!("../../../../fixtures/roadmap.json")))
}

fn id(n: u8) -> String {
    format!("{n:032x}")
}

fn write(path: &str, text: &str, id: Option<String>) -> Op {
    Op::KnowledgeWrite { path: path.to_owned(), text: text.to_owned(), id }
}

/// `ops` applied to a copy of `deck`.
fn with_ops(deck: &Deck, ops: Vec<Op>) -> Deck {
    let mut d = deck.clone();
    ok(apply_all(&mut d, ops, &crate::test_registry::registry()));
    d
}

/// `ops` applied to the deck `r` reads, then written to `r`.
fn edit(r: &mut DeckReplica, ops: Vec<Op>) {
    let deck = with_ops(&r.deck().0, ops);
    ok(r.apply(&deck, "edit", None));
}

/// Two peers `a` and `b` opened from one snapshot of `deck`, their text diffs unbounded in
/// time so a merge granularity assertion never depends on the machine's speed.
fn peers(deck: &Deck, a: u64, b: u64) -> (DeckReplica, DeckReplica) {
    let snap = ok(ok(DeckReplica::new(deck, registry())).snapshot());
    let open = |peer: u64| {
        let mut r = ok(DeckReplica::from_snapshot(&snap, 1 << 24, registry())).0;
        ok(r.doc.set_peer_id(peer));
        r.diff_budget_ms = f64::INFINITY;
        r
    };
    (open(a), open(b))
}

fn reopen(r: &DeckReplica) -> DeckReplica {
    ok(DeckReplica::from_snapshot(&ok(r.snapshot()), 1 << 24, registry())).0
}

fn paths(deck: &Deck) -> Vec<String> {
    let mut out: Vec<String> = deck.knowledge.values().map(|f| f.path.clone()).collect();
    out.sort();
    out
}

fn text_at<'a>(deck: &'a Deck, path: &str) -> Option<&'a str> {
    deck.knowledge.values().find(|f| f.path == path).map(|f| f.text.as_str())
}

fn keys(issues: &[Issue]) -> Vec<&'static str> {
    issues.iter().map(|i| i.message.id()).collect()
}

/// The deck both peers read after `ops_a` on `a` and `ops_b` on `b`, merged in both
/// directions from the same start; asserts the four reads are equal.
fn converge(base: &Deck, ops_a: &[Op], ops_b: &[Op]) -> Deck {
    let run = |a_first: bool| {
        let (mut a, mut b) = peers(base, 1, 2);
        edit(&mut a, ops_a.to_vec());
        edit(&mut b, ops_b.to_vec());
        if a_first {
            ok(a.merge(&b));
            ok(b.merge(&a));
        } else {
            ok(b.merge(&a));
            ok(a.merge(&b));
        }
        let (da, db) = (a.deck().0, b.deck().0);
        assert_that!(da).is_equal_to(&db);
        da
    };
    let first = run(true);
    assert_that!(run(false)).is_equal_to(&first);
    first
}

/// `roadmap` with `/a.md` (`NOTE`) under `id(1)`.
fn with_note() -> Deck {
    with_ops(&roadmap(), vec![write("/a.md", NOTE, Some(id(1)))])
}

/// Writes a raw entry `id → { path, text }` into the root map `knowledge` of `r`.
fn raw_entry(r: &DeckReplica, id: &str, path: &str, text: &str) {
    let entry = ok(r.doc.get_map(KNOWLEDGE).insert_container(id, LoroMap::new()));
    ok(entry.insert("path", path));
    let t = ok(entry.insert_container("text", LoroText::new()));
    ok(t.insert(0, text));
    r.doc.commit();
}

#[test]
fn knowledge_round_trip_is_byte_identical() {
    let crlf = "---\r\ntype: Note\r\n---\r\ntrailing  \r\n\r\n";
    let deck = with_ops(&roadmap(), vec![write("/a.md", crlf, Some(id(1))), write("/log.md", "", Some(id(2)))]);
    let r = ok(DeckReplica::new(&deck, registry()));
    let (read, issues) = reopen(&r).deck();
    assert_that!(issues).is_empty();
    assert_that!(read.knowledge).is_equal_to(&deck.knowledge);
    assert_that!(text_at(&read, "/a.md")).is_equal_to(Some(crlf));
    assert_that!(text_at(&read, "/log.md")).is_equal_to(Some(""));
}

#[test]
fn knowledge_unchanged_apply_writes_nothing() {
    let deck = with_note();
    let mut r = ok(DeckReplica::new(&deck, registry()));
    let vv = r.doc.oplog_vv();
    ok(r.apply(&deck, "same", None));
    assert_that!(r.doc.oplog_vv()).is_equal_to(vv);
}

#[test]
fn knowledge_lives_in_its_root_never_under_deck() {
    let r = ok(DeckReplica::new(&with_note(), registry()));
    assert_that!(r.doc.get_map(ROOT).get(KNOWLEDGE).is_none()).is_true();
    assert_that!(r.doc.get_map(KNOWLEDGE).len()).is_equal_to(1);
}

#[test]
fn knowledge_format_7_snapshot_opens_empty_and_saves_as_8() {
    let r = ok(DeckReplica::new(&roadmap(), registry()));
    ok(r.doc.get_map("meta").insert("format", 7));
    r.doc.set_next_commit_message(compact::COMPACT);
    r.doc.commit();
    let versions = r.history().len();
    let (opened, (_, issues)) = ok(DeckReplica::from_snapshot(&ok(r.snapshot()), 1 << 24, registry()));
    assert_that!(issues).is_empty();
    assert_that!(opened.deck().0.knowledge.is_empty()).is_true();
    assert_that!(opened.history().len()).is_equal_to(versions);
    assert_that!(opened.can_undo()).is_false();
    let saved = reopen(&opened);
    let format = match saved.doc.get_map("meta").get("format") {
        Some(ValueOrContainer::Value(v)) => Some(v),
        _ => None,
    };
    assert_that!(format).is_equal_to(Some(LoroValue::I64(8)));
}

#[test]
fn knowledge_format_9_snapshot_is_refused() {
    let r = ok(DeckReplica::new(&roadmap(), registry()));
    ok(r.doc.get_map("meta").insert("format", 9));
    r.doc.commit();
    let kind = DeckReplica::from_snapshot(&ok(r.snapshot()), 1 << 24, registry()).map(|_| ()).map_err(|e| e.kind());
    assert_that!(kind).is_equal_to(Err(ReplicaErrorKind::UnsupportedVersion));
}

#[test]
fn knowledge_concurrent_edits_at_two_positions_are_both_kept() {
    let edit = |old: &str, new: &str| Op::KnowledgeEdit { path: "/a.md".into(), old: old.into(), new: new.into() };
    let merged = converge(&with_note(), &[edit("alpha", "ALPHA")], &[edit("omega", "OMEGA")]);
    assert_that!(text_at(&merged, "/a.md")).is_equal_to(Some("---\ntype: Note\n---\nALPHA middle OMEGA\n"));
}

#[test]
fn knowledge_delete_against_edit_deletes() {
    let edit = Op::KnowledgeEdit { path: "/a.md".into(), old: "omega".into(), new: "OMEGA".into() };
    let merged = converge(&with_note(), &[Op::KnowledgeDelete { path: "/a.md".into() }], &[edit]);
    assert_that!(merged.knowledge.is_empty()).is_true();
}

#[test]
fn knowledge_move_against_edit_keeps_both() {
    let edit = Op::KnowledgeEdit { path: "/a.md".into(), old: "omega".into(), new: "OMEGA".into() };
    let moved = Op::KnowledgeMove { from: "/a.md".into(), to: "/b/c.md".into() };
    let merged = converge(&with_note(), &[moved], &[edit]);
    assert_that!(paths(&merged)).is_equal_to(vec!["/b/c.md".to_owned()]);
    assert_that!(text_at(&merged, "/b/c.md")).is_equal_to(Some("---\ntype: Note\n---\nalpha middle OMEGA\n"));
}

#[test]
fn knowledge_write_against_edit_merges_both() {
    let edit = Op::KnowledgeEdit { path: "/a.md".into(), old: "alpha".into(), new: "ALPHA".into() };
    let replaced = "---\ntype: Note\n---\nalpha middle omega\nmore\n";
    let merged = converge(&with_note(), &[write("/a.md", replaced, None)], &[edit]);
    assert_that!(text_at(&merged, "/a.md")).is_equal_to(Some("---\ntype: Note\n---\nALPHA middle omega\nmore\n"));
}

#[test]
fn knowledge_three_peers_creating_one_path_are_renamed_then_persisted() {
    let snap = ok(ok(DeckReplica::new(&roadmap(), registry())).snapshot());
    let mut all: Vec<DeckReplica> = (1..=3u8)
        .map(|n| {
            let mut r = ok(DeckReplica::from_snapshot(&snap, 1 << 24, registry())).0;
            ok(r.doc.set_peer_id(u64::from(n)));
            edit(&mut r, vec![write("/a.md", NOTE, Some(id(n)))]);
            r
        })
        .collect();
    for _ in 0..2 {
        for a in 0..3 {
            for b in 0..3 {
                if a != b {
                    let other = reopen(&all[b]);
                    ok(all[a].merge(&other));
                }
            }
        }
    }
    let (read, issues) = all[0].deck();
    let expected: Vec<(String, String)> =
        vec![(id(1), "/a.md".into()), (id(2), "/a-2.md".into()), (id(3), "/a-3.md".into())];
    let got: Vec<(String, String)> = read.knowledge.iter().map(|(k, f)| (k.clone(), f.path.clone())).collect();
    assert_that!(got).is_equal_to(expected);
    assert_that!(keys(&issues)).is_equal_to(vec!["issue-knowledge-path-collision"; 2]);
    for r in &all[1..] {
        assert_that!(r.deck().0).is_equal_to(&read);
    }
    // The first apply persists the repair; the next one writes nothing.
    let r = &mut all[0];
    ok(r.apply(&read, "repair", None));
    let (again, issues) = r.deck();
    assert_that!(issues).is_empty();
    assert_that!(again).is_equal_to(&read);
    let vv = r.doc.oplog_vv();
    ok(r.apply(&again, "again", None));
    assert_that!(r.doc.oplog_vv()).is_equal_to(vv);
}

#[test]
fn knowledge_hostile_raw_entries_are_repaired_on_read() {
    let r = ok(DeckReplica::new(&roadmap(), registry()));
    let long = format!("/{}.md", "a".repeat(10_000));
    raw_entry(&r, &id(1), "/../x.md", NOTE);
    raw_entry(&r, &id(2), "/con.md", NOTE);
    raw_entry(&r, &id(3), &long, NOTE);
    raw_entry(&r, "XYZ", "/bad.md", NOTE);
    raw_entry(&r, &id(4), "/plain.md", "no frontmatter");
    ok(r.doc.get_map(KNOWLEDGE).insert(&id(5), "not a map"));
    r.doc.commit();
    let (read, issues) = reopen(&r).deck();
    let got: Vec<(String, String)> = read.knowledge.iter().map(|(k, f)| (k.clone(), f.path.clone())).collect();
    assert_that!(got).is_equal_to(vec![
        (id(1), format!("/recovered/{}.md", id(1))),
        (id(2), format!("/recovered/{}.md", id(2))),
        (id(3), format!("/recovered/{}.md", id(3))),
        (id(4), "/plain.md".to_owned()),
    ]);
    assert_that!(read.knowledge.get(&id(4)))
        .is_equal_to(Some(&KnowledgeFile { path: "/plain.md".into(), text: "no frontmatter".into() }));
    assert_that!(issues).contains(Issue::knowledge_unreadable(&id(5)));
    assert_that!(issues).contains(Issue::knowledge_unreadable("XYZ"));
    assert_that!(issues).contains(Issue::knowledge_path_invalid(&id(1), PathReason::DotSegment));
    assert_that!(issues).contains(Issue::knowledge_path_invalid(&id(2), PathReason::Reserved));
    assert_that!(issues).contains(Issue::knowledge_path_invalid(&id(3), PathReason::TooLong));
    assert_that!(issues).contains(Issue::knowledge_invalid("/plain.md", &FrontmatterReason::Missing));
    assert_that!(issues.len()).is_equal_to(6);
}

#[test]
fn knowledge_root_written_as_a_list_reads_empty_with_an_issue() {
    let doc = loro::LoroDoc::new();
    ok(doc.get_list(KNOWLEDGE).insert(0, "hostile"));
    doc.commit();
    let (deck, issues) = ok(super::read_deck(&doc));
    assert_that!(deck.knowledge.is_empty()).is_true();
    assert_that!(issues).contains(Issue::knowledge_unreadable(KNOWLEDGE));
}

#[test]
fn knowledge_undo_restores_the_text() {
    let mut r = ok(DeckReplica::new(&with_note(), registry()));
    edit(&mut r, vec![Op::KnowledgeEdit { path: "/a.md".into(), old: "omega".into(), new: "OMEGA".into() }]);
    assert_that!(ok(r.undo())).is_true();
    assert_that!(text_at(&r.deck().0, "/a.md")).is_equal_to(Some(NOTE));
}

#[test]
fn knowledge_history_and_restore_see_knowledge() {
    let mut r = ok(DeckReplica::new(&roadmap(), registry()));
    edit(&mut r, vec![write("/a.md", NOTE, Some(id(1)))]);
    let history = r.history();
    let first = ok(history.deck_at(0)).0;
    let last = ok(history.deck_at(history.len() - 1)).0;
    assert_that!(first.knowledge.is_empty()).is_true();
    assert_that!(text_at(&last, "/a.md")).is_equal_to(Some(NOTE));
    let with = r.deck().0;
    ok(r.restore(&roadmap(), "restore"));
    assert_that!(r.deck().0.knowledge.is_empty()).is_true();
    ok(r.restore(&with, "restore"));
    assert_that!(text_at(&r.deck().0, "/a.md")).is_equal_to(Some(NOTE));
}

#[test]
fn knowledge_issue_localizes_in_german() {
    let issue = Issue::knowledge_path_invalid(&id(1), PathReason::DotSegment);
    assert_that!(issue.message.id()).is_equal_to("issue-knowledge-path-invalid");
    let de = issue.localize(&Localizer::new(Some("de")));
    assert_that!(de).is_not_equal_to(issue.localize(oku_slides_i18n::english()));
    assert_that!(de.contains(&id(1))).is_true();
}

/// A note whose body is `lines` lines made by `line`.
fn big_note(lines: usize, line: impl Fn(usize) -> String) -> String {
    let mut text = String::from("---\ntype: Note\n---\n");
    for i in 0..lines {
        text.push_str(&line(i));
    }
    text
}

#[test]
fn knowledge_large_file_rewritten_whole_ends_equal_to_the_target() {
    // 512 KiB of lines replaced by 512 KiB of entirely different lines.
    let old = big_note(1 << 17, |i| format!("o{:02}\n", i % 100));
    let new = big_note(1 << 17, |i| format!("n{:02}\n", i % 100));
    assert_that!(new.len()).is_greater_than(512 * 1024);
    let mut r = ok(DeckReplica::new(&with_ops(&roadmap(), vec![write("/a.md", &old, Some(id(1)))]), registry()));
    edit(&mut r, vec![write("/a.md", &new, None)]);
    assert_that!(text_at(&reopen(&r).deck().0, "/a.md")).is_equal_to(Some(new.as_str()));
}

#[test]
fn knowledge_large_file_small_rewrite_keeps_a_concurrent_edit() {
    let old = big_note(20_000, |i| format!("line {i}\n"));
    let base = with_ops(&roadmap(), vec![write("/a.md", &old, Some(id(1)))]);
    let rewritten = old.replace("line 10\n", "line ten\n");
    let concurrent = Op::KnowledgeEdit { path: "/a.md".into(), old: "line 19999\n".into(), new: "last\n".into() };
    let merged = converge(&base, &[write("/a.md", &rewritten, None)], &[concurrent]);
    let expected = rewritten.replace("line 19999\n", "last\n");
    assert_that!(text_at(&merged, "/a.md")).is_equal_to(Some(expected.as_str()));
}

#[test]
fn knowledge_undo_of_a_delete_restores_the_text_byte_identical() {
    let crlf = "---\r\ntype: Note\r\n---\r\ntrailing  \r\n";
    let mut r = ok(DeckReplica::new(&with_ops(&roadmap(), vec![write("/a.md", crlf, Some(id(1)))]), registry()));
    edit(&mut r, vec![Op::KnowledgeDelete { path: "/a.md".into() }]);
    assert_that!(r.deck().0.knowledge.is_empty()).is_true();
    assert_that!(ok(r.undo())).is_true();
    let read = r.deck().0;
    assert_that!(read.knowledge.get(&id(1)))
        .is_equal_to(Some(&KnowledgeFile { path: "/a.md".into(), text: crlf.into() }));
}

#[test]
fn knowledge_raw_key_under_deck_is_removed_on_next_apply() {
    let mut r = ok(DeckReplica::new(&with_note(), registry()));
    ok(r.doc.get_map(ROOT).insert(KNOWLEDGE, "hostile"));
    r.doc.commit();
    let (deck, issues) = r.deck();
    assert_that!(keys(&issues)).is_equal_to(vec!["issue-unreadable-field"]);
    ok(r.apply(&deck, "repair", None));
    assert_that!(r.doc.get_map(ROOT).get(KNOWLEDGE).is_none()).is_true();
    let (again, issues) = r.deck();
    assert_that!(issues).is_empty();
    assert_that!(again.knowledge).is_equal_to(&with_note().knowledge);
}

/// The `meta.format` value `r` reads.
fn format_of(r: &DeckReplica) -> Option<LoroValue> {
    match r.doc.get_map("meta").get("format") {
        Some(ValueOrContainer::Value(v)) => Some(v),
        _ => None,
    }
}

#[test]
fn knowledge_merge_of_a_format_7_peer_keeps_format_8_and_knowledge() {
    let (mut a, b) = peers(&with_note(), 1, 2);
    // A local edit the undo must still find after the merge.
    let mut titled = with_note();
    titled.params.title = Some("local".into());
    ok(a.apply(&titled, "title", None));
    // The format-7 peer writes after the base, so its value wins the register.
    ok(b.doc.get_map("meta").insert("format", 7));
    b.doc.commit();
    let versions = a.history().len();
    let report = ok(a.merge(&b));
    assert_that!(format_of(&a)).is_equal_to(Some(LoroValue::I64(8)));
    assert_that!(report.changes).is_equal_to(1);
    assert_that!(a.deck().0.knowledge).is_equal_to(&with_note().knowledge);
    assert_that!(format_of(&reopen(&a))).is_equal_to(Some(LoroValue::I64(8)));
    // The re-assert is silent: the history gains only the peer's change, and undo still
    // reverts the local edit.
    assert_that!(a.history().len()).is_equal_to(versions + 1);
    assert_that!(ok(a.undo())).is_true();
    assert_that!(a.deck().0).is_equal_to(with_note());
    assert_that!(format_of(&a)).is_equal_to(Some(LoroValue::I64(8)));
}

#[test]
fn knowledge_merge_past_the_file_ceiling_keeps_every_file_after_an_unrelated_op() {
    let half = crate::knowledge::limits::MAX_FILES / 2 + 1;
    let (mut a, b) = peers(&roadmap(), 1, 2);
    for n in 0..half {
        raw_entry(&a, &format!("a{n:031x}"), &format!("/a/{n}.md"), NOTE);
        raw_entry(&b, &format!("b{n:031x}"), &format!("/b/{n}.md"), NOTE);
    }
    ok(a.merge(&b));
    let (read, issues) = a.deck();
    assert_that!(read.knowledge.len()).is_equal_to(2 * half);
    assert_that!(keys(&issues)).is_equal_to(vec!["issue-knowledge-over-limit"]);
    let mut titled = read.clone();
    titled.params.title = Some("unrelated".into());
    ok(a.apply(&titled, "title", None));
    let (after, _) = reopen(&a).deck();
    assert_that!(after.knowledge).is_equal_to(&read.knowledge);
    assert_that!(after.params.title.as_deref()).is_equal_to(Some("unrelated"));
}

/// `count` raw entries `<n as 32 hex> → { /f/<n>.md, NOTE }` in one commit.
fn raw_entries(r: &DeckReplica, count: usize) {
    let map = r.doc.get_map(KNOWLEDGE);
    for n in 0..count {
        let entry = ok(map.insert_container(&format!("{n:032x}"), LoroMap::new()));
        ok(entry.insert("path", format!("/f/{n}.md").as_str()));
        let t = ok(entry.insert_container("text", LoroText::new()));
        ok(t.insert(0, NOTE));
    }
    r.doc.commit();
}

fn count_of(issues: &[Issue], key: &str) -> usize {
    issues.iter().filter(|i| i.message.id() == key).count()
}

#[test]
fn knowledge_read_past_the_file_read_ceiling_drops_the_rest_with_a_count() {
    let r = ok(DeckReplica::new(&roadmap(), registry()));
    raw_entries(&r, READ_FACTOR * MAX_FILES + 3);
    let (read, issues) = reopen(&r).deck();
    assert_that!(read.knowledge.len()).is_equal_to(READ_FACTOR * MAX_FILES);
    assert_that!(issues).contains(Issue::knowledge_entries_dropped(DropCause::Files, 3));
    assert_that!(count_of(&issues, "issue-knowledge-entries-dropped")).is_equal_to(1);
}

#[test]
fn knowledge_read_of_a_text_past_the_file_byte_read_ceiling_drops_it() {
    let r = ok(DeckReplica::new(&roadmap(), registry()));
    raw_entry(&r, &id(1), "/a.md", NOTE);
    raw_entry(&r, &id(2), "/huge.md", &"x".repeat(READ_FACTOR * MAX_FILE_BYTES + 1));
    let (read, issues) = r.deck();
    assert_that!(read.knowledge.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec![id(1)]);
    assert_that!(issues).is_equal_to(vec![Issue::knowledge_entries_dropped(DropCause::FileBytes, 1)]);
}

#[test]
fn knowledge_read_reports_32_unreadable_entries_then_a_count() {
    let r = ok(DeckReplica::new(&roadmap(), registry()));
    let map = r.doc.get_map(KNOWLEDGE);
    for n in 0..40u8 {
        ok(map.insert(&id(n), "not a map"));
    }
    r.doc.commit();
    let (read, issues) = reopen(&r).deck();
    assert_that!(read.knowledge.is_empty()).is_true();
    assert_that!(count_of(&issues, "issue-knowledge-unreadable")).is_equal_to(32);
    assert_that!(issues).contains(Issue::knowledge_entries_dropped(DropCause::Unreadable, 8));
    assert_that!(issues.len()).is_equal_to(33);
}

#[test]
fn knowledge_read_of_twice_the_file_limit_keeps_every_file() {
    let r = ok(DeckReplica::new(&roadmap(), registry()));
    raw_entries(&r, 2 * MAX_FILES);
    let (read, issues) = reopen(&r).deck();
    assert_that!(read.knowledge.len()).is_equal_to(2 * MAX_FILES);
    assert_that!(keys(&issues)).is_equal_to(vec!["issue-knowledge-over-limit"]);
}
