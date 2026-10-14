use speculoos::prelude::*;

use super::*;
use crate::pptx::color::{ColorBase, ColorSpec};
use crate::pptx::test_support::THEME;
use crate::source::{ColorMod, Fill, Origin, Rgba, Styled};

fn data() -> ThemeData<'static> {
    ThemeData::parse(THEME, "ppt/theme/theme1.xml", 100_000).unwrap()
}

fn accent1() -> ColorSpec {
    ColorSpec { base: ColorBase::Scheme("accent1".into()), mods: vec![] }
}

#[test]
fn parse_reads_all_twelve_color_slots() {
    let t = data();
    assert_that!(t.theme.colors.len()).is_equal_to(12);
    assert_that!((t.theme.colors.get("accent2").copied(), t.theme.colors.get("dk1").copied()))
        .is_equal_to((Some(Rgba::new(0xed, 0x7d, 0x31)), Some(Rgba::new(0, 0, 0))));
}

#[test]
fn parse_reads_name_and_major_minor_fonts() {
    let t = data();
    assert_that!((t.theme.name.as_str(), t.theme.fonts.major.as_str(), t.theme.fonts.minor.as_str())).is_equal_to((
        "Bureau",
        "Calibri Light",
        "Calibri",
    ));
}

#[test]
fn fill_style_substitutes_the_placeholder_color() {
    let t = data();
    let f = t.fill_style(1, Some(&accent1()), &mut |_| None).map(|s| s.resolve(&t.default_ctx()));
    assert_that!(f).is_equal_to(Some(Fill::Solid { color: Styled::scheme(Rgba::new(0x44, 0x72, 0xc4), "accent1") }));
}

#[test]
fn fill_style_two_applies_its_tint_on_the_placeholder_color() {
    let t = data();
    let Some(Fill::Solid { color }) =
        t.fill_style(2, Some(&accent1()), &mut |_| None).map(|s| s.resolve(&t.default_ctx()))
    else {
        panic!("plein attendu")
    };
    assert_that!(color.origin).is_equal_to(Origin::Scheme {
        name: "accent1".into(),
        slot: "accent1".into(),
        mods: vec![ColorMod::Tint(0.5)],
    });
}

#[test]
fn fill_style_above_1000_reads_background_styles() {
    let t = data();
    assert_that!(t.fill_style(1002, None, &mut |_| None).map(|s| s.resolve(&t.default_ctx())))
        .is_equal_to(Some(Fill::Solid { color: Styled::explicit(Rgba::new(0xf2, 0xf2, 0xf2)) }));
}

#[test]
fn line_style_reads_width_and_placeholder_color() {
    let t = data();
    let s = t.line_style(2, Some(&accent1())).finish(&t.default_ctx()).unwrap();
    assert_that!((s.width, s.color.origin)).is_equal_to((1.0, Origin::scheme("accent1")));
}

#[test]
fn style_index_zero_is_nothing_and_out_of_range_is_none() {
    let t = data();
    assert_that!((t.fill_style(0, None, &mut |_| None), t.fill_style(9, None, &mut |_| None)))
        .is_equal_to((Some(FillSpec::None), None));
    assert_that!((t.line_style(0, None).finish(&t.default_ctx()), t.line_style(9, None)))
        .is_equal_to((None, LineSpec::default()));
}

#[test]
fn office_theme_has_the_office_palette() {
    let t = ThemeData::office().unwrap();
    assert_that!((t.theme.colors.get("accent1").copied(), t.theme.fonts.minor.as_str()))
        .is_equal_to((Some(Rgba::new(0x44, 0x72, 0xc4)), "Calibri"));
}

#[test]
fn absurdly_long_font_name_is_clipped() {
    let xml = THEME.replace(r#"typeface="Calibri""#, &format!(r#"typeface="{}""#, "x".repeat(10_000)));
    let t = ThemeData::parse(&xml, "t", 100_000).unwrap();
    assert_that!(t.theme.fonts.minor.chars().count()).is_equal_to(MAX_FONT_NAME);
}

#[test]
fn style_index_1000_is_no_fill_not_an_index_into_the_shape_fills() {
    let t = data();
    assert_that!(t.fill_style(1000, None, &mut |_| None)).is_equal_to(Some(FillSpec::None));
}
