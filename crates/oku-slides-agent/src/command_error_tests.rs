use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use super::{
    AttachmentGoneSnafu, AttachmentTooLargeSnafu, CommandError, DraftTooLongSnafu, ReportElsewhereSnafu,
    SeqExhaustedSnafu, TooManyAttachmentsSnafu, TooManyDocumentsSnafu, TooManyImagesSnafu, UiTooLargeSnafu,
    UnknownAgentSnafu,
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
fn too_many_documents_is_said_by_its_key_with_the_limit_in_french() {
    let e = CommandError::from(TooManyDocumentsSnafu { max: 4usize }.build());
    let fr = Localizer::new(Some("fr"));
    let expected = fr.tr_with("agent-error-too-many-documents", &[("max", 4u64.into())]);
    assert_that!(expected.as_str()).does_not_contain("agent-error-too-many-documents");
    assert_that!(e.localize(&fr)).is_equal_to(expected);
    assert_that!(e.localize(&fr))
        .is_equal_to("Le brouillon a déjà autant de documents qu'il peut en contenir (4).".to_owned());
}

#[test]
fn too_many_attachments_is_said_by_its_key_with_the_limit_in_french() {
    let e = CommandError::from(TooManyAttachmentsSnafu { max: 12usize }.build());
    let fr = Localizer::new(Some("fr"));
    let expected = fr.tr_with("agent-error-too-many-attachments", &[("max", 12u64.into())]);
    assert_that!(expected.as_str()).does_not_contain("agent-error-too-many-attachments");
    assert_that!(e.localize(&fr)).is_equal_to(expected);
    assert_that!(e.localize(&fr)).is_equal_to("Un message peut porter au plus 12 pièces jointes.".to_owned());
}

#[test]
fn attachment_too_large_is_said_by_its_key_in_french() {
    let e = CommandError::from(AttachmentTooLargeSnafu { len: 9usize, max: 8usize }.build());
    let fr = Localizer::new(Some("fr"));
    assert_that!(fr.tr("agent-error-attachment-too-large").as_str()).does_not_contain("agent-error");
    assert_that!(e.localize(&fr)).is_equal_to("Ce fichier est trop lourd pour être joint.".to_owned());
}

#[test]
fn attachment_gone_is_said_in_the_user_language_without_its_id() {
    let e = CommandError::from(AttachmentGoneSnafu { id: "sha256:aa" }.build());
    assert_that!(e.localize(&Localizer::new(Some("fr"))))
        .is_equal_to("Une pièce jointe n'est plus disponible : joignez-la de nouveau.".to_owned());
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
