use serde_json::json;
use speculoos::prelude::*;

use super::{json_response, sse_response};

#[test]
fn sse_response_skips_other_messages_and_finds_the_id() {
    let body = "id: s_1\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\"}\n\n\
                id: s_2\r\ndata: {\"jsonrpc\":\"2.0\",\"id\":3,\"result\":{\"ok\":true}}\r\n\r\n";
    assert_that!(sse_response(body, 3))
        .is_some()
        .is_equal_to(json!({"jsonrpc": "2.0", "id": 3, "result": {"ok": true}}));
}

#[test]
fn sse_response_joins_multi_line_data() {
    let body = "data: {\"jsonrpc\":\"2.0\",\ndata: \"id\":1,\"error\":{\"code\":-1}}\n\n";
    assert_that!(sse_response(body, 1)).is_some();
}

#[test]
fn sse_response_another_id_is_none() {
    let body = "data: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}\n\n";
    assert_that!(sse_response(body, 1)).is_none();
}

#[test]
fn json_response_finds_the_id_in_a_batch() {
    let body = r#"[{"jsonrpc":"2.0","id":1,"result":{"a":1}},{"jsonrpc":"2.0","id":2,"result":{"b":2}}]"#;
    assert_that!(json_response(body, 2)).is_some().is_equal_to(json!({"jsonrpc": "2.0", "id": 2, "result": {"b": 2}}));
}

#[test]
fn json_response_request_from_the_server_is_none() {
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    assert_that!(json_response(body, 1)).is_none();
}
