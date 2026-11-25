use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::error::{FrontmatterReason, OpErrorKind, PathReason};
use crate::knowledge::limits::{MAX_FILE_BYTES, MAX_FILES, MAX_TOTAL_BYTES};
use crate::ops::{Op, apply_all, ops_schema};
use crate::test_registry::registry;

const ID_A: &str = "0123456789abcdef0123456789abcdef";
const ID_B: &str = "fedcba9876543210fedcba9876543210";
const NOTE: &str = "---\ntype: Note\n---\nbody\n";

fn write_op(path: &str, text: &str, id: Option<&str>) -> Op {
    Op::KnowledgeWrite { path: path.into(), text: text.into(), id: id.map(str::to_owned) }
}

fn apply(deck: &mut Deck, op: Op) -> Result<(), OpError> {
    op.apply(deck, &registry())
}

fn with_note() -> Deck {
    let mut deck = Deck::empty();
    apply(&mut deck, write_op("/a.md", NOTE, Some(ID_A))).unwrap();
    deck
}

fn reason(result: Result<(), OpError>) -> KnowledgeReason {
    let err = result.unwrap_err();
    assert_eq!(err.kind(), OpErrorKind::Knowledge);
    err.knowledge_reason().cloned().unwrap()
}

fn text_of(deck: &Deck, id: &str) -> String {
    deck.knowledge[id].text.clone()
}

#[test]
fn write_creates_a_file_byte_for_byte() {
    let deck = with_note();
    assert_that!(deck.knowledge.len()).is_equal_to(1);
    assert_that!(deck.knowledge[ID_A].path.as_str()).is_equal_to("/a.md");
    assert_that!(text_of(&deck, ID_A).as_str()).is_equal_to(NOTE);
}

#[test]
fn write_on_an_existing_path_replaces_the_text_and_keeps_the_id() {
    let mut deck = with_note();
    let next = "---\ntype: Other\n---\n";
    apply(&mut deck, write_op("/a.md", next, None)).unwrap();
    apply(&mut deck, write_op("/a.md", NOTE, Some(ID_A))).unwrap();
    apply(&mut deck, write_op("/a.md", next, None)).unwrap();
    assert_that!(deck.knowledge.keys().map(String::as_str).collect::<Vec<_>>()).is_equal_to(vec![ID_A]);
    assert_that!(text_of(&deck, ID_A).as_str()).is_equal_to(next);
}

#[rstest]
#[case::missing(None, KnowledgeReason::IdMissing)]
#[case::invalid(Some("ABC"), KnowledgeReason::IdInvalid)]
#[case::upper_case(Some("0123456789ABCDEF0123456789ABCDEF"), KnowledgeReason::IdInvalid)]
#[case::taken(Some(ID_A), KnowledgeReason::IdTaken)]
fn creating_needs_a_valid_unused_id(#[case] id: Option<&str>, #[case] expected: KnowledgeReason) {
    let mut deck = with_note();
    let before = deck.clone();
    assert_that!(reason(apply(&mut deck, write_op("/b.md", NOTE, id)))).is_equal_to(expected);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn replacing_with_another_id_is_refused() {
    let mut deck = with_note();
    let before = deck.clone();
    assert_that!(reason(apply(&mut deck, write_op("/a.md", NOTE, Some(ID_B)))))
        .is_equal_to(KnowledgeReason::IdMismatch);
    assert_that!(deck).is_equal_to(before);
}

#[rstest]
#[case::relative("a.md", KnowledgeReason::Path(PathReason::NotAbsolute))]
#[case::not_markdown("/a.txt", KnowledgeReason::Path(PathReason::NotMarkdown))]
#[case::upper("/A.md", KnowledgeReason::Path(PathReason::BadSegment))]
fn a_bad_path_is_refused(#[case] path: &str, #[case] expected: KnowledgeReason) {
    let mut deck = Deck::empty();
    assert_that!(reason(apply(&mut deck, write_op(path, NOTE, Some(ID_A))))).is_equal_to(expected);
    assert_that!(deck.knowledge.is_empty()).is_true();
}

#[test]
fn a_bad_frontmatter_is_refused() {
    let mut deck = Deck::empty();
    let refused = reason(apply(&mut deck, write_op("/a.md", "no frontmatter", Some(ID_A))));
    assert_that!(refused).is_equal_to(KnowledgeReason::Frontmatter(FrontmatterReason::Missing));
    assert_that!(deck.knowledge.is_empty()).is_true();
}

#[test]
fn edit_replaces_the_single_occurrence() {
    let mut deck = with_note();
    let edit = Op::KnowledgeEdit { path: "/a.md".into(), old: "body".into(), new: "other".into() };
    apply(&mut deck, edit).unwrap();
    assert_that!(text_of(&deck, ID_A).as_str()).is_equal_to("---\ntype: Note\n---\nother\n");
}

fn edit_of(old: &str) -> Op {
    Op::KnowledgeEdit { path: "/a.md".into(), old: old.into(), new: "x".into() }
}

#[test]
fn edit_refuses_an_empty_old_text() {
    let mut deck = with_note();
    assert_that!(reason(apply(&mut deck, edit_of("")))).is_equal_to(KnowledgeReason::EditEmptyOld);
}

#[test]
fn edit_refuses_an_absent_old_text() {
    let mut deck = with_note();
    assert_that!(reason(apply(&mut deck, edit_of("zzz")))).is_equal_to(KnowledgeReason::EditNoMatch);
}

#[rstest]
#[case::once("---\ntype: Note\n---\nab\n", "a", true)]
#[case::overlapping("---\ntype: Note\n---\naaa\n", "aa", false)]
#[case::overlapping_multibyte("---\ntype: Note\n---\néé!é\n", "é!é", true)]
#[case::overlapping_after_a_multibyte_char("---\ntype: Note\n---\n😀😀😀\n", "😀😀", false)]
#[case::twice("---\ntype: Note\n---\naaaa\n", "aa", false)]
fn edit_treats_overlapping_matches_as_ambiguous(#[case] text: &str, #[case] old: &str, #[case] ok: bool) {
    let mut deck = Deck::empty();
    apply(&mut deck, write_op("/a.md", text, Some(ID_A))).unwrap();
    let result = apply(&mut deck, edit_of(old));
    if ok {
        assert_that!(result.is_ok()).is_true();
    } else {
        assert_that!(reason(result)).is_equal_to(KnowledgeReason::EditAmbiguous);
        assert_that!(text_of(&deck, ID_A).as_str()).is_equal_to(text);
    }
}

#[test]
fn edits_are_byte_exact_on_line_endings() {
    let mut deck = Deck::empty();
    apply(&mut deck, write_op("/a.md", "---\r\ntype: Note\r\n---\r\nline\r\nnext\r\n", Some(ID_A))).unwrap();
    assert_that!(reason(apply(&mut deck, edit_of("line\nnext")))).is_equal_to(KnowledgeReason::EditNoMatch);
    apply(&mut deck, edit_of("line\r\nnext")).unwrap();
    assert_that!(text_of(&deck, ID_A).as_str()).is_equal_to("---\r\ntype: Note\r\n---\r\nx\r\n");
}

#[test]
fn edit_rechecks_the_result() {
    let mut deck = with_note();
    let before = deck.clone();
    let edit = Op::KnowledgeEdit { path: "/a.md".into(), old: "type: Note\n".into(), new: String::new() };
    let refused = reason(apply(&mut deck, edit));
    assert_that!(matches!(refused, KnowledgeReason::Frontmatter(_))).is_true();
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn edit_of_an_unknown_file_is_not_found() {
    let mut deck = Deck::empty();
    assert_that!(reason(apply(&mut deck, edit_of("a")))).is_equal_to(KnowledgeReason::NotFound);
}

#[test]
fn move_keeps_the_id_and_the_text() {
    let mut deck = with_note();
    apply(&mut deck, Op::KnowledgeMove { from: "/a.md".into(), to: "/dir/b.md".into() }).unwrap();
    assert_that!(deck.knowledge[ID_A].path.as_str()).is_equal_to("/dir/b.md");
    assert_that!(text_of(&deck, ID_A).as_str()).is_equal_to(NOTE);
}

#[test]
fn move_rechecks_the_text_at_the_target() {
    let mut deck = with_note();
    let op = Op::KnowledgeMove { from: "/a.md".into(), to: "/index.md".into() };
    let refused = reason(apply(&mut deck, op));
    assert_that!(matches!(refused, KnowledgeReason::Frontmatter(FrontmatterReason::NotAllowed { .. }))).is_true();
    assert_that!(deck.knowledge[ID_A].path.as_str()).is_equal_to("/a.md");
}

#[test]
fn move_onto_an_existing_file_is_refused() {
    let mut deck = with_note();
    apply(&mut deck, write_op("/b.md", NOTE, Some(ID_B))).unwrap();
    let op = Op::KnowledgeMove { from: "/a.md".into(), to: "/b.md".into() };
    assert_that!(reason(apply(&mut deck, op))).is_equal_to(KnowledgeReason::Exists);
}

#[test]
fn move_onto_its_own_path_is_refused_as_existing() {
    let mut deck = with_note();
    let op = Op::KnowledgeMove { from: "/a.md".into(), to: "/a.md".into() };
    assert_that!(reason(apply(&mut deck, op))).is_equal_to(KnowledgeReason::Exists);
    assert_that!(deck).is_equal_to(with_note());
}

#[test]
fn move_and_delete_of_an_unknown_file_are_not_found() {
    let mut deck = Deck::empty();
    let op = Op::KnowledgeMove { from: "/a.md".into(), to: "/b.md".into() };
    assert_that!(reason(apply(&mut deck, op))).is_equal_to(KnowledgeReason::NotFound);
    let op = Op::KnowledgeDelete { path: "/a.md".into() };
    assert_that!(reason(apply(&mut deck, op))).is_equal_to(KnowledgeReason::NotFound);
}

#[test]
fn delete_removes_the_file() {
    let mut deck = with_note();
    apply(&mut deck, Op::KnowledgeDelete { path: "/a.md".into() }).unwrap();
    assert_that!(deck.knowledge.is_empty()).is_true();
}

fn id_n(n: usize) -> String {
    format!("{n:032x}")
}

#[test]
fn the_file_after_the_last_allowed_is_refused() {
    let mut deck = Deck::empty();
    for n in 0..MAX_FILES {
        apply(&mut deck, write_op(&format!("/f{n}.md"), NOTE, Some(&id_n(n)))).unwrap();
    }
    let refused = reason(apply(&mut deck, write_op("/last.md", NOTE, Some(&id_n(MAX_FILES)))));
    assert_that!(refused).is_equal_to(KnowledgeReason::TooManyFiles);
    // Replacing is still allowed at the ceiling.
    apply(&mut deck, write_op("/f0.md", NOTE, None)).unwrap();
}

fn padded(len: usize) -> String {
    let head = "---\ntype: Note\n---\n";
    format!("{head}{}", "a".repeat(len - head.len()))
}

#[test]
fn a_file_over_the_byte_ceiling_is_refused() {
    let mut deck = Deck::empty();
    apply(&mut deck, write_op("/a.md", &padded(MAX_FILE_BYTES), Some(ID_A))).unwrap();
    let refused = reason(apply(&mut deck, write_op("/b.md", &padded(MAX_FILE_BYTES + 1), Some(ID_B))));
    assert_that!(refused).is_equal_to(KnowledgeReason::FileTooLarge);
    let edit = Op::KnowledgeEdit { path: "/a.md".into(), old: "type: Note".into(), new: "type: Note\nx: yy".into() };
    assert_that!(reason(apply(&mut deck, edit))).is_equal_to(KnowledgeReason::FileTooLarge);
}

#[test]
fn a_bundle_over_the_byte_ceiling_is_refused() {
    let mut deck = Deck::empty();
    let files = MAX_TOTAL_BYTES / MAX_FILE_BYTES;
    for n in 0..files {
        apply(&mut deck, write_op(&format!("/f{n}.md"), &padded(MAX_FILE_BYTES), Some(&id_n(n)))).unwrap();
    }
    let refused = reason(apply(&mut deck, write_op("/more.md", NOTE, Some(&id_n(files)))));
    assert_that!(refused).is_equal_to(KnowledgeReason::TotalTooLarge);
    // A replacement that fits the freed bytes is accepted.
    apply(&mut deck, write_op("/f0.md", NOTE, None)).unwrap();
}

#[test]
fn an_edit_past_the_bundle_byte_ceiling_is_refused() {
    let mut deck = Deck::empty();
    let files = MAX_TOTAL_BYTES / MAX_FILE_BYTES;
    for n in 0..files {
        apply(&mut deck, write_op(&format!("/f{n}.md"), &padded(MAX_FILE_BYTES), Some(&id_n(n)))).unwrap();
    }
    // `/f0.md` has room to grow, the bundle only 3 bytes.
    apply(&mut deck, write_op("/f0.md", &padded(MAX_FILE_BYTES - 100), None)).unwrap();
    apply(&mut deck, write_op("/g.md", &padded(97), Some(&id_n(files)))).unwrap();
    let before = deck.clone();
    let edit = Op::KnowledgeEdit { path: "/f0.md".into(), old: "type: Note".into(), new: "type: Note\nx: y".into() };
    assert_that!(reason(apply(&mut deck, edit))).is_equal_to(KnowledgeReason::TotalTooLarge);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn a_failing_op_in_a_batch_leaves_the_deck_unchanged() {
    let mut deck = with_note();
    let before = deck.clone();
    let ops = vec![write_op("/b.md", NOTE, Some(ID_B)), Op::KnowledgeDelete { path: "/missing.md".into() }];
    let err = apply_all(&mut deck, ops, &registry()).unwrap_err();
    assert_that!(err.index()).is_equal_to(1);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn ops_round_trip_through_json_and_are_knowledge() {
    let op: Op = serde_json::from_str(r#"{"op":"knowledge_move","from":"/a.md","to":"/b.md"}"#).unwrap();
    assert_that!(op.is_knowledge()).is_true();
    assert_that!(Op::SetVar { key: "k".into(), value: serde_json::Value::Null }.is_knowledge()).is_false();
}

#[test]
fn ops_schema_omits_knowledge_ops() {
    let schema = ops_schema().unwrap().to_string();
    assert_that!(schema.contains("knowledge_")).is_false();
    assert_that!(schema.contains("set_var")).is_true();
    // The unfiltered schema has them: the filter, not a schemars shape change, hides them.
    let unfiltered = serde_json::to_string(&schemars::schema_for!(Vec<Op>)).unwrap();
    for op in ["knowledge_write", "knowledge_edit", "knowledge_move", "knowledge_delete"] {
        assert_that!(unfiltered.contains(&format!("\"{op}\""))).is_true();
    }
}

#[test]
fn an_ambiguous_edit_is_localized_with_the_path() {
    let mut deck = Deck::empty();
    apply(&mut deck, write_op("/a.md", "---\ntype: Note\n---\naaaa\n", Some(ID_A))).unwrap();
    let err = apply(&mut deck, edit_of("aa")).unwrap_err();
    assert_that!(err.knowledge_reason().map(KnowledgeReason::key)).is_equal_to(Some("knowledge-reason-edit-ambiguous"));
    assert_that!(err.subject()).is_equal_to(Some("/a.md"));
    let fr = Localizer::new(Some("fr"));
    let en = Localizer::new(Some("en"));
    let text = err.localize(&fr);
    assert_that!(text.contains("/a.md")).is_true();
    assert_that!(text).is_not_equal_to(err.localize(&en));
    assert_that!(text.contains("plusieurs fois")).is_true();
}

/// Each reason has its own text in every locale, and the wrappers use their inner key.
#[test]
fn every_knowledge_reason_key_exists_in_every_locale() {
    let reasons = [
        KnowledgeReason::NotFound,
        KnowledgeReason::Exists,
        KnowledgeReason::IdMissing,
        KnowledgeReason::IdInvalid,
        KnowledgeReason::IdTaken,
        KnowledgeReason::IdMismatch,
        KnowledgeReason::EditEmptyOld,
        KnowledgeReason::EditNoMatch,
        KnowledgeReason::EditAmbiguous,
        KnowledgeReason::TooManyFiles,
        KnowledgeReason::FileTooLarge,
        KnowledgeReason::TotalTooLarge,
    ];
    let en = Localizer::new(Some("en"));
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        assert!(l.has("op-knowledge"), "{locale}: op-knowledge absent");
        for reason in &reasons {
            assert!(l.has(reason.key()), "{locale}: {} absent", reason.key());
            if locale != "en" {
                assert_ne!(l.tr(reason.key()), en.tr(reason.key()), "{locale}: {} is English", reason.key());
            }
        }
    }
    assert_that!(KnowledgeReason::Path(PathReason::Empty).key()).is_equal_to("knowledge-path-empty");
    let fm = KnowledgeReason::Frontmatter(FrontmatterReason::Missing);
    assert_that!(fm.key()).is_equal_to("knowledge-frontmatter-missing");
}
