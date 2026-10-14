//! Régression (t-755) : une image seulement liée (`r:link`, hors du fichier) disparaissait sans avertissement.

use oku_slides_import::read_pptx;
use oku_slides_import::source::WarningKind;
use speculoos::prelude::*;

use crate::common;

#[test]
fn linked_external_picture_is_reported() {
    let pic = r#"<p:pic><p:nvPicPr><p:cNvPr id="40" name="Lien"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:link="rIdX"/></p:blipFill><p:spPr/></p:pic>"#;
    let rel = r#"<Relationship Id="rIdX" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="file:///C:/photos/a.png" TargetMode="External"/>"#;
    let deck =
        read_pptx(&common::minimal_slide1(pic, rel), &common::limits()).unwrap_or_else(|e| panic!("lecture : {e}"));
    let found: Vec<_> = deck
        .warnings
        .iter()
        .filter_map(|w| match &w.kind {
            WarningKind::ExternalImage(target) => Some((w.slide, w.item.clone(), target.clone())),
            _ => None,
        })
        .collect();
    assert_that!(found).is_equal_to(vec![(Some(1), Some("s1/40".to_owned()), "file:///C:/photos/a.png".to_owned())]);
}
