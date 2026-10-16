use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::pptx::xml::parse;

fn clamped_in(body: &str) -> Vec<&'static str> {
    let xml = format!(r#"<r xmlns:a="urn:a">{body}</r>"#);
    let doc = parse(&xml, "t", 10_000).unwrap();
    clamped(doc.root_element())
}

#[rstest]
#[case::size(r#"<a:rPr sz="50"/>"#, "sz")]
#[case::default_run_spacing(r#"<a:defRPr spc="500000"/>"#, "spc")]
#[case::spacing_points(r#"<a:spcPts val="-1"/>"#, "spcPts")]
#[case::spacing_percent(r#"<a:spcPct val="99999999"/>"#, "spcPct")]
#[case::bullet_size(r#"<a:buSzPct val="1"/>"#, "buSzPct")]
#[case::font_scale(r#"<a:normAutofit fontScale="200000"/>"#, "fontScale")]
#[case::line_reduction(r#"<a:normAutofit lnSpcReduction="-5"/>"#, "lnSpcReduction")]
#[case::level_margin(r#"<a:lvl3pPr marL="-1"/>"#, "marL")]
#[case::indent(r#"<a:defPPr indent="99999999999"/>"#, "indent")]
#[case::gradient_stop(r#"<a:gs pos="120000"/>"#, "gs pos")]
#[case::path_point(
    r#"<a:pathLst><a:path w="10" h="10"><a:lnTo><a:pt x="20000" y="0"/></a:lnTo></a:path></a:pathLst>"#,
    "custGeom"
)]
#[case::unknown_color_modifier(r#"<a:srgbClr val="FF0000"><a:hueOff val="10"/></a:srgbClr>"#, "color modifier")]
#[case::arc_radius(r#"<a:pathLst><a:path w="1" h="1"><a:arcTo wR="-3" hR="0"/></a:path></a:pathLst>"#, "custGeom")]
fn clamped_value_out_of_its_range_is_reported_by_kind(#[case] body: &str, #[case] label: &str) {
    assert_that!(clamped_in(body)).is_equal_to(vec![label]);
}

#[test]
fn clamped_values_at_their_bounds_are_not_reported() {
    let body = r#"<a:rPr sz="100" spc="-400000"/><a:gs pos="100000"/><a:pathLst><a:path w="10" h="10"><a:lnTo><a:pt x="10000" y="-10000"/></a:lnTo></a:path></a:pathLst>"#;
    assert_that!(clamped_in(body)).is_empty();
}

#[test]
fn clamped_kind_seen_twice_is_reported_once() {
    assert_that!(clamped_in(r#"<a:rPr sz="1"/><a:rPr sz="2"/>"#)).is_equal_to(vec!["sz"]);
}

#[test]
fn clamped_attribute_on_an_element_the_reader_ignores_is_not_reported() {
    // `path` d'un dégradé (`a:path path="circle"`) n'est pas un tracé `custGeom`.
    assert_that!(clamped_in(r#"<a:sp sz="1"/><a:gradFill><a:path w="1"><a:pt x="9999"/></a:path></a:gradFill>"#))
        .is_empty();
}

#[test]
fn known_color_modifiers_are_not_reported() {
    let body = r#"<a:schemeClr val="accent1"><a:lumMod val="75000"/><a:alpha val="50000"/></a:schemeClr>"#;
    assert_that!(clamped_in(body)).is_empty();
}

#[test]
fn color_modifiers_beyond_the_cap_are_reported() {
    let mods = r#"<a:tint val="1"/>"#.repeat(MAX_COLOR_MODS + 1);
    assert_that!(clamped_in(&format!(r#"<a:srgbClr val="000000">{mods}</a:srgbClr>"#)))
        .is_equal_to(vec!["color modifier"]);
}
