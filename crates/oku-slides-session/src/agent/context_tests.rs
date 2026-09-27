use speculoos::prelude::*;

use super::turn_label;

#[test]
fn turn_label_keeps_short_prompt() {
    assert_that!(turn_label("titre plus court")).is_equal_to("titre plus court".to_owned());
}

#[test]
fn turn_label_cuts_long_prompt_at_forty_chars_with_ellipsis() {
    let long = "rends ce titre beaucoup plus percutant et essaie trois mises en page";
    let label = turn_label(long);
    assert_that!(label.chars().count()).is_equal_to(41);
    assert_that!(label.ends_with('…')).is_true();
}
