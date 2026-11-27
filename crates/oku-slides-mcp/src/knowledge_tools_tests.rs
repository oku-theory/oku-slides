use oku_slides_session::KnowledgeEntry;
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::{KnowledgeImportArgs, ListedFile, search_description};

fn entry(approved: bool) -> KnowledgeEntry {
    KnowledgeEntry {
        path: "/a.md".into(),
        approved,
        kind: Some("Note".into()),
        title: Some("T".into()),
        description: None,
        status: None,
        stale: true,
    }
}

#[test]
fn search_description_fills_the_hit_cap() {
    assert_that!(search_description(42).as_str()).contains("At most 42 hits; more counts the others.");
    assert_that!(search_description(42).contains('{')).is_false();
}

#[test]
fn listed_file_awaiting_tells_only_its_path() {
    let v = serde_json::to_value(ListedFile::from(entry(false))).unwrap_or_default();
    assert_that!(v).is_equal_to(json!({ "path": "/a.md", "approval": "awaiting" }));
}

#[test]
fn listed_file_approved_names_type_and_skips_absent_fields() {
    let v = serde_json::to_value(ListedFile::from(entry(true))).unwrap_or_default();
    assert_that!(v)
        .is_equal_to(json!({ "path": "/a.md", "type": "Note", "title": "T", "stale": true, "approval": "approved" }));
}

/// The frontmatter values are untrusted: listed as the agent's index shows them.
#[test]
fn listed_file_approved_values_are_cleaned_and_cut_as_in_the_index() {
    let hostile = KnowledgeEntry {
        title: Some(format!("a\u{202E}b\n{}", "t".repeat(100))),
        description: Some("d".repeat(250)),
        status: Some("s".repeat(50)),
        ..entry(true)
    };
    let listed = ListedFile::from(hostile);
    let (title, description, status) = (listed.title.unwrap_or_default(), listed.description, listed.status);
    assert_that!(title.contains(['\u{202E}', '\n'])).is_false();
    assert_that!(title.chars().count()).is_equal_to(80);
    assert_that!(title.starts_with("ab t")).is_true();
    assert_that!(description.map(|d| d.chars().count())).is_equal_to(Some(200));
    assert_that!(status.map(|s| s.chars().count())).is_equal_to(Some(40));
}

#[test]
fn knowledge_import_args_path_is_optional() {
    let args: KnowledgeImportArgs = serde_json::from_value(json!({ "attachment": "doc-1" })).unwrap();
    assert_that!(args.attachment.as_str()).is_equal_to("doc-1");
    assert_that!(args.path).is_none();
    let args: KnowledgeImportArgs =
        serde_json::from_value(json!({ "attachment": "doc-1", "path": "/references/a.md" })).unwrap();
    assert_that!(args.path).is_equal_to(Some("/references/a.md".to_owned()));
}

#[rstest]
#[case::unknown_field(json!({ "attachment": "doc-1", "replace": true }))]
#[case::attachment_not_a_string(json!({ "attachment": 7 }))]
#[case::path_not_a_string(json!({ "attachment": "doc-1", "path": ["/references/a.md"] }))]
fn knowledge_import_args_malformed_are_refused(#[case] args: serde_json::Value) {
    assert_that!(serde_json::from_value::<KnowledgeImportArgs>(args)).is_err();
}

#[test]
fn knowledge_import_args_without_attachment_are_refused() {
    assert_that!(serde_json::from_value::<KnowledgeImportArgs>(json!({ "path": "/references/a.md" }))).is_err();
}
