use speculoos::prelude::*;

use std::collections::HashSet;

use super::{NumberFormat, numeric_texts};
use crate::option::ChartOption;

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
    assert_that!(fmt(Some("en"), 1_234_567.5)).is_equal_to("1,234,567.5".to_owned());
}

#[test]
fn format_german_uses_comma_decimal_and_dot_groups() {
    assert_that!(fmt(Some("de"), 0.5)).is_equal_to("0,5".to_owned());
    assert_that!(fmt(Some("de-CH"), 12345.5)).is_equal_to("12.345,5".to_owned());
}

#[test]
fn format_russian_groups_with_no_break_space() {
    assert_that!(fmt(Some("ru"), 12345.5)).is_equal_to(format!("12{NBSP}345,5"));
    assert_that!(fmt(Some("pl_PL"), 100_000.0)).is_equal_to(format!("100{NBSP}000"));
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
fn format_negatives_use_the_minus_sign() {
    assert_that!(fmt(Some("fr"), -12345.5)).is_equal_to(format!("\u{2212}12{NNBSP}345,5"));
    assert_that!(fmt(Some("en"), -0.25)).is_equal_to("\u{2212}0.25".to_owned());
    assert_that!(fmt(Some("de"), -12345.0)).is_equal_to("\u{2212}12.345".to_owned());
}

#[test]
fn format_negatives_without_language_use_the_minus_sign_without_groups() {
    assert_that!(fmt(None, -12345.5)).is_equal_to("\u{2212}12345.5".to_owned());
    assert_that!(fmt(Some("xx"), -0.25)).is_equal_to("\u{2212}0.25".to_owned());
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
    assert_that!(fr.localize("-0.5")).is_equal_to(Some("\u{2212}0,5".to_owned()));
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
fn localize_svg_plain_format_leaves_positive_numbers_unchanged() {
    let svg = "<svg><text>12345.5</text><text>A: 38.2%</text></svg>";
    assert_that!(NumberFormat::default().localize_svg(svg, &HashSet::new())).is_equal_to(svg.to_owned());
}

#[test]
fn localize_svg_plain_format_only_rewrites_the_sign() {
    let svg = "<svg><text>-12345.5</text><text>A: -2.5%</text><text>-1.5</text></svg>";
    let keep: HashSet<String> = ["-1.5".to_owned()].into();
    assert_that!(NumberFormat::default().localize_svg(svg, &keep))
        .is_equal_to("<svg><text>\u{2212}12345.5</text><text>A: \u{2212}2.5%</text><text>-1.5</text></svg>".to_owned());
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

#[test]
fn localize_svg_rewrites_the_value_of_a_name_value_label() {
    let fr = NumberFormat::for_lang(Some("fr"));
    let svg = "<text>A: 12345.5</text><text>B: x: -2.5%</text><text>C: v1.2</text><text>D:12345</text>";
    assert_that!(fr.localize_svg(svg, &HashSet::new())).is_equal_to(format!(
        "<text>A: 12{NNBSP}345,5</text><text>B: x: \u{2212}2,5%</text><text>C: v1.2</text><text>D:12345</text>"
    ));
}

#[test]
fn localize_svg_keeps_name_value_texts_typed_as_text() {
    let fr = NumberFormat::for_lang(Some("fr"));
    let keep: HashSet<String> = ["Ratio: 1.5".to_owned()].into();
    let svg = "<text>Ratio: 1.5</text><text>Autre: 1.5</text>";
    assert_that!(fr.localize_svg(svg, &keep)).is_equal_to("<text>Ratio: 1.5</text><text>Autre: 1,5</text>".to_owned());
}

fn option(v: serde_json::Value) -> ChartOption {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn numeric_texts_collects_numbers_and_name_value_texts() {
    let texts = numeric_texts(&option(serde_json::json!({
        "dataset": { "source": [["Code", "Ventes"], ["75001", 1], ["Ratio: 1.5", 2], ["Jan", 3]] },
        "series": [{ "type": "bar" }]
    })));
    assert_that!(texts.contains("75001")).is_true();
    assert_that!(texts.contains("Ratio: 1.5")).is_true();
    assert_that!(texts.contains("Jan")).is_false();
}

#[test]
fn numeric_texts_ignores_row_and_column_ids() {
    let texts = numeric_texts(&option(serde_json::json!({
        "dataset": {
            "source": [["Mois", "Ventes"], ["Jan", 12_345_678]],
            "ids": ["00000000", "12345678"],
            "columns": ["87654321", "11223344"]
        },
        "series": [{ "type": "bar" }]
    })));
    for id in ["00000000", "12345678", "87654321", "11223344"] {
        asserting(id).that(&texts.contains(id)).is_false();
    }
}

#[test]
fn cell_as_f64_accepts_both_minus_signs() {
    use crate::option::Cell;
    assert_that!(Cell::Text("-2.5".to_owned()).as_f64()).is_equal_to(Some(-2.5));
    assert_that!(Cell::Text("\u{2212}2,5".to_owned()).as_f64()).is_equal_to(Some(-2.5));
}
