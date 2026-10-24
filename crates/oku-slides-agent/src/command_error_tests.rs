use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

use super::{CommandError, DraftTooLongSnafu, ImageGoneSnafu, TooManyImagesSnafu, UnknownAgentSnafu};

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
fn too_many_images_is_said_in_the_user_language() {
    let e = CommandError::from(TooManyImagesSnafu { max: 8usize }.build());
    assert_that!(e.localize(&Localizer::new(Some("fr"))))
        .is_equal_to("Le brouillon a déjà toutes ses images.".to_owned());
}

#[test]
fn image_gone_is_said_in_the_user_language_without_its_id() {
    let e = CommandError::from(ImageGoneSnafu { id: "sha256:aa" }.build());
    assert_that!(e.localize(&Localizer::new(Some("fr"))))
        .is_equal_to("Une image collée n'est plus disponible : collez-la de nouveau.".to_owned());
}
