//! Régression (t-754) : roxmltree vérifie les attributs en double d'un élément en O(n²) ;
//! un élément forgé à 50 000 attributs bloquait la lecture plusieurs secondes.

use oku_slides_import::source::WarningKind;
use oku_slides_import::{ImportErrorKind, ReadLimits, read_pptx};
use speculoos::prelude::*;

use crate::common;

/// Un élément `tag` vide portant `n` attributs distincts.
fn crowded(tag: &str, n: usize) -> String {
    let attrs: String = (0..n).map(|i| format!(" a{i}=\"\"")).collect();
    format!("<{tag}{attrs}/>")
}

#[test]
fn presentation_with_a_crowded_element_is_unreadable() {
    let pres = common::text("minimal/ppt/presentation.xml");
    let pres = pres.replacen("<p:sldSz", &format!("{}<p:sldSz", crowded("p:x", 50_000)), 1);
    let bytes = common::pptx_with("minimal", &[("ppt/presentation.xml", Some(&pres))]);
    match read_pptx(&bytes, &ReadLimits::default()) {
        Err(e) => assert_that!(e.kind()).is_equal_to(ImportErrorKind::Unreadable),
        Ok(_) => panic!("refus attendu"),
    }
}

#[test]
fn slide_with_a_crowded_element_is_an_unreadable_part() {
    let bytes = common::minimal_slide1(&crowded("p:x", 50_000), "");
    let deck = read_pptx(&bytes, &ReadLimits::default()).unwrap();
    let unreadable =
        deck.warnings.iter().any(|w| matches!(&w.kind, WarningKind::UnreadablePart(p) if p.contains("slide1")));
    assert_that!(unreadable).is_true();
}
