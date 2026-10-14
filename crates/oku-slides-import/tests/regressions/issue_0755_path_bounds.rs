//! Régression (t-755) : un tracé libre de boîte `w="1e-300"` donnait des coordonnées infinies.

use oku_slides_import::read_pptx;
use oku_slides_import::source::{Geometry, ItemKind, PathCmd};
use speculoos::prelude::*;

use crate::common;

fn numbers(c: &PathCmd) -> Vec<f64> {
    match *c {
        PathCmd::MoveTo { x, y } | PathCmd::LineTo { x, y } => vec![x, y],
        PathCmd::CubicTo { x1, y1, x2, y2, x, y } => vec![x1, y1, x2, y2, x, y],
        PathCmd::QuadTo { x1, y1, x, y } => vec![x1, y1, x, y],
        PathCmd::ArcTo { wr, hr, start, swing } => vec![wr, hr, start, swing],
        PathCmd::Close => vec![],
    }
}

#[test]
fn custom_path_with_a_degenerate_box_has_finite_bounded_coordinates() {
    let sp = r#"<p:sp><p:nvSpPr><p:cNvPr id="40" name="Libre"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="127000" cy="127000"/></a:xfrm><a:custGeom><a:pathLst><a:path w="1e-300" h="1e-300"><a:moveTo><a:pt x="5" y="5"/></a:moveTo><a:arcTo wR="3" hR="3" stAng="0" swAng="5400000"/><a:lnTo><a:pt x="1e308" y="-1e308"/></a:lnTo></a:path></a:pathLst></a:custGeom><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></p:spPr></p:sp>"#;
    let deck =
        read_pptx(&common::minimal_slide1(sp, ""), &common::limits()).unwrap_or_else(|e| panic!("lecture : {e}"));
    let shape = deck.slides[0].items.iter().find(|i| i.id == "s1/40").unwrap_or_else(|| panic!("forme absente"));
    let ItemKind::Shape(s) = &shape.kind else { panic!("forme attendue") };
    let Geometry::Path { paths } = &s.geom else { panic!("tracé attendu") };
    let values: Vec<f64> = paths.iter().flat_map(|p| p.cmds.iter().flat_map(numbers)).collect();
    assert_that!(values.len()).is_equal_to(8);
    assert_that!(values.iter().all(|v| v.is_finite() && v.abs() <= 1000.0)).is_true();
}
