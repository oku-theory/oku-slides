use oku_slides_core::knowledge::limits::{MAX_FILE_BYTES, MAX_FILES, MAX_TOTAL_BYTES, READ_FACTOR};
use oku_slides_core::{Op, asset_id, kinds};
use oku_slides_i18n::{Localize, Localizer};
use serde_json::{Value, json};
use speculoos::prelude::*;
use tempfile::TempDir;

use crate::knowledge_fixture::{approve_knowledge, seed_knowledge};
use crate::pack::ESSENTIEL_PACK;
use crate::request::Request;
use crate::test_support::{fresh_dir, opened_on, session_on};
use crate::theme::PAPIER_THEME;
use crate::{KnowledgeApproval, Session, SessionErrorKind};

const NOTE: &str = "---\ntype: Note\n---\nbody\n";
const RECEIVED: &str = "---\ntype: Note\n---\nreceived, maybe injected\n";
const MINE: &str = "---\ntype: Note\n---\nmine\n";

/// A session on a fresh data dir whose deck holds `files` (path, text), `approved` texts approved.
fn seeded(files: &[(&str, &str)], approved: &[&str]) -> (TempDir, Session) {
    let data = fresh_dir("oku-slides-knowledge-review-");
    approve_knowledge(data.path(), approved);
    let mut s = session_on(data.path());
    seed_knowledge(&mut s, files, Vec::new());
    (data, s)
}

/// The id of the file at `path`.
fn id_at(s: &Session, path: &str) -> String {
    s.deck().knowledge.iter().find(|(_, f)| f.path == path).map(|(id, _)| id.clone()).expect("no file at this path")
}

/// The approval of the file at `path` as shown now.
fn approval_of(s: &Session, path: &str) -> KnowledgeApproval {
    let id = id_at(s, path);
    let hash = asset_id(s.deck().knowledge[&id].text.as_bytes());
    KnowledgeApproval { id, hash }
}

fn approved(s: &Session, text: &str) -> bool {
    s.library.knowledge_approvals().is_approved(&asset_id(text.as_bytes()))
}

/// The answer to `req`, as the interface reads it.
fn call(s: &mut Session, req: Request) -> Value {
    serde_json::from_str(&s.handle(req).unwrap()).unwrap()
}

fn refusal(s: &mut Session, req: Request) -> SessionErrorKind {
    s.handle(req).unwrap_err().kind()
}

fn approve(files: Vec<KnowledgeApproval>) -> Request {
    Request::ApproveKnowledge { files }
}

fn text_of(id: &str) -> Request {
    Request::KnowledgeText { id: id.to_owned() }
}

fn moved(from: &str, to: &str) -> Op {
    Op::KnowledgeMove { from: from.into(), to: to.into() }
}

fn write(path: &str, text: &str) -> Op {
    Op::KnowledgeWrite { path: path.into(), text: text.into(), id: None }
}

fn slide_added() -> Op {
    Op::AddSlide { index: None, defaults: false, slide: oku_slides_core::Slide::new("neuve", kinds::essentiel::BLANK) }
}

#[test]
fn knowledge_review_lists_only_awaiting_files_by_path() {
    let titled = "---\ntype: Note\ntitle: \"Plan \u{202e}secret\u{202c} d\u{2019}action\"\n---\nbody\n";
    let long = format!("/{}.md", "a".repeat(100));
    let (_data, mut s) = seeded(&[("/z.md", RECEIVED), ("/a.md", NOTE), ("/m.md", titled), (&long, MINE)], &[NOTE]);
    let review = call(&mut s, Request::KnowledgeReview);
    let files = review["files"].as_array().unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f["path"].as_str().unwrap()).collect();
    let cut = format!("/{}…", "a".repeat(58));
    assert_that!(paths).is_equal_to(vec![cut.as_str(), "/m.md", "/z.md"]);
    assert_that!(files[1]["title"].as_str()).is_equal_to(Some("Plan secret dʼaction"));
    assert_that!(files[1]["id"].as_str()).is_equal_to(Some(id_at(&s, "/m.md").as_str()));
    assert_that!(files[1]["bytes"].as_u64()).is_equal_to(Some(titled.len() as u64));
    assert_that!(files[2]["title"]).is_equal_to(&Value::Null);
    assert_that!(review["more"].as_u64()).is_equal_to(Some(0));
    assert_that!(review["over_limit"]).is_equal_to(json!([]));
}

#[test]
fn knowledge_review_past_two_hundred_rows_counts_the_rest() {
    let paths: Vec<String> = (0..205).map(|n| format!("/f{n:03}.md")).collect();
    let files: Vec<(&str, &str)> = paths.iter().map(|p| (p.as_str(), NOTE)).collect();
    let (_data, mut s) = seeded(&files, &[]);
    let review = call(&mut s, Request::KnowledgeReview);
    assert_that!(review["files"].as_array().unwrap().len()).is_equal_to(200);
    assert_that!(review["files"][199]["path"].as_str()).is_equal_to(Some("/f199.md"));
    assert_that!(review["more"].as_u64()).is_equal_to(Some(5));
}

#[test]
fn knowledge_text_of_an_awaiting_file_is_whole_with_its_hash() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    let id = id_at(&s, "/a.md");
    let text = call(&mut s, text_of(&id));
    assert_that!(text).is_equal_to(json!({
        "id": id,
        "path": "/a.md",
        "hash": asset_id(RECEIVED.as_bytes()),
        "text": RECEIVED,
        "truncated": 0,
    }));
}

#[test]
fn knowledge_text_past_the_read_ceiling_is_cut_and_hashed_whole() {
    let big = "a".repeat(MAX_FILE_BYTES + 10);
    let (_data, mut s) = seeded(&[("/big.md", &big)], &[]);
    let id = id_at(&s, "/big.md");
    let text = call(&mut s, text_of(&id));
    assert_that!(text["text"].as_str().map(str::len)).is_equal_to(Some(MAX_FILE_BYTES));
    assert_that!(text["truncated"].as_u64()).is_equal_to(Some(10));
    assert_that!(text["hash"].as_str()).is_equal_to(Some(asset_id(big.as_bytes()).as_str()));
}

#[test]
fn knowledge_text_cut_inside_a_character_keeps_it_whole_out() {
    let big = format!("{}é{}", "a".repeat(MAX_FILE_BYTES - 1), "b".repeat(4));
    let (_data, mut s) = seeded(&[("/big.md", &big)], &[]);
    let id = id_at(&s, "/big.md");
    let text = call(&mut s, text_of(&id));
    assert_that!(text["text"].as_str().map(str::len)).is_equal_to(Some(MAX_FILE_BYTES - 1));
    assert_that!(text["truncated"].as_u64()).is_equal_to(Some(6));
}

#[test]
fn knowledge_text_of_an_approved_file_is_shown_too() {
    let (_data, mut s) = seeded(&[("/a.md", NOTE)], &[NOTE]);
    let id = id_at(&s, "/a.md");
    assert_that!(call(&mut s, text_of(&id))["text"].as_str()).is_equal_to(Some(NOTE));
}

#[test]
fn knowledge_text_of_an_unknown_id_is_refused() {
    let (_data, mut s) = seeded(&[("/a.md", NOTE)], &[]);
    assert_that!(refusal(&mut s, text_of(&"f".repeat(32)))).is_equal_to(SessionErrorKind::KnowledgeIdNotFound);
}

#[test]
fn approve_knowledge_of_two_files_opens_them_to_the_agent() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED), ("/b.md", MINE)], &[]);
    let files = vec![approval_of(&s, "/a.md"), approval_of(&s, "/b.md")];
    let snap = call(&mut s, approve(files));
    assert_that!(snap["knowledge"]).is_equal_to(&Value::Null);
    assert_that!(s.knowledge_file("/a.md").map(|r| r.text)).is_ok_containing(RECEIVED.to_owned());
    assert_that!(s.knowledge_file("/b.md").map(|r| r.text)).is_ok_containing(MINE.to_owned());
    assert_that!(s.all_knowledge_entries().iter().all(|e| e.approved)).is_true();
}

#[test]
fn approve_knowledge_of_nothing_approves_nothing() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    let snap = call(&mut s, approve(Vec::new()));
    assert_that!(snap["knowledge"]["awaiting"].as_u64()).is_equal_to(Some(1));
    assert_that!(s.library.knowledge_approvals().count()).is_equal_to(0);
}

#[test]
fn approve_knowledge_naming_a_file_twice_is_refused() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    let one = approval_of(&s, "/a.md");
    let err = s.handle(approve(vec![one.clone(), one])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeApprovalInvalid);
    assert_that!(approved(&s, RECEIVED)).is_false();
    let (fr, en) = (Localizer::new(Some("fr")), Localizer::new(Some("en")));
    assert_that!(err.localize(&fr)).is_equal_to(fr.tr("session-knowledge-approval-invalid"));
    assert_that!(err.localize(&fr)).is_not_equal_to(err.localize(&en));
}

#[test]
fn approve_knowledge_past_the_file_count_is_refused() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    let files: Vec<KnowledgeApproval> =
        (0..=MAX_FILES).map(|n| KnowledgeApproval { id: format!("{n:032x}"), hash: String::new() }).collect();
    assert_that!(refusal(&mut s, approve(files))).is_equal_to(SessionErrorKind::KnowledgeApprovalInvalid);
}

#[test]
fn approve_knowledge_with_one_stale_file_approves_none() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED), ("/b.md", MINE)], &[]);
    let stale = KnowledgeApproval { id: id_at(&s, "/b.md"), hash: asset_id(b"an older text") };
    let valid = approval_of(&s, "/a.md");
    let err = refusal(&mut s, approve(vec![valid, stale]));
    assert_that!(err).is_equal_to(SessionErrorKind::KnowledgeChanged);
    assert_that!(approved(&s, RECEIVED)).is_false();
    assert_that!(approved(&s, MINE)).is_false();
}

#[test]
fn approve_knowledge_of_an_unknown_id_is_refused() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    let ghost = KnowledgeApproval { id: "f".repeat(32), hash: asset_id(RECEIVED.as_bytes()) };
    let err = s.handle(approve(vec![approval_of(&s, "/a.md"), ghost])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeIdNotFound);
    assert_that!(approved(&s, RECEIVED)).is_false();
}

#[test]
fn approve_knowledge_of_an_approved_file_changes_nothing() {
    let (_data, mut s) = seeded(&[("/a.md", NOTE)], &[NOTE]);
    let count = s.library.knowledge_approvals().count();
    let files = vec![approval_of(&s, "/a.md")];
    let snap = call(&mut s, approve(files));
    assert_that!(snap["knowledge"]).is_equal_to(&Value::Null);
    assert_that!(s.library.knowledge_approvals().count()).is_equal_to(count);
}

/// Approval is checked before the size: re-approving a big file the user approved is a no-op.
#[test]
fn approve_knowledge_of_an_approved_file_past_the_read_ceiling_changes_nothing() {
    let big = "a".repeat(MAX_FILE_BYTES + 1);
    let (_data, mut s) = seeded(&[("/big.md", &big)], &[&big]);
    let count = s.library.knowledge_approvals().count();
    let files = vec![approval_of(&s, "/big.md")];
    let snap = call(&mut s, approve(files));
    assert_that!(snap["knowledge"]["awaiting"].as_u64()).is_equal_to(Some(0));
    assert_that!(s.library.knowledge_approvals().count()).is_equal_to(count);
}

#[test]
fn approve_knowledge_with_a_forged_hash_is_refused() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    let forged = KnowledgeApproval { id: id_at(&s, "/a.md"), hash: asset_id(MINE.as_bytes()) };
    assert_that!(refusal(&mut s, approve(vec![forged]))).is_equal_to(SessionErrorKind::KnowledgeChanged);
    assert_that!(approved(&s, RECEIVED)).is_false();
    assert_that!(approved(&s, MINE)).is_false();
}

#[test]
fn approve_knowledge_of_a_file_past_the_read_ceiling_is_refused() {
    let big = "a".repeat(MAX_FILE_BYTES + 1);
    let (_data, mut s) = seeded(&[("/big.md", &big)], &[]);
    let err = s.handle(approve(vec![approval_of(&s, "/big.md")])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeTooLargeToApprove);
    assert_that!(approved(&s, &big)).is_false();
    let fr = Localizer::new(Some("fr"));
    assert_that!(err.localize(&fr)).contains("/big.md");
    assert_that!(err.localize(&fr)).is_not_equal_to(err.localize(&Localizer::new(Some("en"))));
}

/// Review Focus 2: a merge changes the file between its text and its approval.
#[test]
fn approve_knowledge_after_a_merge_changed_the_text_is_refused() {
    let (a_data, b_data) = (fresh_dir("oku-slides-knowledge-review-a-"), fresh_dir("oku-slides-knowledge-review-b-"));
    let mut a = opened_on(a_data.path());
    seed_knowledge(&mut a, &[("/k.md", RECEIVED)], Vec::new());
    let shared = a_data.path().join("deck.oku-slides.pdf");
    a.save(Some(&shared)).unwrap();
    let mut b = session_on(b_data.path());
    b.open(&shared).unwrap();
    b.apply("x", vec![write("/k.md", MINE)], None).unwrap();
    let theirs = b_data.path().join("theirs.oku-slides.pdf");
    b.save(Some(&theirs)).unwrap();
    let id = id_at(&a, "/k.md");
    let shown = call(&mut a, text_of(&id));
    a.merge_file(&theirs).unwrap();
    let hash = shown["hash"].as_str().unwrap().to_owned();
    let err = a.handle(approve(vec![KnowledgeApproval { id, hash }])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeChanged);
    assert_that!(approved(&a, RECEIVED)).is_false();
    assert_that!(approved(&a, MINE)).is_false();
    let fr = Localizer::new(Some("fr"));
    assert_that!(err.localize(&fr)).contains("/k.md");
    assert_that!(err.localize(&fr)).is_not_equal_to(err.localize(&Localizer::new(Some("en"))));
}

#[test]
fn approve_knowledge_then_move_keeps_the_approval() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    let files = vec![approval_of(&s, "/a.md")];
    call(&mut s, approve(files));
    s.apply("x", vec![moved("/a.md", "/b.md")], None).unwrap();
    assert_that!(s.knowledge_file("/b.md").map(|r| r.text)).is_ok_containing(RECEIVED.to_owned());
}

#[test]
fn knowledge_summary_counts_the_awaiting_files() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED), ("/b.md", NOTE), ("/c.md", MINE)], &[NOTE]);
    let snap = call(&mut s, Request::Snapshot);
    assert_that!(snap["knowledge"]).is_equal_to(json!({ "awaiting": 2, "over_limit": [] }));
}

#[test]
fn knowledge_summary_at_the_total_ceiling_names_nothing_one_byte_past_names_it() {
    let full = "a".repeat(MAX_FILE_BYTES);
    let paths: Vec<String> = (0..MAX_TOTAL_BYTES / MAX_FILE_BYTES).map(|n| format!("/f{n:02}.md")).collect();
    let mut files: Vec<(&str, &str)> = paths.iter().map(|p| (p.as_str(), full.as_str())).collect();
    let (_data, mut s) = seeded(&files, &[&full, "y"]);
    assert_that!(call(&mut s, Request::Snapshot)["knowledge"]).is_equal_to(&Value::Null);
    files.push(("/last.md", "y"));
    seed_knowledge(&mut s, &files, Vec::new());
    let snap = call(&mut s, Request::Snapshot);
    assert_that!(snap["knowledge"]).is_equal_to(json!({ "awaiting": 0, "over_limit": ["total-too-large"] }));
}

/// A read keeps at most `READ_FACTOR * MAX_FILES` files: the bundle a cut leaves still passes the
/// file limit (a seed past that ceiling would not be the bundle its replica reads back).
#[test]
fn knowledge_summary_of_a_bundle_cut_by_the_read_ceiling_names_too_many_files() {
    let paths: Vec<String> = (0..READ_FACTOR * MAX_FILES).map(|n| format!("/f{n:05}.md")).collect();
    let files: Vec<(&str, &str)> = paths.iter().map(|p| (p.as_str(), "")).collect();
    let (_data, mut s) = seeded(&files, &[""]);
    let snap = call(&mut s, Request::Snapshot);
    assert_that!(snap["knowledge"]).is_equal_to(json!({ "awaiting": 0, "over_limit": ["too-many-files"] }));
}

#[test]
fn knowledge_summary_after_a_slide_edit_is_not_computed_again() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    call(&mut s, Request::Snapshot);
    let before = s.knowledge_summary.computations();
    s.apply("x", vec![slide_added()], None).unwrap();
    call(&mut s, Request::Snapshot);
    assert_that!(s.knowledge_summary.computations()).is_equal_to(before);
}

/// The review and the summary share one hashing pass per generation and approvals count.
#[test]
fn knowledge_review_and_snapshot_share_one_computation() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED), ("/b.md", NOTE)], &[NOTE]);
    let before = s.knowledge_summary.computations();
    let review = call(&mut s, Request::KnowledgeReview);
    assert_that!(review["files"].as_array().map(Vec::len)).is_equal_to(Some(1));
    assert_that!(s.knowledge_summary.computations()).is_equal_to(before + 1);
    call(&mut s, Request::Snapshot);
    call(&mut s, Request::KnowledgeReview);
    assert_that!(s.knowledge_summary.computations()).is_equal_to(before + 1);
    let files = vec![approval_of(&s, "/a.md")];
    call(&mut s, approve(files));
    let review = call(&mut s, Request::KnowledgeReview);
    assert_that!(review["files"]).is_equal_to(json!([]));
    assert_that!(s.knowledge_summary.computations()).is_equal_to(before + 2);
}

#[test]
fn knowledge_summary_after_a_knowledge_write_and_its_undo_is_computed_again() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    call(&mut s, Request::Snapshot);
    let before = s.knowledge_summary.computations();
    s.apply("x", vec![write("/b.md", RECEIVED)], None).unwrap();
    let written = call(&mut s, Request::Snapshot);
    assert_that!(s.knowledge_summary.computations()).is_equal_to(before + 1);
    assert_that!(written["knowledge"]).is_equal_to(&Value::Null);
    s.undo().unwrap();
    call(&mut s, Request::Snapshot);
    assert_that!(s.knowledge_summary.computations()).is_equal_to(before + 2);
}

#[test]
fn knowledge_summary_after_a_merge_is_computed_again() {
    let (a_data, b_data) = (fresh_dir("oku-slides-knowledge-review-a-"), fresh_dir("oku-slides-knowledge-review-b-"));
    let mut a = opened_on(a_data.path());
    let shared = a_data.path().join("deck.oku-slides.pdf");
    a.save(Some(&shared)).unwrap();
    let mut b = session_on(b_data.path());
    b.open(&shared).unwrap();
    b.apply("x", vec![write("/k.md", MINE)], None).unwrap();
    let theirs = b_data.path().join("theirs.oku-slides.pdf");
    b.save(Some(&theirs)).unwrap();
    assert_that!(call(&mut a, Request::Snapshot)["knowledge"]).is_equal_to(&Value::Null);
    a.merge_file(&theirs).unwrap();
    assert_that!(call(&mut a, Request::Snapshot)["knowledge"]["awaiting"].as_u64()).is_equal_to(Some(1));
}

#[test]
fn knowledge_summary_after_an_approval_is_computed_again() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    call(&mut s, Request::Snapshot);
    let before = s.knowledge_summary.computations();
    let files = vec![approval_of(&s, "/a.md")];
    let snap = call(&mut s, approve(files));
    assert_that!(snap["knowledge"]).is_equal_to(&Value::Null);
    assert_that!(s.knowledge_summary.computations()).is_equal_to(before + 1);
}

/// These tabs hold no deck knowledge: the `Mode::Deck` guard itself is checked in history
/// (`knowledge_summary_in_history_is_none`), where awaiting knowledge is held.
#[test]
fn knowledge_summary_of_a_help_pack_or_theme_tab_is_none() {
    let data = fresh_dir("oku-slides-knowledge-review-tabs-");
    let mut pack = session_on(data.path());
    pack.open_pack_tab_here(ESSENTIEL_PACK, None).unwrap();
    let mut theme = session_on(data.path());
    theme.open_theme_here(PAPIER_THEME).unwrap();
    let mut help = session_on(data.path());
    help.open_help("fr", None).unwrap();
    for s in [&pack, &theme, &help] {
        assert_that!(s.snapshot().knowledge).is_none();
    }
}

/// The deck held awaits approval: only the `Mode::Deck` guard leaves the summary out.
#[test]
fn knowledge_summary_in_history_is_none() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    s.apply("x", vec![slide_added()], None).unwrap();
    s.history_open().unwrap();
    assert_that!(s.snapshot().knowledge).is_none();
}

/// The three review requests, each once.
fn review_requests(s: &Session) -> [Request; 3] {
    let id = s.deck().knowledge.keys().next().cloned().unwrap_or_else(|| "0".repeat(32));
    [Request::KnowledgeReview, text_of(&id), approve(Vec::new())]
}

#[test]
fn knowledge_review_requests_outside_a_deck_tab_are_refused() {
    let data = fresh_dir("oku-slides-knowledge-review-tabs-");
    let mut pack = session_on(data.path());
    pack.open_pack_tab_here(ESSENTIEL_PACK, None).unwrap();
    let mut theme = session_on(data.path());
    theme.open_theme_here(PAPIER_THEME).unwrap();
    let mut help = session_on(data.path());
    help.open_help("fr", None).unwrap();
    for s in [&mut pack, &mut theme, &mut help] {
        for req in review_requests(s) {
            assert_that!(refusal(s, req)).is_equal_to(SessionErrorKind::WrongDocument);
        }
    }
}

#[test]
fn knowledge_review_methods_called_outside_a_deck_tab_are_refused() {
    let data = fresh_dir("oku-slides-knowledge-review-direct-");
    let mut pack = session_on(data.path());
    pack.open_pack_tab_here(ESSENTIEL_PACK, None).unwrap();
    let mut theme = session_on(data.path());
    theme.open_theme_here(PAPIER_THEME).unwrap();
    let mut help = session_on(data.path());
    help.open_help("fr", None).unwrap();
    for s in [&pack, &theme, &help] {
        assert_that!(s.knowledge_review().unwrap_err().kind()).is_equal_to(SessionErrorKind::WrongDocument);
        assert_that!(s.knowledge_text(&"0".repeat(32)).unwrap_err().kind())
            .is_equal_to(SessionErrorKind::WrongDocument);
        assert_that!(s.approve_knowledge(&[]).unwrap_err().kind()).is_equal_to(SessionErrorKind::WrongDocument);
    }
}

#[test]
fn knowledge_review_methods_called_in_history_are_refused() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    s.apply("x", vec![slide_added()], None).unwrap();
    s.history_open().unwrap();
    let id = id_at(&s, "/a.md");
    assert_that!(s.knowledge_review().unwrap_err().kind()).is_equal_to(SessionErrorKind::InHistory);
    assert_that!(s.knowledge_text(&id).unwrap_err().kind()).is_equal_to(SessionErrorKind::InHistory);
    assert_that!(s.approve_knowledge(&[]).unwrap_err().kind()).is_equal_to(SessionErrorKind::InHistory);
}

#[test]
fn knowledge_review_requests_in_history_are_refused() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], &[]);
    s.apply("x", vec![slide_added()], None).unwrap();
    s.history_open().unwrap();
    for req in review_requests(&s) {
        assert_that!(refusal(&mut s, req)).is_equal_to(SessionErrorKind::InHistory);
    }
}

#[test]
fn knowledge_review_snapshot_json_has_the_knowledge_field() {
    let (_data, mut s) = seeded(&[], &[]);
    let snap = call(&mut s, Request::Snapshot);
    assert_that!(snap.as_object().map(|o| o.contains_key("knowledge"))).is_equal_to(Some(true));
    assert_that!(snap["knowledge"]).is_equal_to(&Value::Null);
}
