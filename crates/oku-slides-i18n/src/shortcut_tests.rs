use speculoos::prelude::*;

use super::theme_shortcuts;
use crate::available;

#[test]
fn theme_shortcuts_has_every_shipped_locale() {
    assert_that!(theme_shortcuts().len()).is_equal_to(available().len());
}

#[test]
fn theme_shortcuts_holds_each_locale_own_words() {
    assert_that!(theme_shortcuts().get("de").map(String::as_str))
        .is_equal_to(Some("Thema, Themen, Design, theme, themes"));
}
