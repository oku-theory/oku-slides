//! Régression (t-892) : une valeur ramenée dans la plage du schéma (t-755) l'était en silence.

use oku_slides_import::read_pptx;
use oku_slides_import::source::WarningKind;
use speculoos::prelude::*;

use crate::common;

fn text_shape(id: u32, ppr: &str, rpr: &str) -> String {
    format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="Texte"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1270000" cy="1270000"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr><p:txBody><a:bodyPr/><a:p><a:pPr {ppr}/><a:r><a:rPr {rpr}/><a:t>x</a:t></a:r></a:p></p:txBody></p:sp>"#
    )
}

#[test]
fn clamped_values_warn_approximated_once_per_kind_on_their_slide() {
    let shapes = [
        text_shape(40, r#"marL="999999999999" indent="-999999999999""#, r#"sz="-500" spc="99999999""#),
        text_shape(41, "", r#"sz="-7""#),
    ]
    .concat();
    let deck =
        read_pptx(&common::minimal_slide1(&shapes, ""), &common::limits()).unwrap_or_else(|e| panic!("lecture : {e}"));
    let approximated: Vec<(Option<u32>, &str)> = deck
        .warnings
        .iter()
        .filter_map(|w| match &w.kind {
            WarningKind::Approximated(d) => Some((w.slide, d.as_str())),
            _ => None,
        })
        .collect();
    assert_that!(approximated).is_equal_to(vec![
        (Some(1), "sz"),
        (Some(1), "spc"),
        (Some(1), "marL"),
        (Some(1), "indent"),
    ]);
}

#[test]
fn values_within_the_schema_range_warn_nothing() {
    let deck =
        read_pptx(&common::minimal_slide1(&text_shape(40, r#"marL="0""#, r#"sz="1800""#), ""), &common::limits())
            .unwrap_or_else(|e| panic!("lecture : {e}"));
    assert_that!(deck.warnings.iter().any(|w| matches!(w.kind, WarningKind::Approximated(_)))).is_false();
}
