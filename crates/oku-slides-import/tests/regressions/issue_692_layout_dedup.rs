//! Régression (t-692) : un master dont les relations citent deux fois le même layout le relisait (deux `Layout`).

use oku_slides_import::read_pptx;
use speculoos::prelude::*;

use crate::common;

#[test]
fn layout_listed_twice_by_its_master_is_read_once() {
    let rels = common::text("minimal/ppt/slideMasters/_rels/slideMaster1.xml.rels").replace(
        "</Relationships>",
        r#"<Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/></Relationships>"#,
    );
    let deck = read_pptx(
        &common::pptx_with("minimal", &[("ppt/slideMasters/_rels/slideMaster1.xml.rels", Some(&rels))]),
        &common::limits(),
    )
    .unwrap_or_else(|e| panic!("lecture : {e}"));
    assert_that!(deck.layouts.iter().map(|l| l.id.clone()).collect::<Vec<_>>())
        .is_equal_to(vec!["l1".to_owned(), "l2".to_owned()]);
}
