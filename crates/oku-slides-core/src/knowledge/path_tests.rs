use proptest::prelude::*;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::error::PathReason;

#[rstest]
#[case::empty("/", PathReason::Empty)]
#[case::blank("", PathReason::Empty)]
#[case::relative("a.md", PathReason::NotAbsolute)]
#[case::upper("/A.md", PathReason::BadSegment)]
#[case::double_slash("/a//b.md", PathReason::BadSegment)]
#[case::trailing_slash("/a/", PathReason::BadSegment)]
#[case::trailing_dot("/a./b.md", PathReason::BadSegment)]
#[case::parent("/../x.md", PathReason::DotSegment)]
#[case::hidden("/.x.md", PathReason::DotSegment)]
#[case::reserved("/con.md", PathReason::Reserved)]
#[case::reserved_lpt("/a/lpt1.txt.md", PathReason::Reserved)]
fn parse_refuses(#[case] text: &str, #[case] reason: PathReason) {
    assert_that!(KnowledgePath::parse(text)).is_err_containing(reason);
}

#[test]
fn parse_limits_depth_and_length() {
    let nine = format!("/{}", ["a"; 9].join("/"));
    assert_that!(KnowledgePath::parse(&nine)).is_err_containing(PathReason::TooDeep);
    let eight = format!("/{}", ["a"; 8].join("/"));
    assert_that!(KnowledgePath::parse(&eight)).is_ok();
    let long = format!("/{}.md", "a".repeat(252));
    assert_that!(long.len()).is_equal_to(256);
    assert_that!(KnowledgePath::parse(&long)).is_err_containing(PathReason::TooLong);
    let max = format!("/{}.md", "a".repeat(251));
    assert_that!(KnowledgePath::parse(&max)).is_ok();
}

#[test]
fn writable_needs_markdown() {
    assert_that!(KnowledgePath::parse("/a.png")).is_ok();
    assert_that!(KnowledgePath::parse_writable("/a.png")).is_err_containing(PathReason::NotMarkdown);
    assert_that!(KnowledgePath::parse_writable("/.md")).is_err_containing(PathReason::DotSegment);
    assert_that!(KnowledgePath::parse_writable("/a.md")).is_ok();
}

#[test]
fn accessors_describe_the_path() {
    let p = KnowledgePath::parse("/a/b.md").unwrap();
    assert_that!(p.as_str()).is_equal_to("/a/b.md");
    assert_that!(p.file_name()).is_equal_to("b.md");
    assert_that!(p.stem()).is_equal_to("b");
    assert_that!(p.dir()).is_equal_to("/a");
    assert_that!(p.is_index()).is_false();
    let root = KnowledgePath::parse("/index.md").unwrap();
    assert_that!(root.dir()).is_equal_to("");
    assert_that!(root.is_root_index()).is_true();
    assert_that!(KnowledgePath::parse("/a/index.md").unwrap().is_root_index()).is_false();
    assert_that!(KnowledgePath::parse("/a/index.md").unwrap().is_index()).is_true();
    assert_that!(KnowledgePath::parse("/a/log.md").unwrap().is_log()).is_true();
}

#[test]
fn with_suffix_numbers_the_stem() {
    let p = KnowledgePath::parse("/a/b.md").unwrap();
    assert_that!(p.with_suffix(2).unwrap().as_str()).is_equal_to("/a/b-2.md");
    let root = KnowledgePath::parse("/b.md").unwrap();
    assert_that!(root.with_suffix(10).unwrap().as_str()).is_equal_to("/b-10.md");
    let long = KnowledgePath::parse(&format!("/{}.md", "a".repeat(251))).unwrap();
    assert_that!(long.as_str().len()).is_equal_to(255);
    assert_that!(long.with_suffix(2)).is_err_containing(PathReason::TooLong);
    let near = KnowledgePath::parse(&format!("/{}.md", "a".repeat(250))).unwrap();
    assert_that!(near.as_str().len()).is_equal_to(254);
    assert_that!(near.with_suffix(2)).is_err_containing(PathReason::TooLong);
}

#[test]
fn file_ids_are_32_lowercase_hex() {
    assert_that!(is_file_id(&knowledge_file_id())).is_true();
    assert_that!(is_file_id(&"a".repeat(32))).is_true();
    assert_that!(is_file_id(&"A".repeat(32))).is_false();
    assert_that!(is_file_id(&"a".repeat(31))).is_false();
}

proptest! {
    #[test]
    fn parse_never_panics(text in ".*") {
        let _ = KnowledgePath::parse(&text);
    }
}

#[test]
fn a_recovered_path_is_writable_for_any_file_id() {
    for text in ["0123456789abcdef0123456789abcdef", "ffffffffffffffffffffffffffffffff", &knowledge_file_id()] {
        let id = FileId::parse(text.to_owned()).unwrap();
        let path = KnowledgePath::recovered(&id);
        assert_that!(KnowledgePath::parse_writable(path.as_str())).is_ok_containing(&path);
        assert_that!(path.as_str()).is_equal_to(format!("/recovered/{text}.md").as_str());
    }
    assert_that!(FileId::parse("XYZ".to_owned())).is_err_containing("XYZ".to_owned());
}
