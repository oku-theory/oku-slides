//! t-1462: `diagnose` shows a tab only its own log lines and those of no tab, and never the
//! tab's MCP bearer token.

use oku_slides_platform::telemetry;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::{Harness, call, json_of, server, start};

/// The `log` messages of `h`'s `diagnose` reply that start with `marker`.
async fn marked(h: &Harness, marker: &str) -> Vec<String> {
    let c = server(h).await;
    let v: Value = json_of(&call(&c, "diagnose", json!({})).await);
    let log = v["log"].as_array().cloned().unwrap_or_default();
    log.iter().filter_map(|l| l["message"].as_str()).filter(|m| m.starts_with(marker)).map(str::to_owned).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn diagnose_log_of_another_tab_is_not_shown() {
    telemetry::init();
    let h = start().await;
    tracing::warn!(tab = "t-1", "iso-marker mine");
    tracing::warn!(tab = "t-2", "iso-marker theirs");
    tracing::warn!("iso-marker app");
    let messages = marked(&h, "iso-marker").await;
    assert_that!(messages).is_equal_to(vec!["iso-marker mine tab=t-1".to_owned(), "iso-marker app".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn diagnose_log_never_shows_the_tab_token() {
    telemetry::init();
    let h = start().await;
    let authorization = h.server.authorization();
    tracing::warn!(header = %authorization, "token-marker");
    let messages = marked(&h, "token-marker").await;
    let token = authorization.trim_start_matches("Bearer ");
    assert_that!(messages.len()).is_equal_to(1);
    assert_that!(messages.iter().any(|m| m.contains(token))).is_false();
}
