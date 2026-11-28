use speculoos::prelude::*;

use super::{Step, Toggle, step, toggle_step};

#[test]
fn step_detached_without_window_opens_it() {
    assert_that!(step(true, false)).is_equal_to(Step::Open);
}

#[test]
fn step_attached_with_window_closes_it() {
    assert_that!(step(false, true)).is_equal_to(Step::Close);
}

#[test]
fn step_detached_with_window_keeps_it() {
    assert_that!(step(true, true)).is_equal_to(Step::Keep);
}

#[test]
fn step_attached_without_window_keeps_it() {
    assert_that!(step(false, false)).is_equal_to(Step::Keep);
}

#[test]
fn toggle_step_attached_detaches() {
    assert_that!(toggle_step(false, false)).is_equal_to(Toggle::Detach);
}

#[test]
fn toggle_step_detached_with_window_asks_its_page() {
    assert_that!(toggle_step(true, true)).is_equal_to(Toggle::AskPage);
}

#[test]
fn toggle_step_detached_without_window_attaches() {
    assert_that!(toggle_step(true, false)).is_equal_to(Toggle::Attach);
}

/// Une capability de `capabilities/`, lue comme JSON.
fn capability(json: &str) -> serde_json::Value {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("capability illisible : {e}"))
}

/// Les chaînes d'un tableau JSON.
fn strings(v: &serde_json::Value) -> Vec<&str> {
    v.as_array().map(|a| a.iter().filter_map(serde_json::Value::as_str).collect()).unwrap_or_default()
}

#[test]
fn capability_agent_window_has_no_file_dialogs() {
    let agent = capability(include_str!("../capabilities/agent.json"));
    assert_that!(strings(&agent["windows"])).is_equal_to(vec![super::LABEL]);
    assert_that!(strings(&agent["permissions"]).iter().any(|p| p.starts_with("dialog:"))).is_false();
}

#[test]
fn capability_default_leaves_agent_window_out() {
    let default = capability(include_str!("../capabilities/default.json"));
    assert_that!(strings(&default["windows"])).does_not_contain(super::LABEL);
}

/// The opener entries of a capability: each `opener:` permission, named or scoped.
fn opener_entries(capability: &serde_json::Value) -> Vec<&serde_json::Value> {
    let name = |p: &serde_json::Value| p.as_str().or_else(|| p["identifier"].as_str()).map(str::to_owned);
    capability["permissions"]
        .as_array()
        .map(|a| a.iter().filter(|p| name(p).is_some_and(|n| n.starts_with("opener:"))).collect())
        .unwrap_or_default()
}

#[rstest::rstest]
#[case::default(include_str!("../capabilities/default.json"))]
#[case::agent(include_str!("../capabilities/agent.json"))]
fn capability_opener_open_url_scoped_to_web_and_mail(#[case] json: &str) {
    let capability = capability(json);
    let expected = serde_json::json!({
        "identifier": "opener:allow-open-url",
        "allow": [{ "url": "http://*" }, { "url": "https://*" }, { "url": "mailto:*" }],
    });
    assert_that!(opener_entries(&capability)).is_equal_to(vec![&expected]);
}

#[test]
fn capability_default_allows_fullscreen_for_presentation() {
    let default = capability(include_str!("../capabilities/default.json"));
    assert_that!(strings(&default["permissions"])).contains("core:window:allow-set-fullscreen");
}
