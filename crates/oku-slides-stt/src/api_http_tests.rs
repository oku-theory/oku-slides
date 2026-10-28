use std::time::{Duration, Instant};

use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

use super::MAX_REPLY;
use crate::api::{ApiConfig, ApiTranscriber};
use crate::phrase_engine::PhraseEngine;
use crate::test_support::fake_api_with;
use crate::{SttError, SttErrorKind};

/// Transcribes silence against a fake API answering `status`, `body` and `headers`.
fn reply(status: u16, body: String, headers: &'static [(&'static str, &'static str)]) -> SttError {
    let (url, _calls) = fake_api_with(status, body, headers);
    let config = ApiConfig { url, model: "whisper-1".into(), key_env: "HOME".into(), ..ApiConfig::default() };
    let t = ApiTranscriber::new(config, Duration::from_secs(5)).unwrap();
    t.transcribe(&[0.0; 1600], None, None, false).unwrap_err()
}

fn en(e: &SttError) -> String {
    e.localize(&Localizer::new(Some("en")))
}

#[test]
fn send_too_many_requests_with_retry_after_fails_at_once() {
    let started = Instant::now();
    let e = reply(429, r#"{"error":{"message":"slow down"}}"#.into(), &[("Retry-After", "3600")]);
    assert_that!((started.elapsed() < Duration::from_secs(3), en(&e).contains("429"))).is_equal_to((true, true));
}

#[test]
fn send_server_error_sends_one_request() {
    let (url, calls) = fake_api_with(503, "{}".into(), &[("Retry-After", "0")]);
    let config = ApiConfig { url, model: "whisper-1".into(), key_env: "HOME".into(), ..ApiConfig::default() };
    let t = ApiTranscriber::new(config, Duration::from_secs(5)).unwrap();
    t.transcribe(&[0.0; 1600], None, None, false).unwrap_err();
    calls.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_that!(calls.recv_timeout(Duration::from_millis(500)).is_err()).is_true();
}

#[test]
fn send_server_error_page_keeps_the_status_not_the_page() {
    let e = reply(502, "<html><body>upstream internal 10.0.0.7</body></html>".into(), &[]);
    let text = en(&e);
    assert_that!((text.contains("502"), text.contains("10.0.0.7"))).is_equal_to((true, false));
}

#[test]
fn send_not_found_page_keeps_the_status() {
    let e = reply(404, "<html>Not Found</html>".into(), &[]);
    assert_that!((e.kind(), en(&e).contains("404"))).is_equal_to((SttErrorKind::Transcription, true));
}

#[test]
fn send_string_error_body_keeps_the_status() {
    let e = reply(401, r#"{"error":"bad key"}"#.into(), &[]);
    let text = en(&e);
    assert_that!((text.contains("401"), text.contains("bad key"))).is_equal_to((true, false));
}

#[test]
fn send_oversized_reply_returns_transcription_error() {
    let body = format!(r#"{{"text":"{}"}}"#, "a".repeat(MAX_REPLY));
    let e = reply(200, body, &[]);
    assert_that!(e.kind()).is_equal_to(SttErrorKind::Transcription);
}

#[test]
fn send_reply_without_text_returns_transcription_error() {
    let e = reply(200, r#"{"result":"x"}"#.into(), &[]);
    assert_that!(e.kind()).is_equal_to(SttErrorKind::Transcription);
}
