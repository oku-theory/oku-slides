use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::error::FrontmatterReason;
use crate::knowledge::limits::MAX_HEADER_ITEMS;

fn doc(yaml: &str) -> String {
    format!("---\n{yaml}\n---\nbody\n")
}

fn header(yaml: &str) -> KnowledgeHeader {
    knowledge_header(&doc(yaml)).unwrap()
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn knowledge_header_full_block_reads_every_field() {
    let h = header(
        "type: note\ntitle: T\ndescription: D\nstatus: draft\ntags: [a, b]\n\
         generated: {by: x, at: 2026-11-27T10:00:00Z}\nverified: {tier: gold}\nstale: true\n\
         sources:\n  - {resource: r, title: st, url: 'https://e.org'}\n  - plain",
    );
    assert_that!(h).is_equal_to(KnowledgeHeader {
        kind: Some("note".into()),
        title: Some("T".into()),
        description: Some("D".into()),
        status: Some("draft".into()),
        stale: true,
        tags: strings(&["a", "b"]),
        generated_by: Some("x".into()),
        generated_at: Some("2026-11-27T10:00:00Z".into()),
        verified: Some("gold".into()),
        sources: vec![
            KnowledgeSource { resource: Some("r".into()), title: Some("st".into()), url: Some("https://e.org".into()) },
            KnowledgeSource { resource: Some("plain".into()), title: None, url: None },
        ],
    });
}

#[test]
fn knowledge_header_scalar_verified_read() {
    assert_that!(header("verified: gold").verified).is_equal_to(Some("gold".to_owned()));
}

#[test]
fn knowledge_header_lone_scalar_tags_gives_one_tag() {
    assert_that!(header("tags: solo").tags).is_equal_to(strings(&["solo"]));
}

#[test]
fn knowledge_header_nested_tag_item_skipped() {
    assert_that!(header("tags: [a, {b: c}, [d]]").tags).is_equal_to(strings(&["a"]));
}

#[test]
fn knowledge_header_empty_tags_empty() {
    assert_that!(header("tags: []").tags).is_empty();
}

#[test]
fn knowledge_header_non_bool_stale_false() {
    assert_that!(header("stale: \"yes\"").stale).is_false();
}

#[test]
fn knowledge_header_items_capped_at_twenty() {
    let tags = (0..25).map(|i| format!("t{i}")).collect::<Vec<_>>().join(", ");
    let sources = (0..25).map(|i| format!("s{i}")).collect::<Vec<_>>().join(", ");
    let h = header(&format!("tags: [{tags}]\nsources: [{sources}]"));
    assert_that!(h.tags.len()).is_equal_to(MAX_HEADER_ITEMS);
    assert_that!(h.sources.len()).is_equal_to(MAX_HEADER_ITEMS);
}

#[test]
fn knowledge_header_source_without_fields_dropped() {
    assert_that!(header("sources: [{x: 1}]").sources).is_empty();
}

#[test]
fn knowledge_header_alias_refused() {
    let result = knowledge_header(&doc("a: &x 1\nb: *x"));
    assert_that!(result).is_equal_to(Err(FrontmatterReason::Alias));
}

#[test]
fn knowledge_header_bom_and_crlf_read() {
    let h = knowledge_header("\u{feff}---\r\ntype: note\r\ntags: [a]\r\n---\r\nbody").unwrap();
    assert_that!(h.kind).is_equal_to(Some("note".to_owned()));
    assert_that!(h.tags).is_equal_to(strings(&["a"]));
}

#[rstest]
#[case::no_frontmatter("just text\n---\nmore", "just text\n---\nmore")]
#[case::after_fence("---\ntype: a\n---\nrest\n", "rest\n")]
#[case::unterminated("---\ntype: a\nrest", "---\ntype: a\nrest")]
#[case::later_rule_kept("---\ntype: a\n---\nx\n---\ny\n", "x\n---\ny\n")]
fn body_text_cases_frontmatter_cut(#[case] text: &str, #[case] expected: &str) {
    assert_that!(body(text)).is_equal_to(expected);
}
