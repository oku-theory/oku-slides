use rstest::rstest;
use speculoos::prelude::*;

use oku_slides_platform::config::Settings;
use oku_slides_platform::storage::Storage;

use super::*;

/// Un dépôt qui répond `status` à tout, et le téléchargement de `Base` qui s'y brise.
fn download_against(status: u16, keep_going: bool) -> SttError {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap_or_else(|e| panic!("dépôt de test : {e}"));
    let url = format!("http://{}", server.server_addr());
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
            let _ = req.respond(tiny_http::Response::from_data(b"x".to_vec()).with_status_code(status));
        }
    });
    let dir = crate::test_support::fresh_dir("error-download");
    let storage = Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}")).models;
    settings.hub_url = url;
    settings.download_retries = 0;
    match crate::models::download(crate::Model::Base, dir.path(), &settings, &storage, |_, _| keep_going) {
        Ok(_) => panic!("le téléchargement devait échouer"),
        Err(e) => e,
    }
}

fn cancelled_download() -> SttError {
    download_against(200, false)
}

fn failed_download() -> SttError {
    download_against(404, true)
}

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
        (cancelled_download(), SttErrorKind::Cancelled, "Téléchargement annulé"),
        (failed_download(), SttErrorKind::Download, "Le modèle n'a pas pu être téléchargé"),
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
    assert_that!(failed_download().to_string()).is_equal_to("model download failed".to_owned());
}

#[test]
fn api_key_missing_kind_is_model() {
    let e: SttError = ApiKeyMissingSnafu { var: "OPENAI_API_KEY" }.build().into();
    assert_that!(e.kind()).is_equal_to(SttErrorKind::Model);
}

#[rstest]
#[case("en", "OPENAI_API_KEY")]
#[case("fr", "OPENAI_API_KEY")]
fn api_key_missing_localized_names_the_variable(#[case] locale: &str, #[case] var: &str) {
    let l = Localizer::new(Some(locale));
    let e: SttError = ApiKeyMissingSnafu { var }.build().into();
    assert_that!(e.localize(&l)).contains(var);
}

#[test]
fn api_status_localized_in_french_shows_the_status() {
    let e: SttError = ApiStatusSnafu { status: 401u16 }.build().into();
    let text = e.localize(&Localizer::new(Some("fr")));
    assert_that!(text).contains("401");
    assert_that!(text).is_not_equal_to(e.localize(&Localizer::new(Some("en"))));
}

#[test]
fn api_config_kind_is_model_and_french_text_is_the_literal() {
    let e: SttError = ApiConfigSnafu.build().into();
    assert_that!(e.kind()).is_equal_to(SttErrorKind::Model);
    assert_that!(fr(&e)).is_equal_to(
        "Les réglages de l'API de dictée sont invalides : corrigez-les dans la boîte des réglages.".to_owned(),
    );
}
