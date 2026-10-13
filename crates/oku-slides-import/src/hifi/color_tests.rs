use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::source::{ColorMod, Styled};
use crate::test_support::office_theme;

/// La couleur de l'emplacement `slot` du thème Office, écrite `name`, modifiée par `mods`.
fn office(name: &str, slot: &str, mods: Vec<ColorMod>) -> Color {
    let value = office_theme().colors[slot];
    Styled::mapped(value, name, slot, mods)
}

#[rstest]
#[case("lt1", "bg")]
#[case("dk1", "ink")]
#[case("dk2", "ink-muted")]
#[case("lt2", "surface")]
#[case("accent1", "accent")]
#[case("accent2", "accent-2")]
#[case("accent3", "accent-3")]
#[case("accent4", "accent-4")]
#[case("accent5", "accent-5")]
#[case("accent6", "accent-6")]
#[case("hlink", "link")]
#[case("folHlink", "link-visited")]
fn theme_colors_write_an_unmodified_slot_as_its_token(#[case] slot: &str, #[case] token: &str) {
    let colors = ThemeColors::new(&office_theme());
    assert_that!(colors.write(&office(slot, slot, Vec::new()))).is_equal_to(token.to_owned());
}

#[test]
fn theme_colors_write_an_alias_by_its_mapped_slot() {
    // `tx2` écrit dans le fichier, lu dans `dk2` après `clrMap`.
    let colors = ThemeColors::new(&office_theme());
    assert_that!(colors.write(&office("tx2", "dk2", Vec::new()))).is_equal_to("ink-muted".to_owned());
}

#[test]
fn theme_colors_write_a_modified_slot_as_its_resolved_hex() {
    let colors = ThemeColors::new(&office_theme());
    let mut c = office("accent1", "accent1", vec![ColorMod::LumMod(0.75)]);
    c.value = Rgba::new(0x2f, 0x54, 0x97);
    assert_that!(colors.write(&c)).is_equal_to("#2f5497".to_owned());
}

#[test]
fn theme_colors_write_a_translucent_color_with_its_alpha() {
    let colors = ThemeColors::new(&office_theme());
    let mut c = office("accent1", "accent1", vec![ColorMod::Alpha(0.5)]);
    c.value.a = 128;
    assert_that!(colors.write(&c)).is_equal_to("#4472c480".to_owned());
    assert_that!(oku_slides_core::is_hex_color(&colors.write(&c))).is_true();
}

#[rstest]
#[case(Styled::explicit(Rgba::new(0xff, 0, 0)), "#ff0000")]
#[case(Styled::explicit(Rgba::new(0x44, 0x72, 0xc4)), "#4472c4")]
#[case(Styled::default_value(Rgba::new(0, 0, 0)), "#000000")]
fn theme_colors_write_an_explicit_or_default_color_as_hex(#[case] c: Color, #[case] hex: &str) {
    let colors = ThemeColors::new(&office_theme());
    assert_that!(colors.write(&c)).is_equal_to(hex.to_owned());
}

#[test]
fn theme_colors_write_a_slot_of_another_theme_as_hex() {
    // Un second master au thème différent : son `accent1` n'est pas le jeton `accent`.
    let colors = ThemeColors::new(&office_theme());
    let c = Styled::scheme(Rgba::new(0x15, 0x60, 0x82), "accent1");
    assert_that!(colors.write(&c)).is_equal_to("#156082".to_owned());
}

#[test]
fn theme_colors_write_a_slot_missing_from_the_theme_as_hex() {
    let mut theme = office_theme();
    theme.colors.remove("accent6");
    let colors = ThemeColors::new(&theme);
    assert_that!(colors.get("accent-6")).is_none();
    assert_that!(colors.write(&office("accent6", "accent6", Vec::new()))).is_equal_to("#70ad47".to_owned());
}
