use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

use super::{CommandError, DraftTooLongSnafu};

#[test]
fn draft_too_long_is_said_in_the_user_language() {
    let e = CommandError::from(DraftTooLongSnafu { bytes: 10usize, max: 5usize }.build());
    assert_that!(e.localize(&Localizer::new(Some("fr")))).is_equal_to("Le brouillon est trop long.".to_owned());
}
