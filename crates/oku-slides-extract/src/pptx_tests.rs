use speculoos::prelude::*;

use super::*;

#[test]
fn skipped_list_names_what_was_met_in_order() {
    let none = Skipped::default();
    let mut some = Skipped::default();
    some.met(Lost::Parts);
    some.met(Lost::HiddenSlides);
    some.met(Lost::Objects);
    some.met(Lost::Parts);
    assert_that!(none.list().is_empty()).is_true();
    assert_that!(some.list()).is_equal_to(vec![
        "hidden slides",
        "media and embedded objects",
        "unreadable or missing parts",
    ]);
}
