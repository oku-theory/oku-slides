//! t-793 : après un changement de langue à chaud, les erreurs d'outils et les
//! confirmations du hub suivent la langue courante, plus celle du lancement.

use std::sync::Arc;

use oku_slides_i18n::Localizer;
use oku_slides_mcp::SurfaceAction;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use crate::common::*;

#[tokio::test(flavor = "multi_thread")]
async fn tool_error_after_language_switch_uses_the_new_language() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let before = text_of(&call(&c, "nope", json!({})).await);
    let german = Localizer::new(Some("de"));
    h.live.set(Arc::new(Localizer::new(Some("de"))));
    let after = text_of(&call(&c, "nope", json!({})).await);
    assert_that!(before).is_equal_to(h.localizer.tr_with("agent-error-unknown-tool", &[("name", "nope".into())]));
    assert_that!(after).is_equal_to(german.tr_with("agent-error-unknown-tool", &[("name", "nope".into())]));
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_confirmation_after_language_switch_uses_the_new_language() {
    let h = start().await;
    h.live.set(Arc::new(Localizer::new(Some("ja"))));
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pending = tokio::spawn(async move { call(&c, "new", json!({})).await });
    let surface = next_surface(&mut rx).await;
    let shown = serde_json::to_string(&surface).unwrap();
    let mut context = Map::new();
    context.insert("confirmed".into(), Value::Bool(false));
    h.hub.act(SurfaceAction { surface: surface.id, name: "confirm".into(), context }).unwrap();
    pending.await.unwrap();
    let japanese = Localizer::new(Some("ja"));
    assert_that!(shown.contains(&japanese.tr("agent-confirm-new"))).is_true();
}
