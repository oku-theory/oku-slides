use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::ApiConfig;

fn param(name: &str, value: &str) -> ApiParam {
    ApiParam { name: name.into(), value: value.into() }
}

fn literal(name: &str, value: &str) -> ApiHeader {
    ApiHeader { name: name.into(), value: HeaderValue::Literal(value.into()) }
}

fn env(name: &str, var: &str) -> ApiHeader {
    ApiHeader { name: name.into(), value: HeaderValue::Env(var.into()) }
}

fn azure() -> ApiConfig {
    ApiConfig {
        key_env: String::new(),
        query: vec![param("api-version", "2025-03-01-preview")],
        headers: vec![env("api-key", "AZURE_OPENAI_API_KEY")],
        ..ApiConfig::default()
    }
}

#[test]
fn parse_api_query_two_pairs_returns_them_in_order() {
    let parsed = parse_api_query(" api-version=2025-03-01-preview & a=b=c ");
    assert_that!(parsed).is_equal_to(Some(vec![param("api-version", "2025-03-01-preview"), param("a", "b=c")]));
}

#[rstest]
#[case("")]
#[case("   ")]
fn parse_api_query_blank_returns_no_param(#[case] text: &str) {
    assert_that!(parse_api_query(text)).is_equal_to(Some(Vec::new()));
}

#[rstest]
#[case("=x")]
#[case("novalue")]
#[case("a=b c")]
#[case("a=%20")]
#[case("a=b#c")]
#[case("a=é")]
#[case("a=b&&c=d")]
fn parse_api_query_malformed_returns_none(#[case] text: &str) {
    assert_that!(parse_api_query(text)).is_none();
}

#[test]
fn parse_api_query_more_than_eight_pairs_returns_none() {
    let text = (0..9).map(|i| format!("p{i}=v")).collect::<Vec<_>>().join("&");
    assert_that!(parse_api_query(&text)).is_none();
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
fn format_api_query_round_trips_through_parse() {
    let query = vec![param("api-version", "1"), param("b", "")];
    assert_that!(parse_api_query(&format_api_query(&query))).is_equal_to(Some(query));
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
fn valid_api_headers_authorization_alone_is_valid() {
    assert_that!(valid_api_headers(&[literal("Authorization", "Basic x")])).is_true();
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
fn valid_api_access_authorization_with_key_env_is_invalid() {
    let config = ApiConfig { headers: vec![literal("Authorization", "Basic x")], ..ApiConfig::default() };
    assert_that!(valid_api_access(&config)).is_false();
}

#[test]
fn valid_api_access_authorization_without_key_env_is_valid() {
    let config =
        ApiConfig { key_env: String::new(), headers: vec![env("Authorization", "TOKEN")], ..ApiConfig::default() };
    assert_that!(valid_api_access(&config)).is_true();
}

#[test]
fn valid_api_access_duplicate_header_is_invalid() {
    let config = ApiConfig { headers: vec![literal("X-A", "1"), literal("x-a", "2")], ..ApiConfig::default() };
    assert_that!(valid_api_access(&config)).is_false();
}

#[test]
fn valid_api_access_no_variable_at_all_is_invalid() {
    let config = ApiConfig { key_env: String::new(), headers: vec![literal("X-A", "1")], ..ApiConfig::default() };
    assert_that!(valid_api_access(&config)).is_false();
}

#[test]
fn valid_api_access_bad_stored_variable_is_invalid() {
    let config =
        ApiConfig { key_env: String::new(), headers: vec![env("api-key", "bad name")], ..ApiConfig::default() };
    assert_that!(valid_api_access(&config)).is_false();
}

#[test]
fn variables_lists_the_key_then_the_header_variables() {
    let config = ApiConfig { headers: vec![env("api-key", "AZ"), literal("X", "y")], ..ApiConfig::default() };
    assert_that!(config.variables()).is_equal_to(vec!["OPENAI_API_KEY", "AZ"]);
}

#[test]
fn config_without_query_and_headers_reads_from_older_json() {
    let config: ApiConfig =
        serde_json::from_str(r#"{"url":"https://api.openai.com/v1","model":"m","key_env":"K"}"#).unwrap();
    assert_that!(config.headers.is_empty() && config.query.is_empty()).is_true();
}

#[test]
fn config_headers_serialize_variable_by_name_only() {
    let json = serde_json::to_value(azure()).unwrap();
    assert_that!(json["headers"][0]["value"]).is_equal_to(serde_json::json!({"env": "AZURE_OPENAI_API_KEY"}));
}

#[test]
fn api_error_detail_reads_error_message_and_redacts_secrets() {
    let body = r#"{"error":{"code":"x","message":"bad key sk-SECRET1234 and\nmissing api-version"}}"#;
    let detail = api_error_detail(body, &["sk-SECRET1234"]);
    assert_that!(detail).is_equal_to(Some("bad key *** and missing api-version".to_owned()));
}

#[test]
fn api_error_detail_string_error_is_read() {
    assert_that!(api_error_detail(r#"{"error":"Missing API version"}"#, &[]))
        .is_equal_to(Some("Missing API version".to_owned()));
}

#[test]
fn api_error_detail_long_message_is_cut_at_200_chars() {
    let body = serde_json::json!({"error": {"message": "é".repeat(300)}}).to_string();
    assert_that!(api_error_detail(&body, &[]).map(|d| d.chars().count())).is_equal_to(Some(201));
}

#[test]
fn api_error_detail_echo_of_part_of_a_secret_is_dropped() {
    let body = r#"{"error":{"message":"api-key 'abcdefghij...' is invalid"}}"#;
    assert_that!(api_error_detail(body, &["abcdefghijklmnopqrstuvwxyz"])).is_none();
}

#[test]
fn api_error_detail_secret_split_by_a_control_char_is_masked() {
    let body = "{\"error\":{\"message\":\"key sk-SECRET\\u00071234 refused\"}}";
    assert_that!(api_error_detail(body, &["sk-SECRET1234"])).is_equal_to(Some("key *** refused".to_owned()));
}

#[test]
fn api_error_detail_short_secret_drops_the_message() {
    assert_that!(api_error_detail(r#"{"error":"bad value abc"}"#, &["abc"])).is_none();
}

#[test]
fn api_error_detail_masks_the_token_after_a_scheme() {
    let detail = api_error_detail(r#"{"error":"token xyz123456789 expired"}"#, &["Bearer xyz123456789"]);
    assert_that!(detail).is_equal_to(Some("token *** expired".to_owned()));
}

#[rstest]
#[case("not json")]
#[case(r#"{"detail":"x"}"#)]
#[case(r#"{"error":{"message":"   "}}"#)]
fn api_error_detail_without_message_is_none(#[case] body: &str) {
    assert_that!(api_error_detail(body, &[])).is_none();
}
