use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case(VoiceErrorKind::Busy, "La dictée est en cours : réessayez une fois au repos")]
#[case(VoiceErrorKind::NothingToCancel, "Aucun téléchargement n'est en cours")]
fn voice_error_localizes_in_french(#[case] kind: VoiceErrorKind, #[case] text: &str) {
    assert_that!(VoiceError::new(kind).localize(&Localizer::new(Some("fr")))).is_equal_to(text.to_owned());
}

#[rstest]
fn voice_error_every_kind_has_a_translation(
    #[values(
        VoiceErrorKind::NoDictation,
        VoiceErrorKind::Busy,
        VoiceErrorKind::UnknownModel,
        VoiceErrorKind::Unavailable,
        VoiceErrorKind::Installed,
        VoiceErrorKind::NotDownloadable,
        VoiceErrorKind::NothingToCancel,
        VoiceErrorKind::BadSilence,
        VoiceErrorKind::BadLanguage,
        VoiceErrorKind::BadUrl,
        VoiceErrorKind::BadKeyEnv,
        VoiceErrorKind::BadApiModel,
        VoiceErrorKind::BadApiVersion,
        VoiceErrorKind::BadApiHeaders,
        VoiceErrorKind::ApiReservedToUser,
        VoiceErrorKind::Changed,
        VoiceErrorKind::NothingToDownload,
        VoiceErrorKind::UnknownAgent,
        VoiceErrorKind::AgentLocked,
        VoiceErrorKind::AgentNotSaved
    )]
    kind: VoiceErrorKind,
) {
    // `Localizer::tr` rend l'identifiant d'une clé absente.
    let text = VoiceError::new(kind).localize(&Localizer::new(Some("fr")));
    assert_that!(text).does_not_contain("voice-error-");
    assert_that!(text).does_not_contain("agent-error-");
}
