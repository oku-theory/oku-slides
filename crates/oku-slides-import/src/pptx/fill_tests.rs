use std::collections::BTreeMap;

use speculoos::prelude::*;

use super::*;
use crate::pptx::color::{ColorBase, ColorCtx, default_map};
use crate::pptx::xml::parse;
use crate::source::{Dash, Fill, Origin, Rgba, Styled};

fn ctx() -> ColorCtx {
    ColorCtx::new(
        BTreeMap::from([
            ("accent1".to_owned(), Rgba::new(0x44, 0x72, 0xc4)),
            ("lt1".to_owned(), Rgba::new(255, 255, 255)),
        ]),
        default_map(),
    )
}

fn with_sp_pr<R>(inner: &str, f: impl FnOnce(roxmltree::Node<'_, '_>) -> R) -> R {
    let xml = format!(
        r#"<p:spPr xmlns:p="urn:p" xmlns:a="urn:a" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">{inner}</p:spPr>"#
    );
    let d = parse(&xml, "t", 100_000).unwrap();
    f(d.root_element())
}

fn fill_of(inner: &str) -> Option<Fill> {
    with_sp_pr(inner, |n| parse_fill(n, &mut |id| Some(format!("asset-of-{id}")))).map(|s| s.resolve(&ctx()))
}

fn line_of(inner: &str) -> LineSpec {
    let xml = format!(r#"<a:ln xmlns:a="urn:a" {inner}"#);
    let d = parse(&xml, "t", 10_000).unwrap();
    parse_line(d.root_element())
}

fn explicit(r: u8, g: u8, b: u8) -> ColorSpec {
    ColorSpec { base: ColorBase::Fixed(Rgba::new(r, g, b)), mods: vec![] }
}

#[test]
fn no_fill_child_means_inherit() {
    assert_that!(fill_of(r#"<a:prstGeom prst="rect"/>"#)).is_none();
}

#[test]
fn no_fill_is_fill_none() {
    assert_that!(fill_of("<a:noFill/>")).is_equal_to(Some(Fill::None));
}

#[test]
fn solid_fill_resolves_its_color() {
    assert_that!(fill_of(r#"<a:solidFill><a:schemeClr val="accent1"/></a:solidFill>"#))
        .is_equal_to(Some(Fill::Solid { color: Styled::scheme(Rgba::new(0x44, 0x72, 0xc4), "accent1") }));
}

#[test]
fn solid_fill_is_resolved_by_the_consumer_color_map() {
    let spec =
        with_sp_pr(r#"<a:solidFill><a:schemeClr val="bg1"/></a:solidFill>"#, |n| parse_fill(n, &mut |_| None)).unwrap();
    let mut map = default_map();
    map.insert("bg1".into(), "accent1".into());
    let Fill::Solid { color } = spec.resolve(&ColorCtx::new((*ctx().scheme).clone(), map)) else {
        panic!("plein attendu")
    };
    assert_that!(color.origin).is_equal_to(Origin::Scheme { name: "bg1".into(), slot: "accent1".into(), mods: vec![] });
}

#[test]
fn gradient_fill_reads_stops_colors_and_angle() {
    let f = fill_of(
        r#"<a:gradFill><a:gsLst><a:gs pos="0"><a:srgbClr val="FFFFFF"/></a:gs><a:gs pos="100000"><a:srgbClr val="000000"/></a:gs></a:gsLst><a:lin ang="5400000"/></a:gradFill>"#,
    );
    let Some(Fill::Gradient { angle, stops }) = f else { panic!("dégradé attendu, trouvé {f:?}") };
    let got: Vec<(f64, String)> = stops.iter().map(|s| (s.pos, s.color.value.to_hex())).collect();
    assert_that!((angle, got)).is_equal_to((90.0, vec![(0.0, "#ffffff".to_owned()), (1.0, "#000000".to_owned())]));
}

#[test]
fn gradient_stops_beyond_the_cap_are_dropped_and_reported() {
    let stops = r#"<a:gs pos="0"><a:srgbClr val="FFFFFF"/></a:gs>"#.repeat(MAX_GRADIENT_STOPS + 10);
    let xml = format!("<a:gradFill><a:gsLst>{stops}</a:gsLst></a:gradFill>");
    let Some(Fill::Gradient { stops, .. }) = fill_of(&xml) else { panic!("dégradé attendu") };
    assert_that!(stops.len()).is_equal_to(MAX_GRADIENT_STOPS);
    assert_that!(with_sp_pr(&xml, approximation)).is_equal_to(Some("gradFill stops"));
}

#[test]
fn blip_fill_maps_the_relationship_to_an_asset() {
    assert_that!(fill_of(r#"<a:blipFill><a:blip r:embed="rId4"/><a:stretch/></a:blipFill>"#))
        .is_equal_to(Some(Fill::Picture { asset: "asset-of-rId4".into() }));
}

#[test]
fn group_fill_means_inherit_from_group() {
    assert_that!(fill_of("<a:grpFill/>")).is_none();
}

#[test]
fn placeholder_fill_takes_the_reference_color() {
    let spec = with_sp_pr(r#"<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>"#, |n| parse_fill(n, &mut |_| None))
        .unwrap();
    let ph = ColorSpec { base: ColorBase::Scheme("accent1".into()), mods: vec![] };
    assert_that!(spec.with_placeholder(Some(&ph)).resolve(&ctx()))
        .is_equal_to(Fill::Solid { color: Styled::scheme(Rgba::new(0x44, 0x72, 0xc4), "accent1") });
    assert_that!(spec.resolve(&ctx())).is_equal_to(Fill::None);
}

#[test]
fn pattern_and_path_gradient_are_reported_as_approximations() {
    assert_that!(with_sp_pr(r#"<a:pattFill prst="pct5"/>"#, approximation)).is_equal_to(Some("pattFill"));
    assert_that!(with_sp_pr(r#"<a:gradFill><a:path path="circle"/></a:gradFill>"#, approximation))
        .is_equal_to(Some("gradFill path"));
    assert_that!(with_sp_pr("<a:noFill/>", approximation)).is_none();
}

#[test]
fn line_over_base_keeps_base_color_when_absent() {
    let base = LineSpec { width: None, color: Some(Some(explicit(1, 2, 3))), dash: None };
    let s = line_of(r#"w="25400"><a:prstDash val="dash"/></a:ln>"#).over(&base).finish(&ctx()).unwrap();
    assert_that!((s.width, s.dash, s.color.value)).is_equal_to((2.0, Dash::Dash, Rgba::new(1, 2, 3)));
}

#[test]
fn line_no_fill_over_colored_base_has_no_stroke() {
    let base = LineSpec { width: Some(1.0), color: Some(Some(explicit(1, 2, 3))), dash: None };
    assert_that!(line_of(r#"w="12700"><a:noFill/></a:ln>"#).over(&base).finish(&ctx())).is_none();
}

#[test]
fn empty_line_element_inherits_everything() {
    assert_that!(line_of("></a:ln>")).is_equal_to(LineSpec::default());
}

#[test]
fn gradient_stop_positions_are_clamped_to_zero_one() {
    let f = fill_of(
        r#"<a:gradFill><a:gsLst><a:gs pos="-50000"><a:srgbClr val="FFFFFF"/></a:gs><a:gs pos="250000"><a:srgbClr val="000000"/></a:gs></a:gsLst></a:gradFill>"#,
    );
    let Some(Fill::Gradient { stops, .. }) = f else { panic!("dégradé attendu, trouvé {f:?}") };
    assert_that!(stops.iter().map(|s| s.pos).collect::<Vec<_>>()).is_equal_to(vec![0.0, 1.0]);
}
