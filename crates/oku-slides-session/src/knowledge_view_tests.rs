use std::fmt::Write as _;

use oku_slides_core::asset_id;
use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use crate::knowledge_fixture::{approve_knowledge as approve, seed_knowledge};
use crate::test_support::{fresh_dir, logged, session_on, session_with, settings_on, white_png};
use crate::{Session, SessionErrorKind};

const ROADMAP: &str = "/references/roadmap.md";
const OLD: &str = "/references/old.md";
const SECRET: &str = "/notes/secret.md";

/// The hex of `asset_id(bytes)`, as cited in a bundle path.
fn hex_of(bytes: &[u8]) -> String {
    asset_id(bytes).trim_start_matches("sha256:").to_owned()
}

fn roadmap_text(image: &[u8]) -> String {
    let (present, absent) = (hex_of(image), hex_of(b"not in the deck"));
    format!(
        "---\ntype: Note\ntitle: Roadmap\nstatus: draft\nstale: true\n---\nThe ROADMAP for Q1\nnothing\nroadmap again\n\
         ![chart](assets/{present}.png)\n![gone](/references/assets/{absent}.png)\n"
    )
}

const OLD_TEXT: &str = "---\ntype: Note\nstale: yes\n---\nold notes\n";
const SECRET_TEXT: &str = "---\ntype: Note\n---\nroadmap secret\n";

/// A session whose deck holds roadmap and old (approved) and secret (awaiting), with the image.
fn three_files() -> (tempfile::TempDir, Session) {
    let data = fresh_dir("oku-slides-knowledge-view-");
    let image = white_png(3, 2);
    let roadmap = roadmap_text(&image);
    approve(data.path(), &[&roadmap, OLD_TEXT]);
    let mut s = session_on(data.path());
    seed_knowledge(&mut s, &[(SECRET, SECRET_TEXT), (ROADMAP, &roadmap), (OLD, OLD_TEXT)], vec![image]);
    (data, s)
}

/// A session whose deck holds `path` with `text`, approved.
fn one_approved(path: &str, text: &str) -> (tempfile::TempDir, Session) {
    let data = fresh_dir("oku-slides-knowledge-view-");
    approve(data.path(), &[text]);
    let mut s = session_on(data.path());
    seed_knowledge(&mut s, &[(path, text)], Vec::new());
    (data, s)
}

fn kind_of<T: std::fmt::Debug>(r: crate::Result<T>) -> SessionErrorKind {
    r.unwrap_err().kind()
}

#[test]
fn knowledge_entries_all_files_sorted_awaiting_shows_only_its_path() {
    let (_data, s) = three_files();
    let entries = s.knowledge_entries(None).unwrap();
    let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
    assert_that!(paths).is_equal_to(vec![SECRET, OLD, ROADMAP]);
    let secret = &entries[0];
    assert_that!(secret.approved).is_false();
    assert_that!((&secret.kind, &secret.title, &secret.description, &secret.status, secret.stale))
        .is_equal_to((&None, &None, &None, &None, false));
    let roadmap = &entries[2];
    assert_that!(roadmap.approved).is_true();
    assert_that!(roadmap.kind.as_deref()).is_equal_to(Some("Note"));
    assert_that!(roadmap.title.as_deref()).is_equal_to(Some("Roadmap"));
}

#[test]
fn knowledge_entries_frontmatter_gives_status_and_plain_true_stale_only() {
    let (_data, s) = three_files();
    let entries = s.knowledge_entries(None).unwrap();
    let (old, roadmap) = (&entries[1], &entries[2]);
    assert_that!((roadmap.status.as_deref(), roadmap.stale)).is_equal_to((Some("draft"), true));
    assert_that!((old.status.as_deref(), old.stale)).is_equal_to((None, false));
}

#[rstest]
#[case::bare("/references")]
#[case::trailing_slash("/references/")]
fn knowledge_entries_under_dir_lists_its_subtree_only(#[case] dir: &str) {
    let (_data, s) = three_files();
    let paths: Vec<String> = s.knowledge_entries(Some(dir)).unwrap().into_iter().map(|e| e.path).collect();
    assert_that!(paths).is_equal_to(vec![OLD.to_owned(), ROADMAP.to_owned()]);
}

#[rstest]
#[case::relative("refs")]
#[case::dot_segment("/references/..")]
fn knowledge_entries_malformed_dir_is_refused(#[case] dir: &str) {
    let (_data, s) = three_files();
    let err = s.knowledge_entries(Some(dir)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeDirInvalid);
    let fr = err.localize(&Localizer::new(Some("fr")));
    assert_that!(fr.as_str()).contains("Dossier de connaissances invalide");
    assert_that!(fr.as_str()).contains(dir);
}

#[test]
fn knowledge_file_approved_gives_text_and_cited_images() {
    let (_data, s) = three_files();
    let image = white_png(3, 2);
    let read = s.knowledge_file(ROADMAP).unwrap();
    assert_that!(read.text).is_equal_to(roadmap_text(&image));
    assert_that!(read.truncated).is_equal_to(0);
    let present = read.images.iter().find(|i| i.asset == asset_id(&image)).unwrap();
    assert_that!(present.path).is_equal_to(format!("/references/assets/{}.png", hex_of(&image)));
    assert_that!((present.mime, present.width, present.height)).is_equal_to(("image/png", 3, 2));
    assert_that!(present.bytes.as_deref()).is_equal_to(Some(image.as_slice()));
    let absent = read.images.iter().find(|i| i.asset == asset_id(b"not in the deck")).unwrap();
    assert_that!((absent.bytes.is_none(), absent.width, absent.height)).is_equal_to((true, 0, 0));
    assert_that!(read.images).has_length(2);
}

#[test]
fn knowledge_file_unapproved_is_awaiting_and_localized() {
    let (_data, s) = three_files();
    let err = s.knowledge_file(SECRET).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeAwaiting);
    let fr = err.localize(&Localizer::new(Some("fr")));
    assert_that!(fr.as_str()).contains(SECRET);
    assert_that!(fr.as_str()).contains("approbation");
}

#[test]
fn knowledge_file_unknown_path_is_not_found() {
    let (_data, s) = three_files();
    let err = s.knowledge_file("/references/none.md").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeNotFound);
    let fr = err.localize(&Localizer::new(Some("fr")));
    assert_that!(fr.as_str()).contains("Aucun fichier de connaissances");
    assert_that!(fr.as_str()).contains("/references/none.md");
}

#[test]
fn knowledge_file_malformed_path_is_invalid_with_its_reason() {
    let (_data, s) = three_files();
    let err = s.knowledge_file("../x").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgePathInvalid);
    let fr = err.localize(&Localizer::new(Some("fr")));
    assert_that!(fr.as_str()).contains("Chemin de connaissances invalide");
    assert_that!(fr.as_str()).contains(Localizer::new(Some("fr")).tr("knowledge-path-not-absolute").as_str());
}

#[test]
fn knowledge_file_over_one_mib_reads_one_mib_and_counts_the_rest() {
    let mut text = String::from("---\ntype: Note\n---\n");
    text.push_str(&"x".repeat((1 << 20) + 10 - text.len()));
    let (_data, s) = one_approved("/big.md", &text);
    let read = s.knowledge_file("/big.md").unwrap();
    assert_that!((read.text.len(), read.truncated)).is_equal_to((1 << 20, 10));
}

#[test]
fn knowledge_file_cut_lands_on_a_char_boundary() {
    let mut text = String::from("---\ntype: Note\n---\n");
    text.push_str(&"x".repeat((1 << 20) - 1 - text.len()));
    text.push_str("éé");
    let (_data, s) = one_approved("/big.md", &text);
    let read = s.knowledge_file("/big.md").unwrap();
    assert_that!((read.text.len(), read.truncated)).is_equal_to(((1 << 20) - 1, 4));
}

/// A text of 1 MiB of filler, then `tail`: the read stops before `tail`.
fn past_the_cut(tail: &str) -> String {
    let mut text = String::from("---\ntype: Note\n---\n");
    text.push_str(&"x".repeat((1 << 20) - text.len()));
    text.push('\n');
    text.push_str(tail);
    text
}

#[test]
fn knowledge_search_match_after_the_cut_is_not_found() {
    let (_data, s) = one_approved("/big.md", &past_the_cut("needle\n"));
    let found = s.knowledge_search("needle").unwrap();
    assert_that!((found.hits.len(), found.more)).is_equal_to((0, 0));
}

#[test]
fn knowledge_file_image_cited_after_the_cut_is_not_listed() {
    let image = white_png(3, 2);
    let (_data, s) =
        one_approved("/big.md", &past_the_cut(&format!("![late](/references/assets/{}.png)\n", hex_of(&image))));
    let read = s.knowledge_file("/big.md").unwrap();
    assert_that!(read.images).is_empty();
}

/// Review focus: an approved text edited by one character awaits again, in every view.
#[test]
fn knowledge_views_edited_approved_file_awaits_again_everywhere() {
    let (_data, mut s) = three_files();
    let edited = roadmap_text(&white_png(3, 2)).replace("for Q1", "for Q2");
    seed_knowledge(&mut s, &[(SECRET, SECRET_TEXT), (ROADMAP, &edited), (OLD, OLD_TEXT)], Vec::new());
    let entries = s.knowledge_entries(None).unwrap();
    let roadmap = entries.iter().find(|e| e.path == ROADMAP).unwrap();
    assert_that!((roadmap.approved, roadmap.title.is_none())).is_equal_to((false, true));
    assert_that!(kind_of(s.knowledge_file(ROADMAP))).is_equal_to(SessionErrorKind::KnowledgeAwaiting);
    assert_that!(s.knowledge_search("roadmap").unwrap().hits).is_empty();
}

#[test]
fn knowledge_search_matches_approved_lines_one_based_ignoring_case() {
    let (_data, s) = three_files();
    let found = s.knowledge_search("ROADMAP").unwrap();
    let hits: Vec<(&str, usize)> = found.hits.iter().map(|h| (h.path.as_str(), h.line)).collect();
    assert_that!(hits).is_equal_to(vec![(ROADMAP, 3), (ROADMAP, 7), (ROADMAP, 9)]);
    assert_that!(found.hits[1].text.as_str()).is_equal_to("The ROADMAP for Q1");
    assert_that!(found.more).is_equal_to(0);
}

#[test]
fn knowledge_search_ten_thousand_matches_keeps_fifty_and_counts_the_rest() {
    let mut text = String::from("---\ntype: Note\n---\n");
    for n in 0..10_000 {
        writeln!(text, "match {n:05} {}", "y".repeat(80)).unwrap();
    }
    // Every line within the 1 MiB the search reads.
    assert_that!(text.len()).is_less_than(1 << 20);
    let (_data, s) = one_approved("/many.md", &text);
    let found = s.knowledge_search("match").unwrap();
    assert_that!((found.hits.len(), found.more)).is_equal_to((50, 9950));
    assert_that!(found.hits[0].line).is_equal_to(4);
    assert_that!(found.hits[0].text.chars().count()).is_less_than_or_equal_to(200);
}

#[rstest]
#[case::empty(String::new())]
#[case::blank("   ".to_owned())]
#[case::too_long("a".repeat(201))]
#[case::accent_only("\u{301}".to_owned())]
#[case::accents_and_spaces(" \u{301}\u{300} ".to_owned())]
fn knowledge_search_bad_query_is_refused(#[case] query: String) {
    let (_data, s) = three_files();
    let err = s.knowledge_search(&query).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeQueryInvalid);
    assert_that!(err.localize(&Localizer::new(Some("fr"))).as_str()).contains("200");
}

#[test]
fn knowledge_search_query_of_200_chars_is_accepted() {
    let (_data, s) = three_files();
    assert_that!(s.knowledge_search(&"a".repeat(200)).unwrap().hits).is_empty();
}

#[test]
fn knowledge_views_without_approvals_file_everything_awaits() {
    let data = fresh_dir("oku-slides-knowledge-view-");
    let mut s = session_on(data.path());
    seed_knowledge(&mut s, &[(OLD, OLD_TEXT), (SECRET, SECRET_TEXT)], Vec::new());
    assert_that!(s.knowledge_entries(None).unwrap().iter().any(|e| e.approved)).is_false();
    assert_that!(kind_of(s.knowledge_file(OLD))).is_equal_to(SessionErrorKind::KnowledgeAwaiting);
    assert_that!(s.knowledge_search("notes").unwrap().hits).is_empty();
}

#[test]
fn knowledge_settings_above_ceiling_apply_the_ceiling_and_log() {
    let data = fresh_dir("oku-slides-knowledge-view-");
    let mut settings = settings_on(data.path());
    settings.knowledge.index_entries = std::num::NonZeroUsize::new(5000).unwrap();
    let mut s = None;
    let log = logged(|| s = Some(session_with(settings)));
    let s = s.unwrap();
    assert_that!(s.knowledge_settings.effective_index_entries()).is_equal_to(1024);
    assert_that!(log.as_str()).contains("knowledge setting above its ceiling");
}

#[test]
fn knowledge_search_hits_setting_lowers_the_hit_count() {
    let data = fresh_dir("oku-slides-knowledge-view-");
    let image = white_png(3, 2);
    let roadmap = roadmap_text(&image);
    approve(data.path(), &[&roadmap]);
    let mut settings = settings_on(data.path());
    settings.knowledge.search_hits = std::num::NonZeroUsize::new(1).unwrap();
    let mut s = session_with(settings);
    seed_knowledge(&mut s, &[(ROADMAP, &roadmap)], Vec::new());
    let found = s.knowledge_search("roadmap").unwrap();
    assert_that!((found.hits.len(), found.more)).is_equal_to((1, 2));
}

#[test]
fn knowledge_entries_approved_log_without_frontmatter_is_listed_without_fields() {
    let text = "- 2026-11-25 started the roadmap\n";
    let (_data, s) = one_approved("/references/log.md", text);
    let entries = s.knowledge_entries(None).unwrap();
    assert_that!(entries).is_equal_to(vec![crate::KnowledgeEntry {
        path: "/references/log.md".to_owned(),
        approved: true,
        kind: None,
        title: None,
        description: None,
        status: None,
        stale: false,
    }]);
    assert_that!(s.knowledge_file("/references/log.md").unwrap().text.as_str()).is_equal_to(text);
}

#[test]
fn knowledge_settings_above_ceiling_log_once_per_process() {
    let data = fresh_dir("oku-slides-knowledge-view-");
    let mut settings = settings_on(data.path());
    settings.knowledge.search_hits = std::num::NonZeroUsize::new(500).unwrap();
    let first = logged(|| drop(session_with(settings.clone())));
    let second = logged(|| drop(session_with(settings)));
    assert_that!(first.as_str()).contains("knowledge setting above its ceiling");
    assert_that!(second.as_str()).does_not_contain("knowledge setting above its ceiling");
}
