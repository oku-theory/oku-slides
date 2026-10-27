//! A theme tab over MCP: its outline, its draft's parents, and what it refuses.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
mod common;

use common::*;
use oku_slides_session::{LibraryKind, PAPIER_THEME, PRO_THEME, Themes};
use serde_json::json;
use speculoos::prelude::*;

/// The `WrongDocument` refusal naming `kind`, as the harness's localizer says it.
fn wrong_document(h: &Harness, kind: &str) -> String {
    h.localizer.tr_with("session-wrong-document", &[("kind", kind.into())])
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tab_outline_reports_the_edited_theme() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    let papier = Themes::builtins().iter().find(|b| b.manifest.id == PAPIER_THEME).unwrap().clone();
    assert_that!(outline["mode"].as_str()).is_equal_to(Some("theme"));
    assert_that!(outline["theme"].clone()).is_equal_to(serde_json::Value::Null);
    assert_that!(outline["edited_theme"]).is_equal_to(json!({
        "id": papier.manifest.id, "version": papier.version, "namespace": "papier", "name": "Papier",
        "child": true, "extends": [{ "namespace": "papier", "name": "Papier" }], "dirty": false,
    }));
}

#[tokio::test(flavor = "multi_thread")]
async fn deck_outline_edited_theme_has_namespace() {
    let h = start_theme_tab(PRO_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    assert_that!(outline["edited_theme"]["namespace"].as_str()).is_equal_to(Some("pro"));
}

#[tokio::test(flavor = "multi_thread")]
async fn deck_outline_of_a_deck_tab_has_no_edited_theme() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    assert_that!((outline["mode"].as_str(), outline["edited_theme"].is_null())).is_equal_to((Some("deck"), true));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_theme_extends_in_a_theme_tab_sets_the_parents() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "set_theme_extends", json!({ "extends": ["pro"] })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    assert_that!(outline["edited_theme"]["extends"].clone())
        .is_equal_to(json!([{ "namespace": "pro", "name": "Pro" }]));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_theme_extends_outside_a_theme_tab_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "set_theme_extends", json!({ "extends": ["pro"] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(wrong_document(&h, "deck"));
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tab_apply_of_set_theme_is_refused() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "apply", json!({ "ops": [{ "op": "set_theme", "theme": theme_ref("pro") }] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(wrong_document(&h, "theme"));
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    assert_that!(outline["edited_theme"]["name"].as_str()).is_equal_to(Some("Papier"));
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tab_apply_of_a_token_op_changes_the_draft() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let op = json!({ "op": "set_token", "path": ["color", "accent"], "value": "#ff0000" });
    let r = call(&c, "apply", json!({ "ops": [op] })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    assert_that!(outline["edited_theme"]["dirty"].as_bool()).is_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tab_set_widget_is_refused() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let source = "#import \"/oku-slides/lib.typ\": widget\n#let widget = widget(\"w\", (args, ctx) => [])\n";
    let r = call(&c, "set_widget", json!({ "name": "w", "source": source })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(wrong_document(&h, "theme"));
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tab_remove_pack_is_refused() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "remove_pack", json!({ "kind": "layouts", "namespace": "essentiel" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(wrong_document(&h, "theme"));
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tab_save_copy_is_refused_without_asking() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let dir = temp_dir("oku-slides-theme-tab-copy-");
    let path = dir.path().join("copie.oku-slides.pdf");
    let r = call(&c, "save_copy", json!({ "path": path.to_string_lossy() })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(wrong_document(&h, "theme"));
    assert_that!((path.exists(), h.hub.has_pending_ask())).is_equal_to((false, false));
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tab_new_deck_is_refused() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let r = guarded(&h, "new", json!({}), true).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(wrong_document(&h, "theme"));
    assert_that!(h.session.lock().unwrap().document_kind()).is_equal_to(oku_slides_session::DocumentKind::Theme);
}
