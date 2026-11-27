use rstest::rstest;
use speculoos::prelude::*;
use time::macros::datetime;

use super::*;
use crate::knowledge::frontmatter::parse_block;
use crate::knowledge::{check_text, frontmatter, split};

const ID: [u8; 8] = [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef];

fn by() -> String {
    format!("oku-slides/{}", env!("CARGO_PKG_VERSION"))
}

fn provenance(agent: Option<&str>) -> Provenance {
    Provenance::new(by(), datetime!(2026-11-25 10:20:30.456 UTC), ID, agent.map(str::to_owned))
}

fn path(text: &str) -> KnowledgePath {
    KnowledgePath::parse(text).unwrap()
}

fn block() -> String {
    format!("generated:\n  by: \"{}\"\n  at: \"2026-11-25T10:20:30.456Z\"\n  id: \"0123456789abcdef\"\n", by())
}

fn stamped(file: &str, text: &str, agent: Option<&str>) -> String {
    stamp(&path(file), text, &provenance(agent)).unwrap()
}

#[test]
fn stamp_concept_adds_generated_keeps_other_bytes() {
    let text = "\u{feff}---\n# lead comment\ntype: Note\ntitle: \"Prix # été\"\ntags:\n  - a\n# trailing\n---\nbody\n";
    let expected = format!(
        "\u{feff}---\n# lead comment\ntype: Note\ntitle: \"Prix # été\"\ntags:\n  - a\n# trailing\n{}x-oku-slides-agent: \"claude\"\n---\nbody\n",
        block()
    );
    assert_that!(stamped("/a.md", text, Some("claude"))).is_equal_to(expected);
}

#[rstest]
#[case::nested("type: Note\ngenerated:\n  by: someone\n  at: 2020-01-01T00:00:00Z\ntitle: T\n")]
#[case::inline("type: Note\ngenerated: {by: x}\ntitle: T\n")]
#[case::quoted("type: Note\n\"generated\":\n  at: 2020-01-01T00:00:00Z\ntitle: T\n")]
fn stamp_existing_generated_replaced_once(#[case] yaml: &str) {
    let text = format!("---\n{yaml}---\nbody\n");
    let out = stamped("/a.md", &text, None);
    assert_that!(out).is_equal_to(format!("---\ntype: Note\ntitle: T\n{}---\nbody\n", block()));
    assert_that!(stamped("/a.md", &out, None)).is_equal_to(out);
}

#[test]
fn stamp_key_with_column0_sequence_keeps_items() {
    let text = "---\ntype: Note\ntags:\n- a\n- b\ngenerated:\n- stale\n---\n";
    let out = stamped("/a.md", text, None);
    assert_that!(out).is_equal_to(format!("---\ntype: Note\ntags:\n- a\n- b\n{}---\n", block()));
}

#[test]
fn stamp_without_agent_removes_stale_agent_key() {
    let text = "---\ntype: Note\nx-oku-slides-agent: \"old\"\n---\n";
    assert_that!(stamped("/a.md", text, None)).is_equal_to(format!("---\ntype: Note\n{}---\n", block()));
}

#[test]
fn stamp_agent_label_with_quote_and_newline_reads_back() {
    let label = "a \"b\" \\ c\nd\u{7f}\u{2028}";
    let out = stamped("/a.md", "---\ntype: Note\n---\n", Some(label));
    assert_that!(frontmatter(&out).map(|f| f.keys)).is_ok_containing(vec![
        "type".to_owned(),
        "generated".to_owned(),
        "x-oku-slides-agent".to_owned(),
    ]);
    let (yaml, _) = split(&out).unwrap();
    let entries = parse_block(yaml).unwrap();
    let read = entries.iter().find(|(k, _)| k == "x-oku-slides-agent").and_then(|(_, v)| v.text());
    assert_that!(read).is_equal_to(Some(label));
}

#[rstest]
#[case::flow_root("---\n{type: Note}\n---\n")]
#[case::complex_key("---\ntype: Note\n? x\n: y\n---\n")]
fn stamp_flow_mapping_refused(#[case] text: &str) {
    assert_that!(stamp(&path("/a.md"), text, &provenance(None))).is_err_containing(FrontmatterReason::FlowMapping);
}

#[test]
fn stamp_document_end_marker_refused() {
    let text = "---\ntype: Note\n...\n---\n";
    assert_that!(stamp(&path("/a.md"), text, &provenance(None))).is_err_containing(FrontmatterReason::FlowMapping);
}

#[test]
fn stamp_concept_without_frontmatter_unchanged() {
    assert_that!(stamped("/a.md", "just text\n", None)).is_equal_to("just text\n".to_owned());
}

#[test]
fn stamp_log_twice_keeps_one_comment() {
    let once = stamped("/log.md", "# Log\n- one\n", None);
    let twice = stamped("/log.md", &once, None);
    let comment = "<!-- oku-slides generated 2026-11-25T10:20:30.456Z 0123456789abcdef -->";
    assert_that!(twice).is_equal_to(format!("# Log\n- one\n{comment}\n"));
}

#[test]
fn stamp_index_removes_only_prefixed_comments() {
    let text = "# Index\n<!-- keep me -->\n<!-- oku-slides generated old 00 -->\n- [a](a.md)";
    let comment = "<!-- oku-slides generated 2026-11-25T10:20:30.456Z 0123456789abcdef -->";
    assert_that!(stamped("/x/index.md", text, None))
        .is_equal_to(format!("# Index\n<!-- keep me -->\n- [a](a.md)\n{comment}\n"));
}

#[rstest]
#[case::concept("/a.md", "---\ntype: Note\ntitle: T\n---\nbody\n")]
#[case::root_index("/index.md", "---\nokf_version: \"0.1\"\ntitle: T\n---\n# T\n")]
#[case::log("/log.md", "# Log\n")]
#[case::crlf("/a.md", "---\r\ntype: Note\r\ngenerated:\r\n  at: 2020-01-01T00:00:00Z\r\n---\r\nbody\r\n")]
fn stamp_any_file_output_passes_check_text(#[case] file: &str, #[case] text: &str) {
    let out = stamped(file, text, Some("agent"));
    assert_that!(check_text(&path(file), &out)).is_ok();
}

#[test]
fn stamp_crlf_text_keeps_crlf() {
    let out = stamped("/a.md", "---\r\ntype: Note\r\n---\r\n", None);
    assert_that!(out.matches('\n').count()).is_equal_to(out.matches("\r\n").count());
}

#[rstest]
#[case::hundred(datetime!(2026-01-02 03:04:05.1 UTC), "2026-01-02T03:04:05.100Z")]
#[case::zero(datetime!(2026-01-02 03:04:05 +02:00), "2026-01-02T01:04:05.000Z")]
fn stamp_any_offset_at_has_three_fraction_digits(#[case] at: OffsetDateTime, #[case] text: &str) {
    let p = Provenance::new(by(), at, ID, None);
    let out = stamp(&path("/log.md"), "", &p).unwrap();
    assert_that!(out).contains(format!(" {text} "));
}

#[test]
fn stamp_distinct_ids_differ_in_text() {
    let other = Provenance::new(by(), datetime!(2026-11-25 10:20:30.456 UTC), [0; 8], None);
    let text = "---\ntype: Note\n---\n";
    assert_that!(stamp(&path("/a.md"), text, &other).unwrap()).is_not_equal_to(stamped("/a.md", text, None));
}

#[rstest]
#[case::concept("/a.md", "\u{feff}---\ntype: Note\ntags:\n- a\n---\nbody\n")]
#[case::log("/log.md", "# Log\n- one\n")]
fn unstamped_stamped_text_gives_back_original(#[case] file: &str, #[case] text: &str) {
    assert_that!(unstamped(&stamped(file, text, Some("agent")))).is_equal_to(text.to_owned());
}

#[test]
fn unstamped_without_keys_drops_the_stamp_and_the_keys_only() {
    let text = "---\ntype: Note\nx-mark: {part: 1}\ntitle: T\n---\n<!-- oku-slides generated kept -->\nbody\n";
    let kept = "---\ntype: Note\ntitle: T\n---\n<!-- oku-slides generated kept -->\nbody\n";
    assert_that!(unstamped_without_keys(&stamped("/a.md", text, Some("agent")), &["x-mark"]))
        .is_equal_to(kept.to_owned());
}

#[test]
fn unstamped_without_keys_sees_a_frontmatter_edit() {
    let text = "---\ntype: Note\nx-mark: {part: 1}\n---\nbody\n";
    let edited = text.replace("type: Note", "type: Note\ntags: [a]");
    assert_that!(unstamped_without_keys(&stamped("/a.md", &edited, None), &["x-mark"]))
        .is_not_equal_to(unstamped_without_keys(&stamped("/a.md", text, None), &["x-mark"]));
}
