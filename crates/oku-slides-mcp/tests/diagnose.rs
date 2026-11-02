//! The read-only `diagnose` tool: every kind lists it; redaction, the WARN floor and the caps hold.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
mod common;

use common::kinds::{Kind, list_tool_names};
use common::*;
use oku_slides_mcp::{AgentDiagnosis, Diagnosis};
use oku_slides_platform::telemetry;
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

/// The `diagnose` reply of `h`'s tab.
async fn diagnose(h: &Harness) -> Value {
    let c = server(h).await;
    json_of(&call(&c, "diagnose", json!({})).await)
}

/// The user's home dir, as the redaction reads it.
fn home() -> &'static str {
    telemetry::home().unwrap_or_else(|| panic!("no home dir for the test"))
}

#[rstest]
#[case::deck(Kind::Deck)]
#[case::help(Kind::Help)]
#[case::pack(Kind::Pack)]
#[case::theme(Kind::Theme)]
#[tokio::test(flavor = "multi_thread")]
async fn diagnose_is_listed_and_served_for_every_kind(#[case] kind: Kind) {
    let h = kind.start().await;
    let c = server(&h).await;
    assert_that!(list_tool_names(&c).await).contains("diagnose".to_owned());
    let v = json_of(&call(&c, "diagnose", json!({})).await);
    let mut keys: Vec<&str> =
        v.as_object().unwrap_or_else(|| panic!("not an object: {v}")).keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_that!(keys).is_equal_to(vec!["agent", "dictation", "import_error", "log", "render_error"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn diagnose_stderr_line_under_home_comes_back_with_a_tilde_and_capped() {
    let h = start().await;
    let stderr = (0..60).map(|n| format!("{}/.npm/log-{n}", home())).collect();
    *lock(&h.voice.diagnosis) = Diagnosis {
        agent: AgentDiagnosis { selected: "claude".into(), stderr, ..AgentDiagnosis::default() },
        dictation: None,
    };
    let v = diagnose(&h).await;
    let lines = v["agent"]["stderr"].as_array().unwrap_or_else(|| panic!("no stderr: {v}"));
    assert_that!(lines.len()).is_equal_to(40);
    assert_that!(lines[0].as_str()).is_equal_to(Some("~/.npm/log-20"));
    assert_that!(v["dictation"].is_null()).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn diagnose_log_keeps_warnings_redacted_never_info_and_capped() {
    telemetry::init();
    let h = start().await;
    tracing::info!("diagnose-info-marker");
    for n in 0..60 {
        tracing::warn!(path = format!("{}/deck-{n}.pdf", home()), "diagnose-warn-marker");
    }
    let v = diagnose(&h).await;
    let log = v["log"].as_array().unwrap_or_else(|| panic!("no log: {v}"));
    assert_that!(log.len()).is_less_than_or_equal_to(50);
    assert_that!(log.iter().any(|l| l["level"] == "INFO")).is_false();
    // Other parts of the app may log too (the MCP transport): only the markers are checked.
    let markers: Vec<&str> =
        log.iter().filter_map(|l| l["message"].as_str()).filter(|m| m.starts_with("diagnose-warn-marker")).collect();
    assert_that!(markers.last().copied()).is_equal_to(Some("diagnose-warn-marker path=~/deck-59.pdf"));
}

#[tokio::test(flavor = "multi_thread")]
async fn diagnose_after_a_failed_render_tells_the_render_error() {
    let h = start().await;
    let bad = json!({ "ops": [{ "op": "set_token", "path": ["size", "body"], "value": true }] });
    let c = server(&h).await;
    assert_that!(call(&c, "apply", bad).await.is_error).is_equal_to(Some(true));
    let v = json_of(&call(&c, "diagnose", json!({})).await);
    assert_that!(v["render_error"].as_str().is_some_and(|s| !s.is_empty())).is_true();
    assert_that!(v["import_error"].is_null()).is_true();
}
