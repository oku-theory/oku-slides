//! Régression (t-692) : dépasser `max_xml_nodes` était dégradé en avertissement (part omise) au lieu d'être fatal.

use oku_slides_import::{ImportErrorKind, ReadLimits, read_pptx};
use speculoos::prelude::*;

use crate::common;

#[test]
fn xml_node_limit_is_too_large_not_an_omitted_part() {
    // `presentation.xml` tient dans la borne ; master, layouts et slides la dépassent.
    let limits = ReadLimits { max_xml_nodes: 40, ..ReadLimits::default() };
    match read_pptx(&common::pptx("minimal"), &limits) {
        Err(e) => assert_that!(e.kind()).is_equal_to(ImportErrorKind::TooLarge),
        Ok(d) => panic!("TooLarge attendu, lu {} slides", d.slides.len()),
    }
}
