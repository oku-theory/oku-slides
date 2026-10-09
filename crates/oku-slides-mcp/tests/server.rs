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
async fn deck_outline_names_the_deck_theme_and_lists_the_themes() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let outline = json_of(&call(&c, "deck_outline", serde_json::json!({})).await);
    let papier = oku_slides_session::PAPIER_THEME.to_string();
    assert_that!(outline["theme"]).is_equal_to(serde_json::json!({ "id": papier, "name": "Papier" }));
    let names: Vec<&str> = outline["themes"].as_array().unwrap().iter().filter_map(|t| t["name"].as_str()).collect();
    assert_that!(names).is_equal_to(vec!["Papier", "Pro"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_sets_pro_with_only_what_deck_outline_returns() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let outline = json_of(&call(&c, "deck_outline", serde_json::json!({})).await);
    let pro = outline["themes"].as_array().unwrap().iter().find(|t| t["name"] == "Pro").unwrap().clone();
    let theme = serde_json::json!({ "id": pro["id"], "version": pro["version"] });
    let r = call(&c, "apply", serde_json::json!({ "ops": [{ "op": "set_theme", "theme": theme }] })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let s = h.session.lock().unwrap();
    let r = s.deck().theme.clone().unwrap();
    assert_that!(r.id).is_equal_to(oku_slides_session::PRO_THEME);
    assert_that!(Some(r.version.as_str())).is_equal_to(pro["version"].as_str());
    assert_that!(s.sources_for_tests().themes.contains_key(&r.version)).is_true();
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
    assert_that!(text_of(&r).as_str()).is_equal_to("Slide introuvable : inconnue");
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_returns_port_values() {
    let h = start().await;
    h.settings.values.lock().unwrap().insert("dictation.model".into(), serde_json::json!("turbo"));
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let v = json_of(&call(&c, "settings", serde_json::json!({})).await);
    assert_that!(v["dictation.model"].as_str()).is_equal_to(Some("turbo"));
}
