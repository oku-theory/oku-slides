//! Régression (t-755) : tailles, marges et espacements du texte acceptaient tout entier (tailles négatives).

use oku_slides_import::read_pptx;
use oku_slides_import::source::ItemKind;
use speculoos::prelude::*;

use crate::common;

#[test]
fn negative_size_and_huge_margin_are_brought_back_into_the_schema_range() {
    let sp = r#"<p:sp><p:nvSpPr><p:cNvPr id="40" name="Texte"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1270000" cy="1270000"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr><p:txBody><a:bodyPr/><a:p><a:pPr marL="999999999999" indent="-999999999999"/><a:r><a:rPr sz="-500" spc="99999999"/><a:t>x</a:t></a:r></a:p></p:txBody></p:sp>"#;
    let deck =
        read_pptx(&common::minimal_slide1(sp, ""), &common::limits()).unwrap_or_else(|e| panic!("lecture : {e}"));
    let item = deck.slides[0].items.iter().find(|i| i.id == "s1/40").unwrap_or_else(|| panic!("texte absent"));
    let ItemKind::Text(tb) = &item.kind else { panic!("texte attendu") };
    let p = &tb.paragraphs[0];
    assert_that!((p.style.margin_left, p.style.indent, p.runs[0].style.size, p.runs[0].style.char_spacing))
        .is_equal_to((4032.0, -4032.0, 1.0, 4000.0));
}
