use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::parse;
use crate::engine::Reply;

#[rstest]
#[case::text("Done.", Reply::Text("Done.".into()))]
#[case::call("<tool_call>\n{\"name\": \"save\", \"arguments\": {\"x\": 1}}\n</tool_call>", Reply::Call { name: "save".into(), args: json!({"x": 1}) })]
#[case::call_without_args("<tool_call>{\"name\": \"save\"}</tool_call>", Reply::Call { name: "save".into(), args: json!({}) })]
#[case::unclosed("<tool_call>{\"name\": \"save\", \"arguments\": {}}", Reply::Call { name: "save".into(), args: json!({}) })]
#[case::text_before_call("Saving.\n<tool_call>{\"name\": \"save\", \"arguments\": {}}</tool_call>", Reply::Call { name: "save".into(), args: json!({}) })]
fn parse_reply_gives_text_or_call(#[case] text: &str, #[case] expected: Reply) {
    assert_that!(parse(text)).is_equal_to(expected);
}

#[rstest]
#[case::bad_json("<tool_call>{name: save}</tool_call>")]
#[case::no_name("<tool_call>{\"arguments\": {}}</tool_call>")]
#[case::args_not_object("<tool_call>{\"name\": \"save\", \"arguments\": [1]}</tool_call>")]
fn parse_malformed_call_is_invalid(#[case] text: &str) {
    assert_that!(matches!(parse(text), Reply::Invalid(_))).is_true();
}
