use speculoos::prelude::*;

use super::checked;

#[test]
fn checked_relative_path_is_refused() {
    assert_that!(checked("deck.oku-slides.pdf")).is_err();
}

#[test]
fn checked_parent_component_is_refused() {
    assert_that!(checked("/Users/x/../y/deck.pdf")).is_err();
}

#[test]
fn checked_absolute_path_drops_current_dir_components() {
    assert_that!(checked("/Users/x/./deck.pdf")).is_ok().is_equal_to(std::path::PathBuf::from("/Users/x/deck.pdf"));
}
