use std::collections::BTreeMap;

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::pptx::xml::parse;
use crate::source::{ColorMod, Origin, Rgba, Styled};

fn ctx() -> ColorCtx {
    let scheme = BTreeMap::from([
        ("dk1".to_owned(), Rgba::new(0, 0, 0)),
        ("lt1".to_owned(), Rgba::new(255, 255, 255)),
        ("accent1".to_owned(), Rgba::new(0x44, 0x72, 0xc4)),
    ]);
    ColorCtx::new(scheme, default_map())
}

fn spec(inner: &str) -> Option<ColorSpec> {
    let xml = format!(r#"<a:solidFill xmlns:a="urn:a">{inner}</a:solidFill>"#);
    let d = parse(&xml, "t", 10_000).unwrap();
    color_spec_in(d.root_element())
}

fn color_with(inner: &str, c: &ColorCtx) -> Option<Color> {
    spec(inner)?.resolve(c)
}

fn color(inner: &str) -> Option<Color> {
    color_with(inner, &ctx())
}

fn hex(inner: &str) -> String {
    color(inner).map(|c| c.value.to_hex()).unwrap_or_default()
}

#[test]
fn srgb_color_is_explicit() {
    assert_that!(color(r#"<a:srgbClr val="1F4E79"/>"#))
        .is_equal_to(Some(Styled::explicit(Rgba::new(0x1f, 0x4e, 0x79))));
}

#[test]
fn scheme_color_keeps_its_written_name_and_mapped_slot() {
    assert_that!(color(r#"<a:schemeClr val="tx1"/>"#)).is_equal_to(Some(Styled::mapped(
        Rgba::new(0, 0, 0),
        "tx1",
        "dk1",
        vec![],
    )));
}

#[test]
fn overridden_color_map_changes_the_slot_not_the_name() {
    let mut map = default_map();
    map.insert("tx1".into(), "lt1".into());
    let dark = ColorCtx { map: std::rc::Rc::new(map), ..ctx() };
    assert_that!(color_with(r#"<a:schemeClr val="tx1"/>"#, &dark).map(|c| c.origin))
        .is_equal_to(Some(Origin::Scheme { name: "tx1".into(), slot: "lt1".into(), mods: vec![] }));
}

#[test]
fn sys_color_uses_last_clr() {
    assert_that!(hex(r#"<a:sysClr val="windowText" lastClr="1F1F1F"/>"#)).is_equal_to("#1f1f1f".to_owned());
}

#[test]
fn sys_color_without_last_clr_uses_the_system_default() {
    assert_that!(hex(r#"<a:sysClr val="window"/>"#)).is_equal_to("#ffffff".to_owned());
}

#[test]
fn preset_color_white_is_known() {
    assert_that!(hex(r#"<a:prstClr val="white"/>"#)).is_equal_to("#ffffff".to_owned());
}

#[test]
fn alpha_modifier_sets_opacity() {
    assert_that!(color(r#"<a:srgbClr val="FF0000"><a:alpha val="50000"/></a:srgbClr>"#).map(|c| c.value.a))
        .is_equal_to(Some(128));
}

#[rstest]
// Valeurs de PowerPoint pour accent1 = 4472C4 : « Plus sombre 25 % », « Plus clair 80 % », shade 50 % (trait par défaut), tint 40 %.
#[case(r#"<a:schemeClr val="accent1"><a:lumMod val="75000"/></a:schemeClr>"#, "#2f5597")]
#[case(r#"<a:schemeClr val="accent1"><a:lumMod val="20000"/><a:lumOff val="80000"/></a:schemeClr>"#, "#dae3f3")]
#[case(r#"<a:schemeClr val="accent1"><a:shade val="50000"/></a:schemeClr>"#, "#2f528f")]
#[case(r#"<a:schemeClr val="accent1"><a:tint val="40000"/></a:schemeClr>"#, "#cfd5ea")]
fn color_modifiers_match_powerpoint(#[case] inner: &str, #[case] expected: &str) {
    let got = color(inner).unwrap().value;
    let want = u32::from_str_radix(&expected[1..], 16).unwrap();
    let [_, r, g, b] = want.to_be_bytes();
    let off = |a: u8, b: u8| (i16::from(a) - i16::from(b)).abs();
    assert!(
        off(got.r, r) <= 1 && off(got.g, g) <= 1 && off(got.b, b) <= 1,
        "obtenu {}, attendu {expected}",
        got.to_hex()
    );
}

#[test]
fn derived_scheme_color_keeps_its_modifiers_in_the_origin() {
    let c = color(r#"<a:schemeClr val="accent1"><a:lumMod val="75000"/></a:schemeClr>"#).unwrap();
    assert_that!(c.origin).is_equal_to(Origin::Scheme {
        name: "accent1".into(),
        slot: "accent1".into(),
        mods: vec![ColorMod::LumMod(0.75)],
    });
}

#[test]
fn placeholder_color_is_replaced_syntactically_and_mods_appended() {
    let ph = spec(r#"<a:schemeClr val="accent1"/>"#).unwrap();
    let styled = spec(r#"<a:schemeClr val="phClr"><a:tint val="40000"/></a:schemeClr>"#).unwrap();
    let got = styled.with_placeholder(Some(&ph)).unwrap().resolve(&ctx()).unwrap();
    assert_that!(got.origin).is_equal_to(Origin::Scheme {
        name: "accent1".into(),
        slot: "accent1".into(),
        mods: vec![ColorMod::Tint(0.4)],
    });
    assert_that!(got.value.to_hex()).is_equal_to("#cfd5ea".to_owned());
}

#[test]
fn placeholder_color_without_replacement_does_not_resolve() {
    let styled = spec(r#"<a:schemeClr val="phClr"/>"#).unwrap();
    assert_that!((styled.with_placeholder(None), styled.resolve(&ctx()))).is_equal_to((None, None));
}

#[test]
fn modifiers_beyond_the_cap_are_dropped() {
    let many = r#"<a:tint val="99000"/>"#.repeat(MAX_COLOR_MODS + 50);
    let s = spec(&format!(r#"<a:schemeClr val="accent1">{many}</a:schemeClr>"#)).unwrap();
    assert_that!(s.mods.len()).is_equal_to(MAX_COLOR_MODS);
}

#[test]
fn unknown_scheme_slot_is_none() {
    assert_that!(color(r#"<a:schemeClr val="accent6"/>"#)).is_none();
}

#[test]
fn clean_map_keeps_known_keys_and_slots_only() {
    let attrs = [("bg1", "dk1"), ("tx1", "lt1"), ("evil", "dk1"), ("bg2", "nonsense")];
    let m = clean_map(attrs.into_iter(), &default_map());
    assert_that!((
        m.get("bg1").map(String::as_str),
        m.get("tx1").map(String::as_str),
        m.get("bg2").map(String::as_str),
        m.len()
    ))
    .is_equal_to((Some("dk1"), Some("lt1"), Some("lt2"), 12));
}
