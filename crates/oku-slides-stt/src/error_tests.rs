use speculoos::prelude::*;

use super::*;

fn fr(e: &SttError) -> String {
    e.localize(&Localizer::new(Some("fr")))
}

#[test]
fn localize_each_kind_speaks_to_the_user() {
    let cases: Vec<(SttError, SttErrorKind, &str)> = vec![
        (NoMicrophoneSnafu.build().into(), SttErrorKind::Microphone, "Aucun micro trouvé"),
        (MicrophoneTimeoutSnafu.build().into(), SttErrorKind::Microphone, "Le micro ne répond pas"),
        (
            UnsupportedFormatSnafu { format: "I8" }.build().into(),
            SttErrorKind::Microphone,
            "Format audio non pris en charge : I8",
        ),
        (WavSnafu { reason: "x" }.build().into(), SttErrorKind::AudioFile, "Fichier audio illisible"),
        (NoLanguageSnafu.build().into(), SttErrorKind::Transcription, "La transcription a échoué"),
        (NotContiguousSnafu.build().into(), SttErrorKind::Transcription, "La transcription a échoué"),
        (
            IncompleteSnafu { got: 3u64, expected: 5u64 }.build().into(),
            SttErrorKind::Download,
            "Téléchargement incomplet : 3 octets sur 5",
        ),
        (CancelledSnafu.build().into(), SttErrorKind::Cancelled, "Téléchargement annulé"),
        (
            SpeechDeniedSnafu.build().into(),
            SttErrorKind::Permission,
            "Autorisez la reconnaissance vocale dans Réglages Système › Confidentialité et sécurité",
        ),
        (
            LocaleUnsupportedSnafu { locale: "xx" }.build().into(),
            SttErrorKind::Model,
            "Le moteur d'Apple ne prend pas en charge cette langue (xx)",
        ),
        (
            AssetInstallSnafu { reason: "x" }.build().into(),
            SttErrorKind::Download,
            "Le modèle n'a pas pu être téléchargé",
        ),
        (AnalyzerSnafu { reason: "x" }.build().into(), SttErrorKind::Transcription, "La transcription a échoué"),
        (FinishTimeoutSnafu.build().into(), SttErrorKind::Transcription, "La transcription a échoué"),
        (NotDownloadableSnafu.build().into(), SttErrorKind::Model, "Modèle de dictée illisible"),
    ];
    for (err, kind, message) in cases {
        asserting(message).that(&err.kind()).is_equal_to(kind);
        asserting(message).that(&fr(&err)).is_equal_to(message.to_owned());
    }
}

#[test]
fn display_stays_english_for_logs() {
    let err: SttError = IncompleteSnafu { got: 3u64, expected: 5u64 }.build().into();
    assert_that!(err.to_string()).is_equal_to("incomplete file: 3 bytes out of 5".to_owned());
}
