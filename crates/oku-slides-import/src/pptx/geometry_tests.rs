use speculoos::prelude::*;

use super::*;
use crate::pptx::xml::parse;
use crate::source::{Geometry, PathCmd, SubPath};

fn geom(inner: &str) -> Option<Geometry> {
    let xml = format!(r#"<p:spPr xmlns:p="urn:p" xmlns:a="urn:a">{inner}</p:spPr>"#);
    let d = parse(&xml, "t", 1_000_000).unwrap();
    parse_geometry(d.root_element())
}

#[test]
fn preset_reads_name_and_adjustments() {
    let g = geom(r#"<a:prstGeom prst="roundRect"><a:avLst><a:gd name="adj" fmla="val 25000"/></a:avLst></a:prstGeom>"#);
    let Some(Geometry::Preset { name, adj }) = g else { panic!("preset attendu") };
    assert_that!((name.as_str(), adj.get("adj").copied())).is_equal_to(("roundRect", Some(25000.0)));
}

#[test]
fn custom_path_is_normalized_to_the_box() {
    let g = geom(
        r#"<a:custGeom><a:pathLst><a:path w="200" h="100"><a:moveTo><a:pt x="0" y="0"/></a:moveTo><a:lnTo><a:pt x="200" y="50"/></a:lnTo><a:close/></a:path></a:pathLst></a:custGeom>"#,
    );
    let Some(Geometry::Path { paths }) = g else { panic!("tracé attendu") };
    assert_that!(paths[0].cmds.clone()).is_equal_to(vec![
        PathCmd::MoveTo { x: 0.0, y: 0.0 },
        PathCmd::LineTo { x: 1.0, y: 0.5 },
        PathCmd::Close,
    ]);
}

#[test]
fn custom_path_keeps_fill_and_stroke_flags() {
    let g = geom(
        r#"<a:custGeom><a:pathLst><a:path><a:close/></a:path><a:path fill="none"><a:close/></a:path><a:path fill="darken" stroke="false"><a:close/></a:path></a:pathLst></a:custGeom>"#,
    );
    let Some(Geometry::Path { paths }) = g else { panic!("tracé attendu") };
    let flags: Vec<(bool, bool)> = paths.iter().map(|p| (p.fill, p.stroke)).collect();
    assert_that!(flags).is_equal_to(vec![(true, true), (false, true), (true, false)]);
    assert_that!(paths[0].clone()).is_equal_to(SubPath { cmds: vec![PathCmd::Close], fill: true, stroke: true });
}

#[test]
fn custom_path_is_capped_in_paths_and_commands() {
    let cmds = r#"<a:lnTo><a:pt x="1" y="1"/></a:lnTo>"#.repeat(MAX_PATH_CMDS);
    let paths = format!(r#"<a:path w="1" h="1">{cmds}</a:path>"#).repeat(2)
        + &r#"<a:path w="1" h="1"><a:close/></a:path>"#.repeat(MAX_PATHS + 5);
    let Some(Geometry::Path { paths }) = geom(&format!("<a:custGeom><a:pathLst>{paths}</a:pathLst></a:custGeom>"))
    else {
        panic!("tracé attendu")
    };
    assert_that!(paths.len() <= MAX_PATHS && paths.iter().map(|p| p.cmds.len()).sum::<usize>() <= MAX_PATH_CMDS)
        .is_true();
}

#[test]
fn no_geometry_is_none() {
    assert_that!(geom("<a:noFill/>")).is_none();
}

#[test]
fn custom_path_with_a_tiny_box_or_huge_values_stays_finite_and_bounded() {
    let g = geom(
        r#"<a:custGeom><a:pathLst><a:path w="1e-300" h="0.5"><a:moveTo><a:pt x="1" y="1"/></a:moveTo><a:lnTo><a:pt x="-1e300" y="1e300"/></a:lnTo><a:arcTo wR="1e300" hR="-4" stAng="0" swAng="5400000"/></a:path></a:pathLst></a:custGeom>"#,
    );
    let Some(Geometry::Path { paths }) = g else { panic!("tracé attendu") };
    assert_that!(paths[0].cmds.clone()).is_equal_to(vec![
        PathCmd::MoveTo { x: 1.0, y: 1.0 },
        PathCmd::LineTo { x: -MAX_PATH_SPAN, y: MAX_PATH_SPAN },
        PathCmd::ArcTo { wr: MAX_PATH_SPAN, hr: 0.0, start: 0.0, swing: 90.0 },
    ]);
}

/// Un `custGeom` d'une forme de 200 × 100 EMU, guides `guides` (`avLst` puis `gdLst`), un tracé `path` de même boîte.
fn custom(av: &str, gd: &str, path: &str) -> Vec<PathCmd> {
    let g = geom(&format!(
        r#"<a:xfrm><a:off x="0" y="0"/><a:ext cx="200" cy="100"/></a:xfrm><a:custGeom><a:avLst>{av}</a:avLst><a:gdLst>{gd}</a:gdLst><a:pathLst><a:path w="200" h="100">{path}</a:path></a:pathLst></a:custGeom>"#
    ));
    let Some(Geometry::Path { paths }) = g else { panic!("tracé attendu") };
    paths[0].cmds.clone()
}

#[test]
fn custom_point_naming_a_guide_takes_its_value() {
    let cmds = custom(
        "",
        r#"<a:gd name="x1" fmla="*/ w 1 4"/><a:gd name="y1" fmla="+- vc 0 0"/>"#,
        r#"<a:moveTo><a:pt x="x1" y="y1"/></a:moveTo><a:lnTo><a:pt x="r" y="b"/></a:lnTo>"#,
    );
    assert_that!(cmds).is_equal_to(vec![PathCmd::MoveTo { x: 0.25, y: 0.5 }, PathCmd::LineTo { x: 1.0, y: 1.0 }]);
}

#[test]
fn custom_guides_follow_av_then_gd_order() {
    // `g` nomme l'ajustement `adj` (avLst), lui-même évalué avant les guides.
    let cmds = custom(
        r#"<a:gd name="adj" fmla="val 75000"/>"#,
        r#"<a:gd name="g" fmla="*/ w adj 100000"/><a:gd name="h2" fmla="pin 0 g 120"/>"#,
        r#"<a:moveTo><a:pt x="g" y="0"/></a:moveTo><a:lnTo><a:pt x="h2" y="0"/></a:lnTo>"#,
    );
    assert_that!(cmds).is_equal_to(vec![PathCmd::MoveTo { x: 0.75, y: 0.0 }, PathCmd::LineTo { x: 0.6, y: 0.0 }]);
}

#[test]
fn custom_arc_reads_guides_for_radii_and_angles() {
    let cmds = custom(
        "",
        r#"<a:gd name="wr" fmla="*/ w 1 2"/><a:gd name="sw" fmla="+- cd4 0 0"/>"#,
        r#"<a:moveTo><a:pt x="0" y="0"/></a:moveTo><a:arcTo wR="wr" hR="hd2" stAng="cd2" swAng="sw"/>"#,
    );
    assert_that!(cmds[1].clone()).is_equal_to(PathCmd::ArcTo { wr: 0.5, hr: 0.5, start: 180.0, swing: 90.0 });
}

#[rstest::rstest]
#[case::undefined_name(r#"<a:gd name="g" fmla="*/ w 1 2"/>"#, "nope")]
#[case::unreadable_formula(r#"<a:gd name="g" fmla="bogus 1 2"/>"#, "g")]
#[case::guide_naming_a_later_one(r#"<a:gd name="g" fmla="val later"/><a:gd name="later" fmla="val 100"/>"#, "g")]
fn custom_point_with_an_unresolved_name_is_zero(#[case] gd: &str, #[case] x: &str) {
    let cmds = custom("", gd, &format!(r#"<a:moveTo><a:pt x="{x}" y="0"/></a:moveTo>"#));
    assert_that!(cmds).is_equal_to(vec![PathCmd::MoveTo { x: 0.0, y: 0.0 }]);
}

#[test]
fn custom_guides_beyond_the_cap_are_not_evaluated() {
    let gd: String = (0..=MAX_GUIDES).map(|i| format!(r#"<a:gd name="g{i}" fmla="val 100"/>"#)).collect();
    let path =
        format!(r#"<a:moveTo><a:pt x="g{}" y="0"/></a:moveTo><a:lnTo><a:pt x="g0" y="0"/></a:lnTo>"#, MAX_GUIDES);
    let cmds = custom("", &gd, &path);
    assert_that!(cmds).is_equal_to(vec![PathCmd::MoveTo { x: 0.0, y: 0.0 }, PathCmd::LineTo { x: 0.5, y: 0.0 }]);
}

#[test]
fn custom_point_from_a_huge_guide_is_clamped_and_reported() {
    let xml = r#"<p:spPr xmlns:p="urn:p" xmlns:a="urn:a"><a:custGeom><a:gdLst><a:gd name="far" fmla="*/ w 1000000 1"/></a:gdLst><a:pathLst><a:path w="1" h="1"><a:moveTo><a:pt x="far" y="0"/></a:moveTo></a:path></a:pathLst></a:custGeom></p:spPr>"#;
    let d = parse(xml, "t", 1_000_000).unwrap();
    let path = d.descendants().find(|n| n.tag_name().name() == "path").unwrap();
    assert_that!(path_clamped(path)).is_true();
    let Some(Geometry::Path { paths }) = parse_geometry(d.root_element()) else { panic!("tracé attendu") };
    assert_that!(paths[0].cmds.clone()).is_equal_to(vec![PathCmd::MoveTo { x: MAX_PATH_SPAN, y: 0.0 }]);
}
