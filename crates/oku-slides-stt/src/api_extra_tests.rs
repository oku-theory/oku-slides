use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::ApiConfig;

fn literal(name: &str, value: &str) -> ApiHeader {
    ApiHeader { name: name.into(), value: HeaderValue::Literal(value.into()) }
}

fn env(name: &str, var: &str) -> ApiHeader {
    ApiHeader { name: name.into(), value: HeaderValue::Env(var.into()) }
}

fn azure() -> ApiConfig {
    ApiConfig {
        url: "https://acme.openai.azure.com".into(),
        model: "transcribe-prod".into(),
        key_env: "AZURE_OPENAI_API_KEY".into(),
        api_version: "2025-03-01-preview".into(),
        headers: vec![env("X-Gateway-Token", "GATEWAY_TOKEN")],
        ..ApiConfig::default()
    }
}

#[test]
fn parse_api_headers_literal_and_variable_returns_both() {
    let parsed = parse_api_headers("api-key: $AZURE_OPENAI_API_KEY; X-Tenant: acme corp;");
    assert_that!(parsed)
        .is_equal_to(Some(vec![env("api-key", "AZURE_OPENAI_API_KEY"), literal("X-Tenant", "acme corp")]));
}

#[rstest]
#[case("X-A")]
#[case(": v")]
#[case("X A: v")]
#[case("X-A:")]
#[case("X-A: $lower")]
#[case("X-A: é")]
#[case("X-A: a\u{7f}")]
fn parse_api_headers_malformed_returns_none(#[case] text: &str) {
    assert_that!(parse_api_headers(text)).is_none();
}

#[test]
fn parse_api_headers_value_over_256_bytes_returns_none() {
    assert_that!(parse_api_headers(&format!("X-A: {}", "v".repeat(257)))).is_none();
}

#[test]
fn format_api_headers_writes_variables_with_a_dollar() {
    let headers = vec![env("api-key", "K"), literal("X-Tenant", "acme")];
    assert_that!(format_api_headers(&headers).as_str()).is_equal_to("api-key: $K; X-Tenant: acme");
}

#[test]
fn format_api_headers_round_trips_through_parse() {
    let headers = vec![env("api-key", "K"), literal("X-Tenant", "a: b")];
    assert_that!(parse_api_headers(&format_api_headers(&headers))).is_equal_to(Some(headers));
}

#[test]
fn valid_api_access_azure_settings_are_valid() {
    assert_that!(valid_api_access(&azure())).is_true();
}

#[test]
fn valid_api_access_default_settings_are_valid() {
    assert_that!(valid_api_access(&ApiConfig::default())).is_true();
}

#[rstest]
#[case("Authorization")]
#[case("api-key")]
#[case("Host")]
#[case("content-type")]
#[case("Content-Length")]
#[case("Transfer-Encoding")]
#[case("CONNECTION")]
#[case("Proxy-Authorization")]
#[case("TE")]
#[case("Trailer")]
#[case("Upgrade")]
#[case("Expect")]
#[case("Keep-Alive")]
fn valid_api_access_reserved_header_is_invalid(#[case] name: &str) {
    let config = ApiConfig { headers: vec![literal(name, "x")], ..ApiConfig::default() };
    assert_that!(valid_api_access(&config)).is_false();
}

#[test]
fn valid_api_headers_reserved_name_is_invalid() {
    assert_that!(valid_api_headers(&[literal("Host", "evil.example")])).is_false();
}

#[test]
fn valid_api_headers_duplicate_name_is_invalid() {
    assert_that!(valid_api_headers(&[literal("X-A", "1"), literal("x-a", "2")])).is_false();
}

#[test]
fn valid_api_access_duplicate_header_is_invalid() {
    let config = ApiConfig { headers: vec![literal("X-A", "1"), literal("x-a", "2")], ..ApiConfig::default() };
    assert_that!(valid_api_access(&config)).is_false();
}

#[test]
fn valid_api_access_empty_key_variable_is_invalid() {
    let config = ApiConfig { key_env: String::new(), headers: vec![env("X-A", "TOKEN")], ..ApiConfig::default() };
    assert_that!(valid_api_access(&config)).is_false();
}

#[test]
fn valid_api_access_bad_stored_variable_is_invalid() {
    let config = ApiConfig { headers: vec![env("X-A", "bad name")], ..ApiConfig::default() };
    assert_that!(valid_api_access(&config)).is_false();
}

#[test]
fn valid_api_access_bad_api_version_is_invalid() {
    let config = ApiConfig { api_version: "2025/03".into(), ..azure() };
    assert_that!(valid_api_access(&config)).is_false();
}

#[rstest]
#[case("team/whisper")]
#[case("a:b")]
#[case(".")]
#[case("..")]
fn valid_api_access_azure_deployment_outside_a_path_segment_is_invalid(#[case] model: &str) {
    let config = ApiConfig { model: model.into(), ..azure() };
    assert_that!(valid_api_access(&config)).is_false();
}

#[test]
fn valid_api_access_openai_model_with_a_slash_is_valid() {
    let config = ApiConfig { model: "openai/whisper-large".into(), ..ApiConfig::default() };
    assert_that!(valid_api_access(&config)).is_true();
}

#[test]
fn variables_lists_the_key_then_the_header_variables() {
    let config = ApiConfig { headers: vec![env("X-Token", "AZ"), literal("X", "y")], ..ApiConfig::default() };
    assert_that!(config.variables()).is_equal_to(vec!["OPENAI_API_KEY", "AZ"]);
}

#[test]
fn config_without_version_and_headers_reads_from_older_json() {
    let config: ApiConfig =
        serde_json::from_str(r#"{"url":"https://api.openai.com/v1","model":"m","key_env":"K"}"#).unwrap();
    assert_that!(config.headers.is_empty() && config.api_version.is_empty()).is_true();
}

#[test]
fn config_headers_serialize_variable_by_name_only() {
    let json = serde_json::to_value(azure()).unwrap();
    assert_that!(json["headers"][0]["value"]).is_equal_to(serde_json::json!({"env": "GATEWAY_TOKEN"}));
}

#[test]
fn api_error_detail_reads_error_message_and_redacts_secrets() {
    let detail = api_error_detail("bad key sk-SECRET1234 and\nmissing api-version", &["sk-SECRET1234"]);
    assert_that!(detail).is_equal_to(Some("bad key *** and missing api-version".to_owned()));
}

#[test]
fn api_error_detail_long_message_is_cut_at_200_chars() {
    assert_that!(api_error_detail(&"é".repeat(300), &[]).map(|d| d.chars().count())).is_equal_to(Some(201));
}

#[test]
fn api_error_detail_echo_of_part_of_a_secret_is_dropped() {
    assert_that!(api_error_detail("api-key 'abcdefghij...' is invalid", &["abcdefghijklmnopqrstuvwxyz"])).is_none();
}

#[test]
fn api_error_detail_secret_split_by_a_control_char_is_masked() {
    assert_that!(api_error_detail("key sk-SECRET\u{7}1234 refused", &["sk-SECRET1234"]))
        .is_equal_to(Some("key *** refused".to_owned()));
}

#[test]
fn api_error_detail_short_secret_drops_the_message() {
    assert_that!(api_error_detail("bad value abc", &["abc"])).is_none();
}

#[test]
fn api_error_detail_masks_the_token_after_a_scheme() {
    let detail = api_error_detail("token xyz123456789 expired", &["Bearer xyz123456789"]);
    assert_that!(detail).is_equal_to(Some("token *** expired".to_owned()));
}

#[rstest]
#[case("")]
#[case("   ")]
fn api_error_detail_blank_message_is_none(#[case] message: &str) {
    assert_that!(api_error_detail(message, &[])).is_none();
}

fn legacy() -> Vec<LegacyQueryParam> {
    vec![LegacyQueryParam { name: "api-version".into(), value: "2024-06-01".into() }]
}

#[rstest]
#[case(ApiConfig { key_env: String::new(), ..azure() }, Some(ApiAccessProblem::KeyEnv))]
#[case(ApiConfig { api_version: "2025/03".into(), ..azure() }, Some(ApiAccessProblem::Version))]
#[case(ApiConfig { legacy_query: legacy(), ..ApiConfig::default() }, Some(ApiAccessProblem::Version))]
#[case(ApiConfig { headers: vec![literal("Host", "x")], ..azure() }, Some(ApiAccessProblem::Headers))]
#[case(ApiConfig { model: "..".into(), ..azure() }, Some(ApiAccessProblem::Deployment))]
#[case(azure(), None)]
fn api_access_problem_names_the_first_bad_field(#[case] config: ApiConfig, #[case] problem: Option<ApiAccessProblem>) {
    assert_that!(api_access_problem(&config)).is_equal_to(problem);
}

#[test]
fn api_config_legacy_query_from_7_0_0_is_read_and_written_back() {
    let json = r#"{"url":"https://acme.openai.azure.com/openai/deployments/d","query":[{"name":"api-version","value":"2024-06-01"}]}"#;
    let config: ApiConfig = serde_json::from_str(json).unwrap();
    let back = serde_json::to_value(&config).unwrap();
    assert_that!((config.legacy_query == legacy(), back.get("query").is_some())).is_equal_to((true, true));
}

#[test]
fn api_config_without_legacy_query_writes_no_query() {
    let back = serde_json::to_value(ApiConfig::default()).unwrap();
    assert_that!(back.get("query").is_none()).is_true();
}
