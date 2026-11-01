use oku_slides_i18n::Localizer;
use speculoos::prelude::*;

use super::permission_note;

#[test]
fn permission_note_without_received_guides_is_none() {
    assert_that!(permission_note(&[], &Localizer::new(Some("en")))).is_none();
}

#[test]
fn permission_note_names_the_received_guides() {
    let names = ["Acme".to_owned(), "Beta".to_owned()];
    let note = permission_note(&names, &Localizer::new(Some("en")));
    assert_that!(note.as_deref().unwrap_or_default()).contains("received guide of");
    assert_that!(note.as_deref().unwrap_or_default()).contains("Acme, Beta");
}

#[test]
fn permission_note_is_translated() {
    let names = ["Acme".to_owned()];
    let fr = permission_note(&names, &Localizer::new(Some("fr")));
    let en = permission_note(&names, &Localizer::new(Some("en")));
    assert_that!(fr.clone()).is_not_equal_to(en);
    assert_that!(fr.unwrap_or_default()).contains("Acme");
}
