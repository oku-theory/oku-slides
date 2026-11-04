use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use super::{
    CommandError, DraftTooLongSnafu, ImageGoneSnafu, ReportElsewhereSnafu, SeqExhaustedSnafu, TooManyImagesSnafu,
    UiTooLargeSnafu, UnknownAgentSnafu,
};

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

#[rstest]
#[case(CommandError::from(ReportElsewhereSnafu.build()), "agent-error-report-elsewhere")]
#[case(CommandError::from(UiTooLargeSnafu.build()), "agent-error-ui-too-large")]
#[case(CommandError::from(SeqExhaustedSnafu { seq: u64::MAX }.build()), "agent-error-seq-exhausted")]
fn page_report_refusal_is_said_by_its_key_in_french(#[case] e: CommandError, #[case] key: &str) {
    let fr = Localizer::new(Some("fr"));
    assert_that!(fr.tr(key).as_str()).does_not_contain(key);
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr(key));
}
