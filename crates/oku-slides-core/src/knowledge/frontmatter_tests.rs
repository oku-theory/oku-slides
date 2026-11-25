use std::fmt::Write as _;

use oku_slides_i18n::Localizer;
use proptest::prelude::*;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::knowledge::limits::{MAX_FRONTMATTER_BYTES, MAX_YAML_DEPTH};

fn concept(yaml: &str) -> String {
    format!("---\n{yaml}\n---\nbody\n")
}

fn check(path: &str, text: &str) -> Result<(), FrontmatterReason> {
    check_text(&KnowledgePath::parse(path).unwrap(), text)
}

fn nested(levels: usize) -> String {
    let mut yaml = String::new();
    for level in 0..levels {
        yaml.push_str(&"  ".repeat(level));
        yaml.push_str("a:\n");
    }
    yaml.push_str(&"  ".repeat(levels));
    yaml.push('1');
    yaml
}

fn dup(key: &str) -> FrontmatterReason {
    FrontmatterReason::DuplicateKey { key: key.to_owned() }
}

fn bad_at(key: &str) -> FrontmatterReason {
    FrontmatterReason::BadTimestamp { key: key.to_owned() }
}

#[rstest]
#[case::note("type: Note")]
#[case::unknown_keys("type: Note\nx-foo: 1\ntags: [a, b]")]
#[case::quoted_type("type: \"Note\"")]
#[case::upper_z("type: Note\ngenerated: {at: 2026-11-24T10:00:00Z}")]
#[case::lower_z("type: Note\ngenerated: {at: 2026-11-24t10:00:00z}")]
#[case::offset("type: Note\ngenerated:\n  at: 2026-11-24T10:00:00+01:00\n  by: someone")]
#[case::blank_lines_after_document_end("type: Note\n...\n\n  \n")]
#[case::comment_after_document_end("type: Note\n...\n\n# note\n\n")]
#[case::document_end_alone("type: Note\n...")]
fn concept_ok(#[case] yaml: &str) {
    assert_that!(check("/a.md", &concept(yaml))).is_ok();
}

#[rstest]
#[case::no_frontmatter("just text\n", FrontmatterReason::Missing)]
#[case::empty("", FrontmatterReason::Missing)]
#[case::unterminated("---\ntype: Note\n", FrontmatterReason::Unterminated)]
#[case::only_fence("---", FrontmatterReason::Unterminated)]
#[case::bad_yaml(&concept("a: [1"), FrontmatterReason::NotYaml)]
#[case::bad_yaml_nested_map(&concept("a: b: c"), FrontmatterReason::NotYaml)]
#[case::sequence(&concept("- a"), FrontmatterReason::NotMapping)]
#[case::scalar_root(&concept("hello"), FrontmatterReason::NotMapping)]
#[case::complex_key(&concept("? [a]\n: 1"), FrontmatterReason::NotMapping)]
#[case::duplicate(&concept("type: a\ntype: b"), dup("type"))]
#[case::nested_duplicate(&concept("type: a\nm: {k: 1, k: 2}"), dup("k"))]
#[case::alias(&concept("type: a\nb: &x 1\nc: *x"), FrontmatterReason::Alias)]
#[case::anchor_alone(&concept("type: a\nb: &x 1"), FrontmatterReason::Alias)]
#[case::anchored_map(&concept("type: a\nb: &x {c: 1}"), FrontmatterReason::Alias)]
#[case::tag(&concept("type: !!str x"), FrontmatterReason::Tag)]
#[case::custom_tag(&concept("type: a\nb: !thing 1"), FrontmatterReason::Tag)]
#[case::too_deep(&concept(&nested(MAX_YAML_DEPTH + 1)), FrontmatterReason::TooDeep)]
#[case::after_document_end(&concept("type: a\n...\nx: 1"), FrontmatterReason::AfterDocumentEnd)]
#[case::second_document(&concept("type: Note\n...\n: : ["), FrontmatterReason::AfterDocumentEnd)]
#[case::after_blank_document_end(&concept("type: a\n...\n\nx: 1"), FrontmatterReason::AfterDocumentEnd)]
#[case::empty_type(&concept("type: \"\""), FrontmatterReason::TypeMissing)]
#[case::null_type(&concept("type:"), FrontmatterReason::TypeMissing)]
#[case::no_type(&concept("title: x"), FrontmatterReason::TypeMissing)]
#[case::map_type(&concept("type: {a: 1}"), FrontmatterReason::TypeMissing)]
#[case::empty_block("---\n---\nbody", FrontmatterReason::TypeMissing)]
#[case::no_offset(&concept("type: a\ngenerated: {at: 2026-11-24T10:00:00}"), bad_at("generated.at"))]
#[case::generated_scalar(&concept("type: a\ngenerated: 3"), bad_at("generated"))]
#[case::generated_without_at(&concept("type: a\ngenerated: {by: x}"), bad_at("generated.at"))]
#[case::generated_garbage(&concept("type: a\ngenerated: {at: yesterday}"), bad_at("generated.at"))]
fn concept_refused(#[case] text: &str, #[case] reason: FrontmatterReason) {
    assert_that!(check("/a.md", text)).is_err_containing(reason);
}

#[test]
fn concept_refuses_a_large_block() {
    let text = concept(&format!("type: a\nnote: {}", "x".repeat(MAX_FRONTMATTER_BYTES + 1)));
    assert_that!(check("/a.md", &text)).is_err_containing(FrontmatterReason::TooLarge);
}

#[test]
fn concept_accepts_the_deepest_allowed_nesting() {
    assert_that!(frontmatter(&concept(&nested(MAX_YAML_DEPTH)))).is_ok();
}

#[test]
fn concept_accepts_a_block_at_the_byte_ceiling() {
    let head = "type: a\nnote: ";
    let text = concept(&format!("{head}{}", "x".repeat(MAX_FRONTMATTER_BYTES - head.len() - 1)));
    assert_that!(check("/a.md", &text)).is_ok();
}

#[rstest]
#[case::lf("---\ntype: Note\n---\nbody\n")]
#[case::crlf("---\r\ntype: Note\r\n---\r\nbody\r\n")]
#[case::bom("\u{feff}---\ntype: Note\n---\nbody\n")]
#[case::bom_crlf("\u{feff}---\r\ntype: Note\r\n---\r\nbody\r\n")]
#[case::no_body("---\ntype: Note\n---")]
fn concept_line_endings_and_bom(#[case] text: &str) {
    assert_that!(check("/a.md", text)).is_ok();
}

#[test]
fn a_second_bom_is_not_a_fence() {
    assert_that!(check("/a.md", "\u{feff}\u{feff}---\ntype: Note\n---\n"))
        .is_err_containing(FrontmatterReason::Missing);
}

#[test]
fn billion_laughs_is_refused_without_expansion() {
    let mut yaml = String::from("type: a\nl0: &l0 [x, x, x, x, x, x, x, x, x]\n");
    for n in 1..10 {
        let prev = n - 1;
        let refs = vec![format!("*l{prev}"); 9].join(", ");
        writeln!(yaml, "l{n}: &l{n} [{refs}]").unwrap();
    }
    assert_that!(check("/a.md", &concept(&yaml))).is_err_containing(FrontmatterReason::Alias);
}

#[test]
fn root_index_allows_its_four_keys() {
    let text = concept("okf_version: \"0.3\"\ntitle: Deck\ndescription: d\nlicense: MIT");
    assert_that!(check("/index.md", &text)).is_ok();
    assert_that!(check("/index.md", "# Index\n")).is_ok();
}

#[rstest]
#[case::kind("type: Note", "type")]
#[case::after_allowed("title: x\nauthor: me", "author")]
fn root_index_refuses_other_keys(#[case] yaml: &str, #[case] key: &str) {
    assert_that!(check("/index.md", &concept(yaml)))
        .is_err_containing(FrontmatterReason::NotAllowed { key: key.to_owned() });
}

#[rstest]
#[case::sub_index("/a/index.md", "title: x", "title")]
#[case::log("/log.md", "type: Log", "type")]
#[case::sub_log("/a/log.md", "x-foo: 1", "x-foo")]
fn other_reserved_files_carry_none(#[case] path: &str, #[case] yaml: &str, #[case] key: &str) {
    assert_that!(check(path, &concept(yaml))).is_err_containing(FrontmatterReason::NotAllowed { key: key.to_owned() });
}

#[rstest]
#[case::empty_log("/log.md", "")]
#[case::log_body("/log.md", "- 2026-11-24 did a thing\n")]
#[case::sub_index("/a/index.md", "# A\n")]
#[case::empty_block("/a/index.md", "---\n---\nbody")]
fn reserved_files_without_frontmatter_are_fine(#[case] path: &str, #[case] text: &str) {
    assert_that!(check(path, text)).is_ok();
}

#[test]
fn reserved_files_still_refuse_a_broken_block() {
    assert_that!(check("/log.md", "---\ntype: x\n")).is_err_containing(FrontmatterReason::Unterminated);
    assert_that!(check("/index.md", &concept("a: &x 1"))).is_err_containing(FrontmatterReason::Alias);
}

#[rstest]
#[case::missing("no fence", FrontmatterReason::Missing)]
#[case::missing_near("---x\na: 1\n---\n", FrontmatterReason::Missing)]
#[case::unterminated("---\na: 1\n", FrontmatterReason::Unterminated)]
#[case::indented_close("---\na: 1\n ---\n", FrontmatterReason::Unterminated)]
fn split_refuses(#[case] text: &str, #[case] reason: FrontmatterReason) {
    assert_that!(split(text)).is_err_containing(reason);
}

#[rstest]
#[case::lf("---\na: 1\n---\nbody\n", "a: 1\n", "body\n")]
#[case::crlf("---\r\na: 1\r\n---\r\nbody", "a: 1\r\n", "body")]
#[case::bom("\u{feff}---\na: 1\n---\n", "a: 1\n", "")]
#[case::empty_block("---\n---\nx", "", "x")]
#[case::first_close_wins("---\na: 1\n---\nb: 2\n---\nc", "a: 1\n", "b: 2\n---\nc")]
fn split_returns_the_block_and_the_body(#[case] text: &str, #[case] yaml: &str, #[case] body: &str) {
    assert_that!(split(text)).is_ok().is_equal_to((yaml, body));
}

#[test]
fn frontmatter_reads_the_index_fields_in_order() {
    let fm = frontmatter(&concept("title: Pricing\ntype: Note\nx-foo: [1]\ndescription: How we price")).unwrap();
    assert_that!(fm.kind).is_equal_to(Some("Note".to_owned()));
    assert_that!(fm.title).is_equal_to(Some("Pricing".to_owned()));
    assert_that!(fm.description).is_equal_to(Some("How we price".to_owned()));
    assert_that!(fm.keys).is_equal_to(vec![
        "title".to_owned(),
        "type".to_owned(),
        "x-foo".to_owned(),
        "description".to_owned(),
    ]);
}

#[test]
fn frontmatter_leaves_non_text_fields_empty() {
    let fm = frontmatter(&concept("type: {a: 1}\ntitle:\ndescription: [x]")).unwrap();
    assert_that!(fm.kind).is_none();
    assert_that!(fm.title).is_none();
    assert_that!(fm.description).is_none();
    assert_that!(fm.keys.len()).is_equal_to(3);
}

#[rstest]
#[case::draft_and_stale("type: Note\nstatus: draft\nstale: true", Some("draft"), true)]
#[case::quoted_stale("type: Note\nstale: \"true\"", None, false)]
#[case::stale_yes("type: Note\nstale: \"yes\"", None, false)]
#[case::stale_false("type: Note\nstale: false", None, false)]
#[case::stale_capitalized("type: Note\nstale: True", None, false)]
#[case::status_map("type: Note\nstatus: {a: 1}", None, false)]
#[case::status_null("type: Note\nstatus:", None, false)]
#[case::absent("type: Note", None, false)]
fn frontmatter_reads_status_and_stale(#[case] yaml: &str, #[case] status: Option<&str>, #[case] stale: bool) {
    let fm = frontmatter(&concept(yaml)).unwrap();
    assert_that!(fm.status.as_deref()).is_equal_to(status);
    assert_that!(fm.stale).is_equal_to(stale);
}

#[test]
fn frontmatter_refuses_what_check_text_refuses_structurally() {
    assert_that!(frontmatter("none")).is_err_containing(FrontmatterReason::Missing);
    assert_that!(frontmatter(&concept("a: 1\na: 2"))).is_err_containing(dup("a"));
}

#[test]
fn every_frontmatter_reason_key_exists_in_every_locale() {
    let reasons = [
        FrontmatterReason::Missing,
        FrontmatterReason::Unterminated,
        FrontmatterReason::TooLarge,
        FrontmatterReason::NotYaml,
        FrontmatterReason::NotMapping,
        dup("k"),
        FrontmatterReason::Alias,
        FrontmatterReason::Tag,
        FrontmatterReason::TooDeep,
        FrontmatterReason::AfterDocumentEnd,
        FrontmatterReason::TypeMissing,
        bad_at("generated.at"),
        FrontmatterReason::NotAllowed { key: "k".to_owned() },
    ];
    let en = Localizer::new(Some("en"));
    let fr = Localizer::new(Some("fr"));
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        for reason in &reasons {
            assert!(l.has(reason.key()), "{locale}: {} absent", reason.key());
        }
    }
    for reason in &reasons {
        assert_ne!(en.tr(reason.key()), fr.tr(reason.key()), "{} not translated in fr", reason.key());
        assert!(!en.tr(reason.key()).is_empty());
    }
}

#[test]
fn duplicate_key_names_the_key_in_french() {
    let fr = Localizer::new(Some("fr"));
    let said = dup("titre").message(&fr);
    assert_that!(said.contains("titre")).is_true();
    assert_that!(said).is_equal_to("la clé « titre » apparaît deux fois dans l'en-tête".to_owned());
    assert_that!(FrontmatterReason::Alias.message(&fr)).is_equal_to(fr.tr("knowledge-frontmatter-alias"));
}

#[test]
fn display_is_english_and_names_the_key() {
    assert_that!(dup("k").to_string()).is_equal_to("the frontmatter key \"k\" appears twice".to_owned());
    assert_that!(FrontmatterReason::Missing.to_string()).contains("---");
}

proptest! {
    #[test]
    fn split_never_panics(text in "\\PC{0,200}") {
        let _ = split(&text);
    }

    #[test]
    fn split_never_panics_on_fences(text in "(---|\r|\n|a: 1|\u{feff}| ){0,30}") {
        let _ = split(&text);
    }

    #[test]
    fn check_text_never_panics(text in "(---\n)?\\PC{0,200}") {
        let path = KnowledgePath::parse("/a.md").unwrap();
        let _ = check_text(&path, &text);
        let _ = frontmatter(&text);
    }

    #[test]
    fn check_text_never_panics_on_yaml_shapes(body in "([a-z&*!:?\\[\\]{},# \n-]|\\.\\.\\.){0,120}") {
        let path = KnowledgePath::parse("/a.md").unwrap();
        let index = KnowledgePath::parse("/index.md").unwrap();
        let text = format!("---\n{body}\n---\n");
        let _ = check_text(&path, &text);
        let _ = check_text(&index, &text);
    }
}
