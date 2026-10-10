use oku_slides_i18n::{Localize, Localizer, english};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case(ImportErrorKind::Unreadable)]
#[case(ImportErrorKind::NotAPresentation)]
#[case(ImportErrorKind::Protected)]
#[case(ImportErrorKind::TooLarge)]
fn message_id_of_each_kind_exists_in_english(#[case] kind: ImportErrorKind) {
    assert_that!(english().try_tr(message_id(kind))).is_some();
}

#[test]
fn localize_unreadable_in_french_reads_naturally() {
    assert_that!(structure("x").localize(&Localizer::new(Some("fr"))))
        .is_equal_to("Cette présentation est illisible".to_owned());
}

#[test]
fn kind_of_xml_error_is_unreadable() {
    let err = roxmltree::Document::parse("<a>").unwrap_err();
    assert_that!(xml_error("p.xml", err).kind()).is_equal_to(ImportErrorKind::Unreadable);
}
