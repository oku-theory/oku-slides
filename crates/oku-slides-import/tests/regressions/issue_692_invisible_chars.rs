//! Régression (t-692) : `clip` laissait passer des caractères de mise en forme invisibles
//! (U+061C, U+2028, U+2029, U+180E, U+FFF9 à U+FFFB) venus du fichier jusque dans les avertissements.

use oku_slides_import::read_pptx;
use oku_slides_import::source::WarningKind;
use speculoos::prelude::*;

use crate::common;

const INVISIBLE: [char; 7] = ['\u{061c}', '\u{2028}', '\u{2029}', '\u{180e}', '\u{fff9}', '\u{fffa}', '\u{fffb}'];

#[test]
fn warning_about_a_part_name_has_no_invisible_formatting_character() {
    let name: String = INVISIBLE.iter().map(|c| format!("&#x{:04X};", u32::from(*c))).collect();
    let pic = r#"<p:pic><p:nvPicPr><p:cNvPr id="40" name="p"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rIdX"/></p:blipFill><p:spPr/></p:pic>"#;
    let slide = common::text("minimal/ppt/slides/slide1.xml").replace("</p:spTree>", &format!("{pic}</p:spTree>"));
    let rels = common::text("minimal/ppt/slides/_rels/slide1.xml.rels").replace(
        "</Relationships>",
        &format!(
            r#"<Relationship Id="rIdX" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/a{name}b.png"/></Relationships>"#
        ),
    );
    let deck = read_pptx(
        &common::pptx_with(
            "minimal",
            &[("ppt/slides/slide1.xml", Some(&slide)), ("ppt/slides/_rels/slide1.xml.rels", Some(&rels))],
        ),
        &common::limits(),
    )
    .unwrap_or_else(|e| panic!("lecture : {e}"));
    let missing: Vec<String> = deck
        .warnings
        .iter()
        .filter_map(|w| match &w.kind {
            WarningKind::MissingPart(p) => Some(p.clone()),
            _ => None,
        })
        .collect();
    assert_that!(missing).is_equal_to(vec!["ppt/media/ab.png".to_owned()]);
}
