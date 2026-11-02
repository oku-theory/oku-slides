use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::parse;
use crate::engine::Reply;

#[test]
fn parse_call_gives_name_and_arguments() {
    let reply = parse(r#"{"arguments": {"value": 30, "key": "history_limit"}, "tool": "set_setting"}"#);
    assert_that!(reply)
        .is_equal_to(Reply::Call { name: "set_setting".into(), args: json!({"value": 30, "key": "history_limit"}) });
}

#[rstest]
#[case::missing(r#"{"tool": "deck_outline"}"#)]
#[case::null(r#"{"tool": "deck_outline", "arguments": null}"#)]
fn parse_call_without_arguments_has_empty_object(#[case] json: &str) {
    assert_that!(parse(json)).is_equal_to(Reply::Call { name: "deck_outline".into(), args: json!({}) });
}

#[test]
fn parse_text_is_trimmed_text() {
    assert_that!(parse(r#"{"text": "  Done. "}"#)).is_equal_to(Reply::Text("Done.".into()));
}

#[rstest]
#[case::not_json("{\"tool\": ", "not valid JSON")]
#[case::not_object("[1]", "JSON object")]
#[case::tool_not_string(r#"{"tool": 3}"#, "\"tool\"")]
#[case::arguments_not_object(r#"{"tool": "a", "arguments": [1]}"#, "\"arguments\"")]
#[case::neither(r#"{"other": 1}"#, "\"tool\" or a \"text\"")]
#[case::text_not_string(r#"{"text": 1}"#, "\"tool\" or a \"text\"")]
fn parse_bad_reply_is_invalid_with_reason(#[case] json: &str, #[case] reason: &str) {
    let Reply::Invalid(why) = parse(json) else { panic!("expected Invalid for {json}") };
    assert_that!(why.as_str()).contains(reason);
}
