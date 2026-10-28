use std::time::Duration;

use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

use super::*;
use crate::SttErrorKind;
use crate::test_support::{ApiCall, fake_api};

#[test]
fn wav_one_second_parses_back_at_16k_mono() {
    let (rate, samples) = crate::audio::parse_wav(&wav(&vec![0.25; 16_000]).unwrap()).unwrap();
    assert_that!(rate).is_equal_to(SAMPLE_RATE);
    assert_that!(samples).has_length(16_000);
}

#[test]
fn key_unset_variable_returns_api_key_missing() {
    let config = ApiConfig { key_env: "OKU_SLIDES_TEST_NO_SUCH_KEY_7f3a".into(), ..ApiConfig::default() };
    assert_that!(ApiConfig::secret(&config.key_env).unwrap_err().kind()).is_equal_to(SttErrorKind::Model);
}

#[test]
fn key_present_unset_variable_is_absent() {
    let config = ApiConfig { key_env: "OKU_SLIDES_TEST_NO_SUCH_KEY_7f3a".into(), ..ApiConfig::default() };
    assert_that!(config.key_present()).is_false();
}

#[test]
fn key_present_set_variable_is_present() {
    let config = ApiConfig { key_env: "HOME".into(), ..ApiConfig::default() };
    assert_that!(config.key_present()).is_true();
}

fn transcriber(url: String) -> ApiTranscriber {
    let config = ApiConfig { url, model: "whisper-1".into(), key_env: "HOME".into(), ..ApiConfig::default() };
    ApiTranscriber::new(config, Duration::from_secs(5)).unwrap()
}

#[test]
fn transcribe_ok_returns_the_api_text() {
    let (url, _calls) = fake_api(200, r#"{"text":"Bonjour à tous."}"#);
    let t = transcriber(url).transcribe(&[0.0; 1600], Some("fr"), Some("Oku"), false).unwrap();
    assert_that!(t.text).is_equal_to("Bonjour à tous.".to_owned());
}

#[test]
fn transcribe_sends_bearer_key_model_language_and_prompt() {
    let (url, calls) = fake_api(200, r#"{"text":"x"}"#);
    transcriber(url).transcribe(&[0.0; 1600], Some("fr"), Some("Oku"), false).unwrap();
    let call = calls.recv_timeout(Duration::from_secs(5)).unwrap();
    let body = String::from_utf8_lossy(&call.body).into_owned();
    assert_that!(call.authorization).is_equal_to(format!("Bearer {}", std::env::var("HOME").unwrap()));
    assert_that!(body.as_str()).contains("whisper-1");
    assert_that!(body.as_str()).contains("name=\"language\"\r\n\r\nfr\r\n");
    assert_that!(body.as_str()).contains("name=\"prompt\"\r\n\r\nOku\r\n");
}

/// Azure settings: deployment in the path, `api-version` in the query, the key in `api-key`.
fn azure_transcriber(url: String) -> ApiTranscriber {
    let config = ApiConfig {
        url,
        model: "transcribe-prod".into(),
        // PATH: always set and long enough to be masked (HOME is `/root` on some CI).
        key_env: "PATH".into(),
        api_version: "2025-03-01-preview".into(),
        headers: vec![
            ApiHeader { name: "X-Token".into(), value: HeaderValue::Env("HOME".into()) },
            ApiHeader { name: "X-Tenant".into(), value: HeaderValue::Literal("acme".into()) },
        ],
        ..ApiConfig::default()
    };
    ApiTranscriber::new(config, Duration::from_secs(5)).unwrap()
}

fn header<'a>(call: &'a ApiCall, name: &str) -> Option<&'a str> {
    call.headers.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
}

fn azure_call() -> ApiCall {
    let (url, calls) = fake_api(200, r#"{"text":"x"}"#);
    azure_transcriber(url).transcribe(&[0.0; 1600], None, None, false).unwrap();
    calls.recv_timeout(Duration::from_secs(5)).unwrap()
}

#[test]
fn transcribe_azure_calls_the_deployment_with_its_api_version() {
    assert_that!(azure_call().url).is_equal_to(
        "/v1/openai/deployments/transcribe-prod/audio/transcriptions?api-version=2025-03-01-preview".to_owned(),
    );
}

#[test]
fn transcribe_azure_sends_the_key_as_api_key() {
    let call = azure_call();
    assert_that!(header(&call, "api-key")).is_equal_to(Some(std::env::var("PATH").unwrap().as_str()));
}

#[test]
fn transcribe_azure_sends_no_authorization() {
    assert_that!(header(&azure_call(), "Authorization")).is_none();
}

#[test]
fn transcribe_variable_header_sends_the_variable_value() {
    let call = azure_call();
    assert_that!(header(&call, "X-Token")).is_equal_to(Some(std::env::var("HOME").unwrap().as_str()));
}

#[test]
fn transcribe_literal_header_sends_the_literal() {
    assert_that!(header(&azure_call(), "X-Tenant")).is_equal_to(Some("acme"));
}

#[test]
fn transcribe_openai_sends_no_organization_or_project() {
    let (url, calls) = fake_api(200, r#"{"text":"x"}"#);
    transcriber(url).transcribe(&[0.0; 1600], None, None, false).unwrap();
    let call = calls.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_that!((header(&call, "OpenAI-Organization"), header(&call, "OpenAI-Project"))).is_equal_to((None, None));
}

#[test]
fn transcribe_bad_request_shows_the_api_message_without_secrets() {
    let path = std::env::var("PATH").unwrap();
    let body: &'static str =
        Box::leak(format!(r#"{{"error":{{"message":"Missing API version for key {path}"}}}}"#).into_boxed_str());
    let (url, _calls) = fake_api(400, body);
    let e = azure_transcriber(url).transcribe(&[0.0; 1600], None, None, false).unwrap_err();
    let text = e.localize(&Localizer::new(Some("fr")));
    assert_that!((text.contains("Missing API version for key ***"), text.contains(path.as_str())))
        .is_equal_to((true, false));
}

#[test]
fn transcribe_forbidden_shows_no_api_message() {
    let (url, _calls) = fake_api(403, r#"{"error":{"message":"leaked"}}"#);
    let e = azure_transcriber(url).transcribe(&[0.0; 1600], None, None, false).unwrap_err();
    assert_that!(e.localize(&Localizer::new(Some("en")))).does_not_contain("leaked");
}

#[test]
fn new_unset_header_variable_returns_api_key_missing() {
    let config = ApiConfig {
        key_env: "HOME".into(),
        headers: vec![ApiHeader { name: "X-Token".into(), value: HeaderValue::Env("OKU_T1213_UNSET".into()) }],
        ..ApiConfig::default()
    };
    let e = ApiTranscriber::new(config, Duration::from_secs(5)).unwrap_err();
    assert_that!(e.localize(&Localizer::new(Some("en")))).contains("OKU_T1213_UNSET");
}

#[rstest::rstest]
#[case("Host")]
#[case("Authorization")]
#[case("api-key")]
fn new_reserved_header_is_refused(#[case] name: &str) {
    let headers = vec![ApiHeader { name: name.into(), value: HeaderValue::Literal("evil.example".into()) }];
    refused(ApiConfig { headers, ..ApiConfig::default() });
}

#[test]
fn key_present_unset_header_variable_is_absent() {
    let config = ApiConfig {
        key_env: "HOME".into(),
        headers: vec![ApiHeader { name: "X-Token".into(), value: HeaderValue::Env("OKU_T1213_UNSET".into()) }],
        ..ApiConfig::default()
    };
    assert_that!(config.key_present()).is_false();
}

#[test]
fn transcribe_unauthorized_returns_transcription_error() {
    let (url, _calls) = fake_api(401, r#"{"error":{"message":"bad key"}}"#);
    let e = transcriber(url).transcribe(&[0.0; 1600], None, None, false).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SttErrorKind::Transcription);
    assert_that!(e.localize(&Localizer::new(Some("en")))).contains("401");
}

#[test]
fn debug_never_shows_the_key() {
    let (url, _calls) = fake_api(200, "{}");
    let shown = format!("{:?}", transcriber(url));
    assert_that!(shown.contains(&std::env::var("HOME").unwrap())).is_false();
}

#[test]
fn transcribe_non_json_body_returns_transcription_error() {
    let (url, _calls) = fake_api(200, "not json");
    let e = transcriber(url).transcribe(&[0.0; 1600], None, None, false).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SttErrorKind::Transcription);
}

#[test]
fn transcribe_closed_port_returns_localized_transcription_error() {
    let e = transcriber("http://127.0.0.1:9/v1".into()).transcribe(&[0.0; 1600], None, None, false).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SttErrorKind::Transcription);
    let en = e.localize(&Localizer::new(Some("en")));
    let fr = e.localize(&Localizer::new(Some("fr")));
    assert_that!(fr).is_not_equal_to(en);
}

fn refused(config: ApiConfig) {
    let e = ApiTranscriber::new(config, Duration::from_secs(5)).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SttErrorKind::Model);
    assert_that!(e.localize(&Localizer::new(Some("fr")))).is_equal_to(
        "Les réglages de l'API de dictée sont invalides : corrigez-les dans la boîte des réglages.".to_owned(),
    );
}

#[test]
fn api_transcriber_invalid_stored_url_is_refused() {
    refused(ApiConfig {
        url: "http://192.168.1.5/v1".into(),
        model: "whisper-1".into(),
        key_env: "HOME".into(),
        ..ApiConfig::default()
    });
}

#[test]
fn api_transcriber_empty_key_env_is_refused() {
    refused(ApiConfig { key_env: String::new(), ..ApiConfig::default() });
}

#[test]
fn api_transcriber_invalid_key_env_is_refused() {
    refused(ApiConfig {
        url: "http://127.0.0.1:9/v1".into(),
        model: "whisper-1".into(),
        key_env: "sk-x".into(),
        ..ApiConfig::default()
    });
}

#[test]
fn api_transcriber_redirect_is_not_followed() {
    let second = tiny_http::Server::http("127.0.0.1:0").unwrap_or_else(|e| panic!("seconde API : {e}"));
    let second_addr = second.server_addr().to_ip().unwrap_or_else(|| panic!("seconde API : adresse non IP"));
    let hit = std::thread::spawn(move || second.recv_timeout(Duration::from_millis(500)));
    let first = tiny_http::Server::http("127.0.0.1:0").unwrap_or_else(|e| panic!("première API : {e}"));
    let url =
        format!("http://{}/v1", first.server_addr().to_ip().unwrap_or_else(|| panic!("première API : adresse non IP")));
    std::thread::spawn(move || {
        for request in first.incoming_requests() {
            let location = tiny_http::Header::from_bytes("Location", format!("http://{second_addr}/x"))
                .unwrap_or_else(|()| panic!("en-tête Location"));
            request.respond(tiny_http::Response::empty(307).with_header(location)).ok();
        }
    });
    let e = transcriber(url).transcribe(&[0.0; 1600], None, None, false).unwrap_err();
    assert_that!(e.localize(&Localizer::new(Some("en")))).contains("307");
    let reached = hit.join().unwrap().unwrap_or_else(|e| panic!("seconde API : {e}"));
    assert_that!(reached.is_none()).is_true();
}

#[test]
fn transcribe_bad_request_masks_a_long_literal_header() {
    let config = ApiConfig {
        key_env: "PATH".into(),
        headers: vec![ApiHeader { name: "X-Gateway".into(), value: HeaderValue::Literal("gw-0123456789".into()) }],
        ..ApiConfig::default()
    };
    let (url, _calls) = fake_api(400, r#"{"error":{"message":"bad gateway token gw-0123456789"}}"#);
    let t =
        ApiTranscriber::new(ApiConfig { url, model: "whisper-1".into(), ..config }, Duration::from_secs(5)).unwrap();
    let e = t.transcribe(&[0.0; 1600], None, None, false).unwrap_err();
    assert_that!(e.localize(&Localizer::new(Some("en")))).does_not_contain("gw-0123456789");
}

#[test]
fn api_transcriber_legacy_query_is_refused() {
    refused(ApiConfig {
        url: "http://127.0.0.1:9/v1".into(),
        key_env: "HOME".into(),
        legacy_query: vec![LegacyQueryParam { name: "api-version".into(), value: "2024-06-01".into() }],
        ..ApiConfig::default()
    });
}
