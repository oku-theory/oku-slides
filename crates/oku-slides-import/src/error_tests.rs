use oku_slides_i18n::{Localize, Localizer, english};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case(ImportErrorKind::Unreadable)]
#[case(ImportErrorKind::NotAPresentation)]
#[case(ImportErrorKind::Protected)]
#[case(ImportErrorKind::TooLarge)]
#[case(ImportErrorKind::NoProfiles)]
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

#[test]
fn output_too_large_kind_is_too_large() {
    assert_that!(output_too_large(10).kind()).is_equal_to(ImportErrorKind::TooLarge);
}

#[test]
fn output_too_large_message_key_exists_in_english() {
    assert_that!(english().try_tr(OUTPUT_TOO_LARGE)).is_some();
}

#[test]
fn localize_output_too_large_in_french_names_the_output() {
    assert_that!(output_too_large(10).localize(&Localizer::new(Some("fr"))))
        .is_equal_to("Le deck obtenu dépasserait la taille admise pour un import".to_owned());
}

#[test]
fn no_profiles_kind_is_no_profiles() {
    assert_that!(no_profiles().kind()).is_equal_to(ImportErrorKind::NoProfiles);
}

#[test]
fn localize_no_profiles_in_french_names_the_layout_packs() {
    assert_that!(no_profiles().localize(&Localizer::new(Some("fr"))))
        .is_equal_to("Les packs de layouts choisis ne proposent aucun profil.".to_owned());
}
