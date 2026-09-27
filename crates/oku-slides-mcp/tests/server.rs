mod common;

use common::*;
use rust_mcp_sdk::McpClient;
use speculoos::prelude::*;

// `ClientRuntime` n'implémente pas `Debug` : `ResultAssertions` l'exige même
// pour `is_err`, donc on efface le côté `Ok` avant l'assertion.
#[tokio::test(flavor = "multi_thread")]
async fn without_token_client_is_refused() {
    let h = start().await;
    assert_that!(client(h.server.url(), None).await.map(|_| ())).is_err();
    assert_that!(client(h.server.url(), Some("Bearer faux".into())).await.map(|_| ())).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_list_names_every_read_tool() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let names: Vec<String> = c.request_tool_list(None).await.unwrap().tools.into_iter().map(|t| t.name).collect();
    for n in ["deck_outline", "slide", "render_slide", "templates", "settings"] {
        assert_that!(names.contains(&n.to_owned())).is_true();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn deck_outline_lists_slides_with_titles() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let outline = json_of(&call(&c, "deck_outline", serde_json::json!({})).await);
    assert_that!(outline["slides"].as_array().map(Vec::len)).is_equal_to(Some(8));
    assert_that!(outline["mode"].as_str()).is_equal_to(Some("deck"));
}

#[tokio::test(flavor = "multi_thread")]
async fn render_slide_returns_png_image() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "render_slide", serde_json::json!({ "id": "s-libre" })).await;
    let block = serde_json::to_value(&r.content).unwrap();
    assert_that!(block[0]["type"].as_str()).is_equal_to(Some("image"));
    assert_that!(block[0]["mimeType"].as_str()).is_equal_to(Some("image/png"));
}

#[tokio::test(flavor = "multi_thread")]
async fn slide_unknown_id_returns_tool_error() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "slide", serde_json::json!({ "id": "inconnue" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r).contains("slide not found")).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_returns_port_values() {
    let h = start().await;
    h.settings.values.lock().unwrap().insert("dictation.model".into(), serde_json::json!("turbo"));
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let v = json_of(&call(&c, "settings", serde_json::json!({})).await);
    assert_that!(v["dictation.model"].as_str()).is_equal_to(Some("turbo"));
}
