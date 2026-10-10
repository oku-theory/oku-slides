use speculoos::prelude::*;

use super::*;
use crate::pptx::xml::parse;
use crate::source::{Geometry, PathCmd};

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
    assert_that!(paths[0].clone()).is_equal_to(vec![
        PathCmd::MoveTo { x: 0.0, y: 0.0 },
        PathCmd::LineTo { x: 1.0, y: 0.5 },
        PathCmd::Close,
    ]);
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
    assert_that!(paths.len() <= MAX_PATHS && paths.iter().map(Vec::len).sum::<usize>() <= MAX_PATH_CMDS).is_true();
}

#[test]
fn no_geometry_is_none() {
    assert_that!(geom("<a:noFill/>")).is_none();
}
