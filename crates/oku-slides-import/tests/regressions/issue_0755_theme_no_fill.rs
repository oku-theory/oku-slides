//! Régression (t-755) : `bgRef idx="1000"` (aucun fond) tombait hors de la liste et héritait du fond du master.

use oku_slides_import::read_pptx;
use oku_slides_import::source::Fill;
use speculoos::prelude::*;

use crate::common;

#[test]
fn slide_background_reference_1000_is_no_fill() {
    let slide = common::text("minimal/ppt/slides/slide1.xml")
        .replace("<p:cSld>", r#"<p:cSld><p:bg><p:bgRef idx="1000"><a:schemeClr val="bg1"/></p:bgRef></p:bg>"#);
    let deck = read_pptx(&common::pptx_with("minimal", &[("ppt/slides/slide1.xml", Some(&slide))]), &common::limits())
        .unwrap_or_else(|e| panic!("lecture : {e}"));
    assert_that!(deck.slides[0].background.clone()).is_equal_to(Fill::None);
}
