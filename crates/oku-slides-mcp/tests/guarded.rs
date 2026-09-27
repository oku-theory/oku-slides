mod common;

use common::*;
use oku_slides_mcp::{SettingsPort, SurfaceAction};
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

fn answer(h: &Harness, surface: &str, confirmed: bool) {
    let mut context = Map::new();
    context.insert("confirmed".into(), Value::Bool(confirmed));
    h.hub
        .act(SurfaceAction { surface: surface.into(), name: "confirm".into(), context })
        .unwrap_or_else(|e| panic!("réponse à la surface : {e}"));
}

async fn guarded(
    h: &Harness,
    tool: &'static str,
    args: Value,
    confirmed: bool,
) -> rust_mcp_sdk::schema::CallToolResult {
    let mut rx = h.hub.subscribe();
    let c =
        client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("client de test : {e}"));
    let pending = tokio::spawn(async move { call(&c, tool, args).await });
    let surface = next_surface(&mut rx).await;
    let json = serde_json::to_value(&surface).unwrap_or_else(|e| panic!("sérialisation de la surface : {e}"));
    assert_that!(json["place"].as_str()).is_equal_to(Some("modal"));
    answer(h, &surface.id, confirmed);
    pending.await.unwrap_or_else(|e| panic!("tâche de l'outil : {e}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn save_denied_writes_nothing() {
    let h = start().await;
    let path = std::env::temp_dir().join(format!("oku-slides-guarded-{}.oku-slides.pdf", std::process::id()));
    std::fs::remove_file(&path).ok();
    let r = guarded(&h, "save", json!({ "path": path.to_string_lossy() }), false).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r).as_str()).is_equal_to("denied by user");
    assert_that!(path.exists()).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn save_confirmed_writes_file() {
    let h = start().await;
    let path = std::env::temp_dir().join(format!("oku-slides-guarded-ok-{}.oku-slides.pdf", std::process::id()));
    let r = guarded(&h, "save", json!({ "path": path.to_string_lossy() }), true).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(path.exists()).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn open_with_parent_component_is_refused_without_asking() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "open", json!({ "path": "/tmp/../etc/deck.pdf" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(h.hub.has_pending_ask()).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn open_of_a_file_open_in_another_tab_is_refused_without_asking() {
    let h = start_open_elsewhere().await;
    let before =
        h.session.lock().unwrap_or_else(std::sync::PoisonError::into_inner).snapshot().path.map(|p| p.to_path_buf());
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let path = std::env::temp_dir().join("oku-slides-ailleurs.oku-slides.pdf");
    let r = call(&c, "open", json!({ "path": path.to_string_lossy() })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-open-elsewhere"));
    assert_that!(h.hub.has_pending_ask()).is_false();
    let after =
        h.session.lock().unwrap_or_else(std::sync::PoisonError::into_inner).snapshot().path.map(|p| p.to_path_buf());
    assert_that!(after).is_equal_to(before);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_setting_outside_whitelist_is_refused_without_asking() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "set_setting", json!({ "key": "paths.data", "value": "/tmp" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(h.settings.read().contains_key("paths.data")).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_setting_confirmed_writes_through_port() {
    let h = start().await;
    let r = guarded(&h, "set_setting", json!({ "key": "dictation.language", "value": "fr" }), true).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "applied": "now" }));
    assert_that!(h.settings.read().get("dictation.language").cloned()).is_equal_to(Some(json!("fr")));
}
