use speculoos::prelude::*;

use std::collections::HashSet;

use super::NumberFormat;

const NNBSP: char = '\u{202F}';
const NBSP: char = '\u{00A0}';

fn fmt(lang: Option<&str>, n: f64) -> String {
    NumberFormat::for_lang(lang).format(n)
}

#[test]
fn format_french_groups_with_narrow_space_and_comma() {
    assert_that!(fmt(Some("fr"), 12345.5)).is_equal_to(format!("12{NNBSP}345,5"));
}

#[test]
fn format_french_below_ten_thousand_is_not_grouped() {
    assert_that!(fmt(Some("fr"), 1234.0)).is_equal_to("1234".to_owned());
    assert_that!(fmt(Some("fr"), 9999.25)).is_equal_to("9999,25".to_owned());
}

#[test]
fn format_english_groups_with_comma() {
    assert_that!(fmt(Some("en"), 12345.0)).is_equal_to("12,345".to_owned());
    assert_that!(fmt(Some("en"), 1234567.5)).is_equal_to("1,234,567.5".to_owned());
}

#[test]
fn format_german_uses_comma_decimal_and_dot_groups() {
    assert_that!(fmt(Some("de"), 0.5)).is_equal_to("0,5".to_owned());
    assert_that!(fmt(Some("de-CH"), 12345.5)).is_equal_to("12.345,5".to_owned());
}

#[test]
fn format_russian_groups_with_no_break_space() {
    assert_that!(fmt(Some("ru"), 12345.5)).is_equal_to(format!("12{NBSP}345,5"));
    assert_that!(fmt(Some("pl_PL"), 100000.0)).is_equal_to(format!("100{NBSP}000"));
}

#[test]
fn format_arabic_keeps_latin_digits() {
    assert_that!(fmt(Some("ar"), 12345.5)).is_equal_to("12,345.5".to_owned());
}

#[test]
fn format_unknown_or_absent_language_is_plain() {
    assert_that!(fmt(None, 12345.5)).is_equal_to("12345.5".to_owned());
    assert_that!(fmt(Some("xx"), 12345.5)).is_equal_to("12345.5".to_owned());
    assert_that!(fmt(Some(""), 12345.5)).is_equal_to("12345.5".to_owned());
}

#[test]
fn format_integers_print_without_decimals() {
    assert_that!(fmt(Some("fr"), 12345.0)).is_equal_to(format!("12{NNBSP}345"));
    assert_that!(fmt(Some("en"), 7.0)).is_equal_to("7".to_owned());
}

#[test]
fn format_negatives_keep_the_ascii_sign() {
    assert_that!(fmt(Some("fr"), -12345.5)).is_equal_to(format!("-12{NNBSP}345,5"));
    assert_that!(fmt(Some("en"), -0.25)).is_equal_to("-0.25".to_owned());
}

#[test]
fn format_language_tag_is_case_insensitive() {
    assert_that!(fmt(Some("FR-ca"), 0.5)).is_equal_to("0,5".to_owned());
}

#[test]
fn format_large_numbers_never_use_scientific_notation() {
    assert_that!(fmt(Some("en"), 1e14 + 0.5)).is_equal_to("100,000,000,000,000.5".to_owned());
    assert_that!(fmt(None, 1e-7)).is_equal_to("0.0000001".to_owned());
}

#[test]
fn localize_rewrites_a_plain_number_with_unit() {
    let fr = NumberFormat::for_lang(Some("fr"));
    assert_that!(fr.localize("22.2k")).is_equal_to(Some("22,2k".to_owned()));
    assert_that!(fr.localize("12345.5")).is_equal_to(Some(format!("12{NNBSP}345,5")));
    assert_that!(fr.localize("-0.5")).is_equal_to(Some("-0,5".to_owned()));
}

#[test]
fn localize_leaves_non_numbers_alone() {
    let fr = NumberFormat::for_lang(Some("fr"));
    for s in ["Jan", "1.2.3", "", "-", ".5", "1e3", "v1.2", "12 345"] {
        asserting(s).that(&fr.localize(s)).is_none();
    }
}

#[test]
fn localize_svg_rewrites_numeric_text_nodes_only() {
    let fr = NumberFormat::for_lang(Some("fr"));
    let svg = "<svg><text x=\"1\">\n12345.5\n</text><text>Jan 2.5</text><line x1=\"1.5\"/><text>0.25</text></svg>";
    assert_that!(fr.localize_svg(svg, &HashSet::new())).is_equal_to(format!(
        "<svg><text x=\"1\">\n12{NNBSP}345,5\n</text><text>Jan 2.5</text><line x1=\"1.5\"/><text>0,25</text></svg>"
    ));
}

#[test]
fn localize_svg_plain_format_returns_the_svg_unchanged() {
    let svg = "<svg><text>12345.5</text></svg>";
    assert_that!(NumberFormat::default().localize_svg(svg, &HashSet::new())).is_equal_to(svg.to_owned());
}

#[test]
fn localize_svg_keeps_texts_typed_as_text() {
    let fr = NumberFormat::for_lang(Some("fr"));
    let keep: HashSet<String> = ["75001".to_owned(), "1.5".to_owned()].into();
    let svg = "<text>75001</text><text> 1.5 </text><text>12345</text>";
    assert_that!(fr.localize_svg(svg, &keep))
        .is_equal_to(format!("<text>75001</text><text> 1.5 </text><text>12{NNBSP}345</text>"));
}

#[test]
fn localize_svg_skips_self_closing_text_path_and_mixed_text() {
    let fr = NumberFormat::for_lang(Some("fr"));
    let svg =
        "<text x=\"1\"/><textPath>2.5</textPath><text>Total 2.5</text><text><tspan>2.5</tspan></text><text>0.5</text>";
    assert_that!(fr.localize_svg(svg, &HashSet::new())).is_equal_to(
        "<text x=\"1\"/><textPath>2.5</textPath><text>Total 2.5</text><text><tspan>2.5</tspan></text><text>0,5</text>"
            .to_owned(),
    );
}

#[test]
fn localize_svg_unterminated_text_is_left_unchanged() {
    let fr = NumberFormat::for_lang(Some("fr"));
    assert_that!(fr.localize_svg("<text>2.5", &HashSet::new())).is_equal_to("<text>2.5".to_owned());
}
