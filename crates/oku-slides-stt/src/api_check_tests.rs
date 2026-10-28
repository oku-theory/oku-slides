use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case("https://api.openai.com/v1", "https://api.openai.com/v1")]
#[case("https://api.openai.com/v1/", "https://api.openai.com/v1")]
#[case("https://api.openai.com", "https://api.openai.com")]
#[case("HTTPS://API.groq.com/openai/v1", "https://api.groq.com/openai/v1")]
#[case("http://localhost:8000/v1", "http://localhost:8000/v1")]
#[case("http://127.0.0.1:9000", "http://127.0.0.1:9000")]
#[case("http://127.1:9000", "http://127.0.0.1:9000")]
#[case("http://0x7f.1/v1", "http://127.0.0.1/v1")]
#[case("http://2130706433/v1", "http://127.0.0.1/v1")]
#[case("http://[::1]:9000/v1", "http://[::1]:9000/v1")]
fn normalized_url_accepted_form_is_canonical(#[case] url: &str, #[case] expected: &str) {
    assert_that!(normalized_url(url)).is_equal_to(Some(expected.to_owned()));
}

#[rstest]
#[case("http://api.openai.com/v1")]
#[case("http://localhost.evil.example/v1")]
#[case("http://localhost./v1")]
#[case("http://127.0.0.1@evil.example/v1")]
#[case("http://[::ffff:127.0.0.1]/v1")]
#[case("http://[::1%25lo]/v1")]
#[case("https://api.openai.com@evil.example/v1")]
#[case("https://user:pw@api.openai.com/v1")]
#[case("https://api.openai.com/v1?x=1")]
#[case("https://api.openai.com/v1?")]
#[case("https://api.openai.com/v1#f")]
#[case("https://api.openai.com/\u{202e}1v")]
#[case("https://аpi.openai.com/v1")]
#[case(" https://api.openai.com/v1")]
#[case("https://api.openai.com/v 1")]
#[case("https:api.openai.com/v1")]
#[case("http://localhost\\@evil.example/v1")]
#[case("https://a..b/v1")]
#[case("https://.../v1")]
#[case("https://a{b}.example/v1")]
#[case("ftp://api.openai.com")]
#[case("api.openai.com/v1")]
#[case("")]
fn normalized_url_refused_form_is_none(#[case] url: &str) {
    assert_that!(normalized_url(url)).is_none();
}

#[test]
fn normalized_url_over_512_characters_is_none() {
    assert_that!(normalized_url(&format!("https://a.example/{}", "x".repeat(512)))).is_none();
}

#[test]
fn normalized_url_grown_past_512_by_encoding_is_none() {
    // `"` devient `%22` : la forme normalisée dépasse la borne.
    assert_that!(normalized_url(&format!("https://a.example/{}", "\"".repeat(200)))).is_none();
}

#[test]
fn normalized_url_with_a_64_character_label_is_none() {
    assert_that!(normalized_url(&format!("https://{}.example/v1", "a".repeat(64)))).is_none();
}

#[rstest]
#[case("https://api.openai.com/v1/")]
#[case("HTTP://127.1:9000/")]
#[case("https://api.groq.com/openai/v1")]
fn normalized_url_is_idempotent(#[case] url: &str) {
    let once = normalized_url(url).unwrap();
    assert_that!(normalized_url(&once)).is_equal_to(Some(once));
}

#[rstest]
#[case("https://api.groq.com/openai/v1", Some("api.groq.com"))]
#[case("http://[::1]:9000/v1", Some("[::1]"))]
#[case("http://evil.example", None)]
fn url_host_names_the_host_of_a_valid_url(#[case] url: &str, #[case] host: Option<&str>) {
    assert_that!(url_host(url)).is_equal_to(host.map(str::to_owned));
}

#[rstest]
#[case("http://127.1:9000", true)]
#[case("http://localhost:8000/v1", true)]
#[case("https://api.openai.com/v1", false)]
fn is_loopback_follows_the_normalized_host(#[case] url: &str, #[case] loopback: bool) {
    assert_that!(is_loopback(url)).is_equal_to(loopback);
}

#[rstest]
#[case("OPENAI_API_KEY", true)]
#[case("_KEY2", true)]
#[case("openai_api_key", false)]
#[case("sk-proj-abc123", false)]
#[case("A B", false)]
#[case("", false)]
fn valid_key_env_follows_the_name_rule(#[case] name: &str, #[case] ok: bool) {
    assert_that!(valid_key_env(name)).is_equal_to(ok);
}

#[rstest]
#[case(64, true)]
#[case(65, false)]
fn valid_key_env_name_length_is_capped_at_64(#[case] len: usize, #[case] ok: bool) {
    assert_that!(valid_key_env(&"A".repeat(len))).is_equal_to(ok);
}

#[rstest]
#[case("gpt-4o-transcribe", true)]
#[case("whisper-1", true)]
#[case("openai/whisper-large-v3", true)]
#[case("model\r\nX: y", false)]
#[case("m\"x", false)]
#[case("", false)]
fn valid_api_model_follows_the_name_rule(#[case] name: &str, #[case] ok: bool) {
    assert_that!(valid_api_model(name)).is_equal_to(ok);
}

#[rstest]
#[case("", true)]
#[case("2025-03-01-preview", true)]
#[case("2024-10-21", true)]
#[case("2025/03", false)]
#[case("v1&x=y", false)]
#[case("01234567890123456789012345678901", true)]
#[case("012345678901234567890123456789012", false)]
fn valid_api_version_follows_the_version_rule(#[case] version: &str, #[case] ok: bool) {
    assert_that!(valid_api_version(version)).is_equal_to(ok);
}
