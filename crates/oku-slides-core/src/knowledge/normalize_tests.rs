use std::collections::BTreeMap;

use proptest::prelude::*;
use speculoos::prelude::*;

use super::*;
use crate::error::{FrontmatterReason, KnowledgeLimit, PathReason};

const NOTE: &str = "---\ntype: Note\n---\nbody\n";

fn id(n: u8) -> String {
    format!("{n:032x}")
}

fn file(path: &str, text: &str) -> KnowledgeFile {
    KnowledgeFile { path: path.to_owned(), text: text.to_owned() }
}

fn bundle(entries: &[(String, KnowledgeFile)]) -> BTreeMap<String, KnowledgeFile> {
    entries.iter().cloned().collect()
}

fn paths(files: &BTreeMap<String, KnowledgeFile>) -> Vec<(String, String)> {
    files.iter().map(|(id, f)| (id.clone(), f.path.clone())).collect()
}

fn keys(issues: &[Issue]) -> Vec<&'static str> {
    issues.iter().map(|i| i.message.id()).collect()
}

#[test]
fn normalize_valid_bundle_is_unchanged() {
    let input = bundle(&[(id(1), file("/a.md", NOTE)), (id(2), file("/log.md", ""))]);
    let (out, issues) = normalize(input.clone());
    assert_that!(out).is_equal_to(input);
    assert_that!(issues).is_empty();
}

#[test]
fn normalize_bad_id_is_dropped() {
    let input = bundle(&[("XYZ".to_owned(), file("/a.md", NOTE)), (id(1), file("/b.md", NOTE))]);
    let (out, issues) = normalize(input);
    assert_that!(out.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec![id(1)]);
    assert_that!(issues).is_equal_to(vec![Issue::knowledge_unreadable("XYZ")]);
}

#[test]
fn normalize_oversized_file_is_kept_and_reported() {
    let big = format!("{NOTE}{}", "x".repeat(MAX_FILE_BYTES));
    let input = bundle(&[(id(1), file("/a.md", &big)), (id(2), file("/b.md", NOTE)), (id(3), file("/c.md", &big))]);
    let (out, issues) = normalize(input.clone());
    assert_that!(out).is_equal_to(&input);
    assert_that!(issues).is_equal_to(vec![Issue::knowledge_over_limit(KnowledgeLimit::FileTooLarge, &id(1), 2)]);
}

#[test]
fn normalize_files_past_count_cap_are_kept_with_one_issue() {
    let input: BTreeMap<_, _> =
        (0..=MAX_FILES + 1).map(|n| (format!("{n:032x}"), file(&format!("/f{n}.md"), NOTE))).collect();
    let (out, issues) = normalize(input.clone());
    assert_that!(out).is_equal_to(&input);
    let first = format!("{MAX_FILES:032x}");
    assert_that!(issues).is_equal_to(vec![Issue::knowledge_over_limit(KnowledgeLimit::TooManyFiles, &first, 2)]);
    // Still idempotent, and the issue stays as long as the bundle is past the cap.
    let (again, issues_again) = normalize(out.clone());
    assert_that!(again).is_equal_to(&out);
    assert_that!(issues_again).is_equal_to(&issues);
}

#[test]
fn normalize_files_past_total_cap_are_kept_with_one_issue() {
    let text = format!("{NOTE}{}", "x".repeat(MAX_FILE_BYTES - NOTE.len()));
    let fit = MAX_TOTAL_BYTES / MAX_FILE_BYTES;
    let input: BTreeMap<_, _> =
        (0..fit + 2).map(|n| (format!("{n:032x}"), file(&format!("/f{n}.md"), &text))).collect();
    let (out, issues) = normalize(input.clone());
    assert_that!(out).is_equal_to(&input);
    assert_that!(issues).is_equal_to(vec![Issue::knowledge_over_limit(
        KnowledgeLimit::TotalTooLarge,
        &format!("{fit:032x}"),
        2,
    )]);
}

#[test]
fn repair_does_not_read_texts_nor_ceilings() {
    let big = format!("{NOTE}{}", "x".repeat(MAX_FILE_BYTES));
    let input = bundle(&[(id(1), file("/a.md", "no frontmatter")), (id(2), file("/b.md", &big))]);
    let (out, issues) = repair(input.clone());
    assert_that!(out).is_equal_to(input);
    assert_that!(issues).is_empty();
}

#[test]
fn repair_of_an_empty_bundle_is_empty() {
    let (out, issues) = repair(BTreeMap::new());
    assert_that!(out.is_empty()).is_true();
    assert_that!(issues).is_empty();
}

#[test]
fn every_readable_file_is_placed_even_when_collisions_fill_the_suffixes() {
    // Every claimant of `/a.md` and every suffix it could take: all still get a place.
    let mut entries = vec![(id(0), file("/a.md", NOTE))];
    entries.extend((1..20).map(|n| (id(n), file(&format!("/a-{}.md", n + 1), NOTE))));
    entries.extend((20..40).map(|n| (id(n), file("/a.md", NOTE))));
    let input = bundle(&entries);
    let (out, _) = normalize(input.clone());
    assert_that!(out.keys().collect::<Vec<_>>()).is_equal_to(input.keys().collect::<Vec<_>>());
    let distinct: std::collections::BTreeSet<_> = out.values().map(|f| f.path.clone()).collect();
    assert_that!(distinct.len()).is_equal_to(input.len());
}

#[test]
fn normalize_invalid_path_is_recovered() {
    let input = bundle(&[(id(1), file("/../x.md", NOTE)), (id(2), file("/a.png", NOTE))]);
    let (out, issues) = normalize(input);
    assert_that!(paths(&out))
        .is_equal_to(vec![(id(1), format!("/recovered/{}.md", id(1))), (id(2), format!("/recovered/{}.md", id(2)))]);
    assert_that!(issues).is_equal_to(vec![
        Issue::knowledge_path_invalid(&id(1), PathReason::DotSegment),
        Issue::knowledge_path_invalid(&id(2), PathReason::NotMarkdown),
    ]);
}

#[test]
fn normalize_collision_lowest_id_keeps_path() {
    let input = bundle(&[(id(2), file("/a.md", NOTE)), (id(1), file("/a.md", NOTE))]);
    let (out, issues) = normalize(input);
    assert_that!(paths(&out)).is_equal_to(vec![(id(1), "/a.md".to_owned()), (id(2), "/a-2.md".to_owned())]);
    assert_that!(issues).is_equal_to(vec![Issue::knowledge_path_collision("/a.md")]);
}

#[test]
fn normalize_collision_skips_taken_suffix() {
    let input = bundle(&[(id(1), file("/a.md", NOTE)), (id(2), file("/a.md", NOTE)), (id(3), file("/a-2.md", NOTE))]);
    let (out, issues) = normalize(input);
    assert_that!(paths(&out)).is_equal_to(vec![
        (id(1), "/a.md".to_owned()),
        (id(2), "/a-3.md".to_owned()),
        (id(3), "/a-2.md".to_owned()),
    ]);
    assert_that!(keys(&issues)).is_equal_to(vec!["issue-knowledge-path-collision"]);
}

#[test]
fn normalize_collision_without_room_for_suffix_is_recovered() {
    // 254 bytes: `-2` would make 256, past the limit (253 + 2 still fits in 255).
    let long = format!("/{}.md", "a".repeat(250));
    assert_that!(long.len()).is_equal_to(254);
    let input = bundle(&[(id(1), file(&long, NOTE)), (id(2), file(&long, NOTE))]);
    let (out, issues) = normalize(input);
    assert_that!(paths(&out)).is_equal_to(vec![(id(1), long.clone()), (id(2), format!("/recovered/{}.md", id(2)))]);
    assert_that!(issues).is_equal_to(vec![Issue::knowledge_path_collision(&long)]);
}

#[test]
fn normalize_recovered_path_collision_is_suffixed() {
    let taken = format!("/recovered/{}.md", id(1));
    let input = bundle(&[(id(1), file("/con.md", NOTE)), (id(2), file(&taken, NOTE))]);
    let (out, issues) = normalize(input);
    assert_that!(paths(&out)).is_equal_to(vec![(id(1), format!("/recovered/{}-2.md", id(1))), (id(2), taken.clone())]);
    assert_that!(issues).is_equal_to(vec![
        Issue::knowledge_path_invalid(&id(1), PathReason::Reserved),
        Issue::knowledge_path_collision(&taken),
    ]);
}

#[test]
fn normalize_suffixed_index_becomes_invalid_concept() {
    let input = bundle(&[(id(1), file("/index.md", "# Home\n")), (id(2), file("/index.md", "# Other\n"))]);
    let (out, issues) = normalize(input);
    assert_that!(out.get(&id(2)).map(|f| f.path.as_str())).is_equal_to(Some("/index-2.md"));
    assert_that!(out.get(&id(2)).map(|f| f.text.as_str())).is_equal_to(Some("# Other\n"));
    assert_that!(issues).is_equal_to(vec![
        Issue::knowledge_path_collision("/index.md"),
        Issue::knowledge_invalid("/index-2.md", &FrontmatterReason::Missing),
    ]);
}

#[test]
fn normalize_invalid_text_is_kept() {
    let input = bundle(&[(id(1), file("/a.md", "no frontmatter"))]);
    let (out, issues) = normalize(input.clone());
    assert_that!(out).is_equal_to(input);
    assert_that!(issues).is_equal_to(vec![Issue::knowledge_invalid("/a.md", &FrontmatterReason::Missing)]);
}

fn arb_bundle() -> impl Strategy<Value = BTreeMap<String, KnowledgeFile>> {
    let ids = prop_oneof![8 => (0u8..12).prop_map(id), 1 => Just("XYZ".to_owned())];
    let paths = prop::sample::select(vec![
        "/a.md",
        "/a-2.md",
        "/b/c.md",
        "/index.md",
        "/log.md",
        "/../x.md",
        "/con.md",
        "/A.md",
        "/a.png",
        "",
    ]);
    let texts = prop::sample::select(vec![NOTE, "", "# plain\n", "---\ntype: [\n---\n"]);
    prop::collection::btree_map(ids, (paths, texts).prop_map(|(p, t)| file(p, t)), 0..10)
}

proptest! {
    #[test]
    fn repair_is_idempotent_and_agrees_with_normalize(input in arb_bundle()) {
        let (once, _) = repair(input.clone());
        let (twice, issues) = repair(once.clone());
        prop_assert_eq!(&twice, &once);
        prop_assert!(issues.is_empty());
        prop_assert_eq!(&normalize(input).0, &once);
    }

    #[test]
    fn normalize_is_idempotent(input in arb_bundle()) {
        let (once, _) = normalize(input);
        let (twice, issues) = normalize(once.clone());
        prop_assert_eq!(&twice, &once);
        prop_assert!(keys(&issues).iter().all(|k| *k == "issue-knowledge-invalid"));
        let all_valid = once.values().all(|f| {
            KnowledgePath::parse_writable(&f.path).is_ok_and(|p| check_text(&p, &f.text).is_ok())
        });
        if all_valid {
            prop_assert!(issues.is_empty());
        }
    }
}
