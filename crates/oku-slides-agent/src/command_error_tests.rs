use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

use super::{AgentLockedSnafu, CommandError, DraftTooLongSnafu, UnknownAgentSnafu};

#[test]
fn draft_too_long_is_said_in_the_user_language() {
    let e = CommandError::from(DraftTooLongSnafu { bytes: 10usize, max: 5usize }.build());
    assert_that!(e.localize(&Localizer::new(Some("fr")))).is_equal_to("Le brouillon est trop long.".to_owned());
}

#[test]
fn unknown_agent_is_said_in_the_user_language() {
    let e = CommandError::from(UnknownAgentSnafu { id: "x" }.build());
    assert_that!(e.localize(&Localizer::new(Some("fr"))))
        .is_equal_to("Cet agent n'est plus dans la configuration.".to_owned());
}

#[test]
fn agent_locked_is_said_in_the_user_language_without_user_data() {
    let e = CommandError::from(AgentLockedSnafu.build());
    assert_that!(e.localize(&Localizer::new(Some("en"))))
        .is_equal_to("The agent choice is set by the environment.".to_owned());
}
