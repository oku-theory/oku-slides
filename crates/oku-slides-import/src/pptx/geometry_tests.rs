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
