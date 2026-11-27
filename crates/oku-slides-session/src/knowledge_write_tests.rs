use oku_slides_core::knowledge::is_file_id;
use oku_slides_core::knowledge::limits::{MAX_FILE_BYTES, MAX_FRONTMATTER_BYTES};
use oku_slides_core::{KnowledgeReason, Op, asset_id};
use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;
use tempfile::TempDir;

use crate::SessionErrorKind;
use crate::knowledge_fixture::{approve_knowledge, seed_knowledge, unstamped};
use crate::test_support::{add_pack, add_version, fresh_dir, manifest_ns, opened_on, session_on};
use crate::{Session, SessionError};

const NOTE: &str = "---\ntype: Note\n---\nbody\n";
const RECEIVED: &str = "---\ntype: Note\n---\nreceived, maybe injected\n";
const MINE: &str = "---\ntype: Note\n---\nmine\n";

fn write(path: &str, text: &str) -> Op {
    Op::KnowledgeWrite { path: path.into(), text: text.into(), id: None }
}

fn edit(path: &str, old: &str, new: &str) -> Op {
    Op::KnowledgeEdit { path: path.into(), old: old.into(), new: new.into() }
}

fn moved(from: &str, to: &str) -> Op {
    Op::KnowledgeMove { from: from.into(), to: to.into() }
}

fn delete(path: &str) -> Op {
    Op::KnowledgeDelete { path: path.into() }
}

/// A session on a fresh data dir whose deck holds `files` (path, text), `approved` texts approved.
fn seeded(files: &[(&str, &str)], approved: &[&str]) -> (TempDir, Session) {
    let data = fresh_dir("oku-slides-knowledge-write-");
    approve_knowledge(data.path(), approved);
    let mut s = session_on(data.path());
    seed_knowledge(&mut s, files, Vec::new());
    (data, s)
}

/// The text at `path` in the deck shown, approved or not.
fn text_at(s: &Session, path: &str) -> Option<String> {
    s.deck().knowledge.values().find(|f| f.path == path).map(|f| f.text.clone())
}

/// The id of the file at `path`.
fn id_at(s: &Session, path: &str) -> Option<String> {
    s.deck().knowledge.iter().find(|(_, f)| f.path == path).map(|(id, _)| id.clone())
}

fn approved(s: &Session, text: &str) -> bool {
    s.library.knowledge_approvals().is_approved(&asset_id(text.as_bytes()))
}

fn knowledge_reason(err: &SessionError) -> Option<KnowledgeReason> {
    err.transaction().and_then(|t| t.op_error().knowledge_reason()).cloned()
}

#[test]
fn write_without_id_mints_hex_id_and_approves() {
    let (_data, mut s) = seeded(&[], &[]);
    s.apply("x", vec![write("/a.md", NOTE), write("/b.md", MINE)], None).unwrap();
    let (a, b) = (id_at(&s, "/a.md").unwrap(), id_at(&s, "/b.md").unwrap());
    assert_that!(is_file_id(&a)).is_true();
    assert_that!(is_file_id(&b)).is_true();
    assert_that!(a).is_not_equal_to(b);
    assert_that!(s.knowledge_file("/a.md").unwrap().text.as_str()).is_equal_to(NOTE);
    assert_that!(s.knowledge_file("/b.md").unwrap().text.as_str()).is_equal_to(MINE);
}

#[test]
fn replace_of_awaiting_file_approves_new_text() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    let id = id_at(&s, "/a.md");
    s.apply("x", vec![write("/a.md", MINE)], None).unwrap();
    assert_that!(id_at(&s, "/a.md")).is_equal_to(id);
    assert_that!(s.knowledge_file("/a.md").unwrap().text.as_str()).is_equal_to(MINE);
    assert_that!(approved(&s, RECEIVED)).is_false();
}

#[test]
fn edit_of_awaiting_file_refused_before_core() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    let before = s.deck().clone();
    // `old` matches nothing: core would refuse with EditNoMatch, the session refuses first.
    for old in ["received", "absent"] {
        let err = s.apply("x", vec![edit("/a.md", old, "mine")], None).unwrap_err();
        assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeChangeAwaiting);
        let fr = err.localize(&Localizer::new(Some("fr")));
        assert_that!(fr.as_str()).contains("/a.md");
        assert_that!(fr.as_str()).is_not_equal_to(err.localize(&Localizer::new(Some("en"))).as_str());
    }
    assert_that!(s.deck()).is_equal_to(&before);
    assert_that!(s.library.knowledge_approvals().count()).is_equal_to(0);
}

#[test]
fn move_then_edit_of_awaiting_refused() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    let ops = vec![moved("/a.md", "/b.md"), edit("/b.md", "received", "mine")];
    let err = s.apply("x", ops, None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeChangeAwaiting);
    assert_that!(text_at(&s, "/a.md").as_deref()).is_equal_to(Some(RECEIVED));
}

#[test]
fn delete_then_write_same_path_then_edit_allowed() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    let old_id = id_at(&s, "/a.md");
    let ops = vec![delete("/a.md"), write("/a.md", MINE), edit("/a.md", "mine", "mine, edited")];
    s.apply("x", ops, None).unwrap();
    let text = "---\ntype: Note\n---\nmine, edited\n";
    assert_that!(id_at(&s, "/a.md")).is_not_equal_to(old_id);
    assert_that!(s.knowledge_file("/a.md").unwrap().text.as_str()).is_equal_to(text);
    assert_that!(approved(&s, RECEIVED)).is_false();
}

#[test]
fn write_then_edit_awaiting_in_one_batch_allowed() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    s.apply("x", vec![write("/a.md", MINE), edit("/a.md", "mine", "ours")], None).unwrap();
    let text = "---\ntype: Note\n---\nours\n";
    assert_that!(s.knowledge_file("/a.md").unwrap().text.as_str()).is_equal_to(text);
    // Only the batch's result is approved, not the intermediate text.
    assert_that!(approved(&s, MINE)).is_false();
}

#[test]
fn move_of_awaiting_file_refused_and_stays_awaiting() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    let before = s.deck().clone();
    // A free target and a taken one: the session refuses before core looks at either.
    for to in ["/b.md", "/a.md"] {
        let err = s.apply("x", vec![moved("/a.md", to)], None).unwrap_err();
        assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeChangeAwaiting);
    }
    assert_that!(s.deck()).is_equal_to(&before);
    assert_that!(s.knowledge_file("/a.md").unwrap_err().kind()).is_equal_to(SessionErrorKind::KnowledgeAwaiting);
    assert_that!(s.library.knowledge_approvals().count()).is_equal_to(0);
}

#[test]
fn delete_awaiting_then_write_its_id_at_new_path_then_edit_allowed() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    let id = id_at(&s, "/a.md");
    let ops = vec![
        delete("/a.md"),
        Op::KnowledgeWrite { path: "/c.md".into(), text: MINE.into(), id: id.clone() },
        edit("/c.md", "mine", "ours"),
    ];
    s.apply("x", ops, None).unwrap();
    let text = "---\ntype: Note\n---\nours\n";
    assert_that!(id_at(&s, "/c.md")).is_equal_to(id);
    assert_that!(s.knowledge_file("/c.md").unwrap().text.as_str()).is_equal_to(text);
    assert_that!(text_at(&s, "/a.md")).is_none();
    assert_that!(approved(&s, RECEIVED)).is_false();
}

#[test]
fn move_of_approved_file_stays_approved() {
    let (_data, mut s) = seeded(&[("/a.md", NOTE)], &[NOTE]);
    s.apply("x", vec![moved("/a.md", "/b.md"), edit("/b.md", "body", "text")], None).unwrap();
    let text = "---\ntype: Note\n---\ntext\n";
    assert_that!(s.knowledge_file("/b.md").unwrap().text.as_str()).is_equal_to(text);
}

#[test]
fn failed_batch_approves_nothing() {
    let (_data, mut s) = seeded(&[], &[]);
    let before = s.deck().clone();
    let err = s.apply("x", vec![write("/a.md", MINE), delete("/missing.md")], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Op);
    assert_that!(s.deck()).is_equal_to(&before);
    assert_that!(approved(&s, MINE)).is_false();
}

/// Where the simulation does not follow a step, core refuses the batch: nothing is approved.
fn assert_core_refused(s: &mut Session, ops: Vec<Op>, reason: KnowledgeReason) {
    let (before, count) = (s.deck().clone(), s.library.knowledge_approvals().count());
    let err = s.apply("x", ops, None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Op);
    assert_that!(knowledge_reason(&err)).is_equal_to(Some(reason));
    assert_that!(s.deck()).is_equal_to(&before);
    assert_that!(approved(s, RECEIVED)).is_false();
    assert_that!(s.library.knowledge_approvals().count()).is_equal_to(count);
}

#[test]
fn move_onto_occupied_path_then_edit_refused_by_core() {
    // /a.md is approved: a move of an awaiting file is refused by the session first.
    let (_data, mut s) = seeded(&[("/a.md", MINE), ("/b.md", NOTE)], &[NOTE, MINE]);
    // The move is not followed (the target is taken), so the edit reaches the approved /b.md.
    let ops = vec![moved("/a.md", "/b.md"), edit("/b.md", "body", "text")];
    assert_core_refused(&mut s, ops, KnowledgeReason::Exists);
}

#[test]
fn write_with_received_id_at_free_path_then_edit_refused_by_core() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    let id = id_at(&s, "/a.md");
    // The write takes the received file's id at a new path; core refuses the id as taken.
    let ops = vec![Op::KnowledgeWrite { path: "/c.md".into(), text: MINE.into(), id }, edit("/c.md", "mine", "ours")];
    assert_core_refused(&mut s, ops, KnowledgeReason::IdTaken);
}

#[test]
fn undo_redo_of_agent_write_never_approves() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    s.apply_by_agent("x", vec![write("/a.md", MINE)], None).unwrap();
    let count = s.library.knowledge_approvals().count();
    s.undo().unwrap();
    assert_that!(text_at(&s, "/a.md").as_deref()).is_equal_to(Some(RECEIVED));
    assert_that!(s.knowledge_file("/a.md").unwrap_err().kind()).is_equal_to(SessionErrorKind::KnowledgeAwaiting);
    s.redo().unwrap();
    assert_that!(unstamped(&s.knowledge_file("/a.md").unwrap().text)).is_equal_to(MINE.to_owned());
    assert_that!(s.library.knowledge_approvals().count()).is_equal_to(count);
}

#[test]
fn merge_never_approves() {
    let (a_data, b_data) = (fresh_dir("oku-slides-knowledge-merge-a-"), fresh_dir("oku-slides-knowledge-merge-b-"));
    let mut a = opened_on(a_data.path());
    let shared = a_data.path().join("deck.oku-slides.pdf");
    a.save(Some(&shared)).unwrap();
    let mut b = session_on(b_data.path());
    b.open(&shared).unwrap();
    b.apply("x", vec![write("/k.md", MINE)], None).unwrap();
    let theirs = b_data.path().join("theirs.oku-slides.pdf");
    b.save(Some(&theirs)).unwrap();
    a.merge_file(&theirs).unwrap();
    assert_that!(text_at(&a, "/k.md").as_deref()).is_equal_to(Some(MINE));
    assert_that!(a.knowledge_file("/k.md").unwrap_err().kind()).is_equal_to(SessionErrorKind::KnowledgeAwaiting);
    assert_that!(a.library.knowledge_approvals().count()).is_equal_to(0);
}

/// A damaged approvals file (a folder in its place) refuses the apply and rolls back the
/// widgets a pin change installed.
#[test]
fn damaged_approvals_refuse_apply_with_widgets_unchanged() {
    let data = fresh_dir("oku-slides-knowledge-damaged-");
    std::fs::create_dir(data.path().join("knowledge-approvals.json")).unwrap();
    let mut s = session_on(data.path());
    let m = manifest_ns(crate::test_support::ATELIER, "atelier", &["base"]);
    let pin = add_version(&s.library, &m, &[]);
    let (deck, pins, registry) = (s.deck().clone(), s.widgets_pins.clone(), s.registry.clone());
    let err = s.apply("x", vec![add_pack(&m, &pin), write("/a.md", MINE)], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeApprovalsDamaged);
    assert_that!(s.deck()).is_equal_to(&deck);
    assert_that!(s.widgets_pins).is_equal_to(pins);
    assert_that!(std::sync::Arc::ptr_eq(&s.registry, &registry)).is_true();
}

/// A received bundle past a limit: writes and edits are refused, moves and deletes pass.
#[test]
fn over_limit_refuses_write_and_edit_allows_delete_and_move() {
    let big = format!("---\ntype: Note\n---\n{}", "a".repeat(MAX_FILE_BYTES));
    let (_data, mut s) = seeded(&[("/a.md", NOTE), ("/big.md", &big)], &[NOTE, &big]);
    for op in [write("/a.md", MINE), write("/new.md", MINE), edit("/a.md", "body", "text")] {
        let err = s.apply("x", vec![op], None).unwrap_err();
        assert_that!(knowledge_reason(&err)).is_equal_to(Some(KnowledgeReason::OverLimit));
    }
    s.apply("x", vec![moved("/a.md", "/b.md"), delete("/big.md")], None).unwrap();
    s.apply("x", vec![write("/b.md", MINE)], None).unwrap();
    assert_that!(s.knowledge_file("/b.md").unwrap().text.as_str()).is_equal_to(MINE);
}

#[test]
fn history_refuses_knowledge_ops() {
    let data = fresh_dir("oku-slides-knowledge-history-");
    let mut s = opened_on(data.path());
    s.history_open().unwrap();
    let err = s.apply("x", vec![write("/a.md", MINE)], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InHistory);
}

#[test]
fn apply_silent_ops_refuses_knowledge_ops() {
    let (_data, mut s) = seeded(&[], &[]);
    let err = s.apply_silent_ops(vec![write("/a.md", MINE)]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::WrongDocument);
    assert_that!(text_at(&s, "/a.md")).is_none();
}

#[test]
fn apply_by_agent_path_approves() {
    let (_data, mut s) = seeded(&[], &[]);
    s.apply_by_agent("x", vec![write("/a.md", MINE)], None).unwrap();
    assert_that!(unstamped(&s.knowledge_file("/a.md").unwrap().text)).is_equal_to(MINE.to_owned());
}

/// Stamps from a counter: each batch gets a distinct id, at a fixed time.
fn counted_stamps(s: &mut Session) {
    let n = std::sync::atomic::AtomicU8::new(0);
    s.stamp_source = Box::new(move || {
        let id = n.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok((time::macros::datetime!(2026-11-25 10:20:30.456 UTC), [id; 8]))
    });
}

#[test]
fn agent_write_of_received_text_leaves_it_awaiting() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    s.apply_by_agent("x", vec![write("/b.md", RECEIVED)], None).unwrap();
    assert_that!(approved(&s, RECEIVED)).is_false();
    assert_that!(s.knowledge_file("/a.md").unwrap_err().kind()).is_equal_to(SessionErrorKind::KnowledgeAwaiting);
    let written = s.knowledge_file("/b.md").unwrap().text;
    assert_that!(written.as_str()).is_not_equal_to(RECEIVED);
    assert_that!(unstamped(&written)).is_equal_to(RECEIVED.to_owned());
}

#[test]
fn agent_edit_result_restamped_and_approved() {
    let (_data, mut s) = seeded(&[("/a.md", MINE)], &[MINE]);
    counted_stamps(&mut s);
    s.apply_by_agent("x", vec![edit("/a.md", "mine", "ours")], None).unwrap();
    let first = s.knowledge_file("/a.md").unwrap().text;
    s.apply_by_agent("x", vec![edit("/a.md", "ours", "theirs")], None).unwrap();
    let second = s.knowledge_file("/a.md").unwrap().text;
    assert_that!(unstamped(&second)).is_equal_to("---\ntype: Note\n---\ntheirs\n".to_owned());
    assert_that!(second.matches("generated:").count()).is_equal_to(1);
    assert_that!(second.contains("0101010101010101")).is_true();
    assert_that!(first.contains("0000000000000000")).is_true();
    assert_that!(approved(&s, &second)).is_true();
}

#[test]
fn stamp_past_frontmatter_limit_refuses_batch_nothing_approved() {
    let (_data, mut s) = seeded(&[], &[]);
    let pad = "x".repeat(MAX_FRONTMATTER_BYTES - "type: Note\ntitle: \n".len());
    let text = format!("---\ntype: Note\ntitle: {pad}\n---\nbody\n");
    let before = s.library.knowledge_approvals().count();
    let err = s.apply_by_agent("x", vec![write("/a.md", &text), write("/b.md", MINE)], None).unwrap_err();
    assert_that!(knowledge_reason(&err))
        .is_equal_to(Some(KnowledgeReason::Frontmatter(oku_slides_core::FrontmatterReason::TooLarge)));
    assert_that!(s.knowledge_file("/b.md").unwrap_err().kind()).is_equal_to(SessionErrorKind::KnowledgeNotFound);
    assert_that!(s.library.knowledge_approvals().count()).is_equal_to(before);
    s.apply("x", vec![write("/a.md", &text)], None).unwrap();
}

#[test]
fn user_apply_knowledge_write_not_stamped() {
    let (_data, mut s) = seeded(&[], &[]);
    s.set_agent_label(Some("claude"));
    s.apply("x", vec![write("/a.md", MINE)], None).unwrap();
    assert_that!(s.knowledge_file("/a.md").unwrap().text.as_str()).is_equal_to(MINE);
}

#[test]
fn agent_apply_with_label_carries_cleaned_label() {
    let (_data, mut s) = seeded(&[], &[]);
    s.set_agent_label(Some("omp\n\"x\"\u{202e}"));
    s.apply_by_agent("x", vec![write("/a.md", MINE)], None).unwrap();
    let text = s.knowledge_file("/a.md").unwrap().text;
    assert_that!(text.contains("x-oku-slides-agent: \"omp \u{2bc}x\u{2bc}\"\n")).is_true();
}

#[test]
fn agent_label_after_end_turn_absent() {
    let (_data, mut s) = seeded(&[], &[]);
    s.begin_turn(1, "go");
    s.set_agent_label(Some("omp"));
    s.end_turn();
    s.apply_by_agent("x", vec![write("/a.md", MINE)], None).unwrap();
    let text = s.knowledge_file("/a.md").unwrap().text;
    assert_that!(text.contains("x-oku-slides-agent")).is_false();
    assert_that!(text.contains("generated:")).is_true();
}

#[test]
fn agent_batch_undo_redo_one_step_same_stamp() {
    let (_data, mut s) = seeded(&[], &[]);
    s.apply_by_agent("x", vec![write("/a.md", MINE), write("/b.md", NOTE)], None).unwrap();
    let stamped = text_at(&s, "/a.md");
    assert_that!(stamped.as_deref().is_some_and(|t| t.contains("generated:"))).is_true();
    s.undo().unwrap();
    assert_that!(text_at(&s, "/a.md")).is_none();
    assert_that!(text_at(&s, "/b.md")).is_none();
    s.redo().unwrap();
    assert_that!(text_at(&s, "/a.md")).is_equal_to(stamped);
}

#[test]
fn agent_write_flow_mapping_refused_localized_in_fr() {
    let (_data, mut s) = seeded(&[], &[]);
    let err = s.apply_by_agent("x", vec![write("/a.md", "---\n{type: Note}\n---\n")], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeStamp);
    let fr = Localizer::new(Some("fr"));
    assert_that!(err.localize(&fr)).contains(fr.tr("knowledge-frontmatter-flow-mapping"));
    assert_that!(text_at(&s, "/a.md")).is_none();
}

#[test]
fn agent_apply_stamp_source_failure_refuses_batch() {
    let (_data, mut s) = seeded(&[], &[]);
    s.stamp_source = Box::new(|| Err(getrandom::Error::UNSUPPORTED));
    let err = s.apply_by_agent("x", vec![write("/a.md", MINE)], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeStampRandom);
    assert_that!(err.localize(&Localizer::new(Some("fr"))))
        .contains(Localizer::new(Some("fr")).tr("session-knowledge-stamp-random"));
    assert_that!(text_at(&s, "/a.md")).is_none();
}
