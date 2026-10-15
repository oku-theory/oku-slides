use std::sync::Arc;

use speculoos::prelude::*;

use super::*;

#[test]
fn live_localizer_set_reaches_every_clone() {
    let live = LiveLocalizer::new(Arc::new(Localizer::new(Some("en"))));
    let clone = live.clone();
    live.set(Arc::new(Localizer::new(Some("fr"))));
    assert_that!(clone.current().locale()).is_equal_to("fr");
}

#[test]
fn live_localizer_current_keeps_the_language_it_was_read_in() {
    let live = LiveLocalizer::new(Arc::new(Localizer::new(Some("de"))));
    let before = live.current();
    live.set(Arc::new(Localizer::new(Some("ja"))));
    assert_that!(before.locale()).is_equal_to("de");
}
