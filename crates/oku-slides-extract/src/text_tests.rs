use oku_slides_core::knowledge::{Cut, CutKind};
use speculoos::prelude::*;

use super::*;

fn limits(max_text_bytes: usize) -> ExtractLimits {
    ExtractLimits {
        max_text_bytes,
        sheet_rows: 2000,
        sheet_columns: 50,
        max_images: 0,
        max_image_pixels: 0,
        max_image_bytes: 0,
    }
}

fn md(text: &str) -> String {
    markdown(text.as_bytes(), &limits(1 << 20)).unwrap().markdown
}

fn txt(text: &str) -> String {
    plain(text.as_bytes(), &limits(1 << 20)).unwrap().markdown
}

#[test]
fn text_markdown_passes_through() {
    let text = "# Title\n\nSome *emphasis*, `code` and a [link](https://example.com/a).\n\n- item\n- [mail](mailto:a@b.org)\n\n| a | b |\n|---|---|\n| 1 | 2 |\n";
    assert_that!(md(text)).is_equal_to(text.to_owned());
}

#[test]
fn text_markdown_frontmatter_stripped() {
    assert_that!(md("---\ntype: Note\ntitle: x\n---\nBody\n")).is_equal_to("Body\n".to_owned());
}

#[test]
fn text_markdown_unterminated_frontmatter_kept_as_text() {
    assert_that!(md("---\ntitle: x\n")).contains("title: x");
}

#[test]
fn text_markdown_raw_html_and_comment_removed() {
    let out = md("a <span onclick=\"x\">b</span> c\n\n<div>\nhidden block\n</div>\n\n<!-- secret -->\n\nd\n");
    assert_that!(out.as_str()).does_not_contain("<");
    assert_that!(out.as_str()).does_not_contain("secret");
    assert_that!(out.as_str()).does_not_contain("hidden block");
    assert_that!(out.as_str()).contains("a b c");
    assert_that!(out.as_str()).contains("d");
}

#[test]
fn text_markdown_remote_image_becomes_link() {
    assert_that!(md("![chart](https://e.org/c.png)\n")).is_equal_to("[chart](https://e.org/c.png)\n".to_owned());
    assert_that!(md("![local](img/c.png)\n")).is_equal_to("local\n".to_owned());
}

#[test]
fn text_markdown_javascript_link_is_text() {
    let out = md("[click](javascript:alert(1)) and [f](file:///etc/passwd)\n");
    assert_that!(out).is_equal_to("click and f\n".to_owned());
}

#[test]
fn text_markdown_link_url_encoded() {
    assert_that!(md("[a](<https://e.org/a b\\c>)\n")).is_equal_to("[a](https://e.org/a%20b%5Cc)\n".to_owned());
}

#[test]
fn text_txt_plain_escaped() {
    assert_that!(txt("*a* # b\n# c\n")).is_equal_to("\\*a\\* # b\n\\# c\n".to_owned());
}

#[test]
fn text_txt_starting_with_dashes_fenced() {
    assert_that!(txt("---\ntitle: x\n---\nhello\n")).is_equal_to("```\n---\ntitle: x\n---\nhello\n```\n".to_owned());
}

#[test]
fn text_with_fence_line_fenced() {
    assert_that!(txt("a\n```\nrm -rf\n```\n")).is_equal_to("````\na\n```\nrm -rf\n```\n````\n".to_owned());
}

#[test]
fn text_tag_and_bidi_chars_stripped() {
    let text = "a\u{E0041}\u{E0042}b\u{202E}c\u{2066}d\u{200B}e\u{FEFF}f\n";
    assert_that!(txt(text)).is_equal_to("abcdef\n".to_owned());
    assert_that!(md(text)).is_equal_to("abcdef\n".to_owned());
}

#[test]
fn text_past_max_text_bytes_cut_and_stated() {
    let text = (0..100).map(|i| format!("line {i:02}\n")).collect::<Vec<_>>().concat();
    let out = plain(text.as_bytes(), &limits(200)).unwrap();
    assert_that!(out.markdown.len()).is_less_than_or_equal_to(200);
    assert_that!(out.markdown.as_str()).starts_with("line 00\n");
    let kept = out.markdown.split("\n\n…").next().unwrap_or_default();
    let cut = Cut { what: CutKind::Text, count: (text.len() - kept.len()) as u64 };
    assert_that!(out.cuts).is_equal_to(vec![cut]);
    assert_that!(out.markdown.as_str()).ends_with(format!("… {} bytes not imported (limit 200)\n", cut.count));
}

#[test]
fn text_fenced_past_max_text_bytes_keeps_the_closing_fence() {
    let text: String = std::iter::once("---\n".to_owned()).chain((0..100).map(|i| format!("line {i:02}\n"))).collect();
    let out = plain(text.as_bytes(), &limits(200)).unwrap().markdown;
    assert_that!(out.len()).is_less_than_or_equal_to(200);
    assert_that!(out.as_str()).contains("\n```\n\n…");
}

#[test]
fn text_not_utf8_type_mismatch() {
    assert_that!(plain(&[0xff, 0xfe, 0x00], &limits(100)).err()).is_equal_to(Some(ExtractRefusal::TypeMismatch));
}
