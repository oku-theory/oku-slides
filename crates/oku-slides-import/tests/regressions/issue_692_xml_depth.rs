//! Régression (t-692) : un XML imbriqué sur des milliers de niveaux débordait la pile de roxmltree.

use oku_slides_import::{ImportErrorKind, ReadLimits, read_pptx};
use speculoos::prelude::*;

use crate::common;

#[test]
fn deeply_nested_part_is_refused_not_crashed() {
    let deep = format!("{}{}", "<a>".repeat(20_000), "</a>".repeat(20_000));
    let bytes = common::pptx_with("minimal", &[("ppt/presentation.xml", Some(&deep))]);
    match read_pptx(&bytes, &ReadLimits::default()) {
        Err(e) => assert_that!(e.kind()).is_equal_to(ImportErrorKind::Unreadable),
        Ok(_) => panic!("refus attendu"),
    }
}
