use oku_slides_core::knowledge::is_file_id;
use oku_slides_core::knowledge::limits::MAX_FILE_BYTES;
use oku_slides_core::{KnowledgeReason, Op, asset_id};
use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;
use tempfile::TempDir;

use crate::SessionErrorKind;
use crate::knowledge_fixture::{approve_knowledge, seed_knowledge};
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
    assert_that!(s.knowledge_file("/a.md").unwrap().text.as_str()).is_equal_to(MINE);
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
    assert_that!(s.knowledge_file("/a.md").unwrap().text.as_str()).is_equal_to(MINE);
}
