//! The `a2ui_examples` tool over MCP dispatch: full list, filtered list, unknown keys.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
mod common;

use common::*;
use serde_json::json;
use speculoos::prelude::*;

/// The `component` names of an `a2ui_examples` result.
fn example_components(r: &rust_mcp_sdk::schema::CallToolResult) -> Vec<String> {
    let Some(list) = json_of(r).as_array().cloned() else {
        panic!("a2ui_examples did not return a JSON array: {}", text_of(r));
    };
    list.iter().filter_map(|c| c["component"].as_str().map(str::to_owned)).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a2ui_examples_without_a_list_returns_every_render_component() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "a2ui_examples", json!({})).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let names = example_components(&r);
    let expected: Vec<String> =
        oku_slides_a2ui::agent_reference().unwrap().iter().map(|c| c.component.clone()).collect();
    assert_that!(names).is_equal_to(expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn a2ui_examples_with_a_list_returns_only_those_components() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "a2ui_examples", json!({ "components": ["Card"] })).await;
    assert_that!(example_components(&r)).is_equal_to(vec!["Card".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a2ui_examples_with_an_unknown_key_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "a2ui_examples", json!({ "names": ["Card"] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).contains("unknown field `names`");
}
