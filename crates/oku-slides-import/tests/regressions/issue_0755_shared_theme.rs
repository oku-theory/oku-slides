//! Régression (t-755) : un thème partagé par plusieurs masters était lu et analysé une fois par master.

use oku_slides_import::read_pptx;
use oku_slides_import::source::WarningKind;
use speculoos::prelude::*;

use crate::common;

const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

#[test]
fn theme_shared_by_two_masters_is_parsed_once() {
    let pres = common::text("minimal/ppt/presentation.xml")
        .replace("</p:sldMasterIdLst>", r#"<p:sldMasterId id="2147483660" r:id="rId9"/></p:sldMasterIdLst>"#);
    let pres_rels = common::text("minimal/ppt/_rels/presentation.xml.rels").replace(
        "</Relationships>",
        &format!(r#"<Relationship Id="rId9" Type="{REL}/slideMaster" Target="slideMasters/slideMaster2.xml"/></Relationships>"#),
    );
    let master2 = common::text("minimal/ppt/slideMasters/slideMaster1.xml");
    let master2_rels = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{REL}/theme" Target="../theme/theme1.xml"/></Relationships>"#
    );
    let bytes = common::pptx_plus(
        "minimal",
        &[
            ("ppt/presentation.xml", Some(&pres)),
            ("ppt/_rels/presentation.xml.rels", Some(&pres_rels)),
            ("ppt/theme/theme1.xml", Some("<a:theme")),
        ],
        &[
            ("ppt/slideMasters/slideMaster2.xml", master2.as_bytes()),
            ("ppt/slideMasters/_rels/slideMaster2.xml.rels", master2_rels.as_bytes()),
        ],
    );
    let deck = read_pptx(&bytes, &common::limits()).unwrap_or_else(|e| panic!("lecture : {e}"));
    let theme_warnings = deck
        .warnings
        .iter()
        .filter(|w| matches!(&w.kind, WarningKind::UnreadablePart(p) if p.contains("theme1.xml")))
        .count();
    assert_that!((deck.themes.len(), theme_warnings)).is_equal_to((2, 1));
}
