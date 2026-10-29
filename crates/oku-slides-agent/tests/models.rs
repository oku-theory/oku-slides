//! The ACP agent's model, chosen in the box and asked of each session before a prompt.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
mod common;

use common::{Relayed, TestStack, cmd, logged, read_log, stack_with_sink_and_log, wait_for, wait_turn_done};
use serde_json::{Value, json};
use speculoos::prelude::*;

/// A fake agent offering `fast` (in use) and `deep`, resuming its sessions; `crash` kills it.
fn script() -> Value {
    json!({
        "resume": true,
        "models": [{ "id": "fast", "name": "Fast" }, { "id": "deep", "name": "Deep" }],
        "turns": [{ "chunks": ["ok"] }],
        "by_prompt": { "crash": { "crash": true } }
    })
}

/// Sends `text` from `tab` and waits for its turn `n` to end.
async fn prompt(stack: &TestStack, rx: &Relayed, tab: &str, text: &str, n: u32) {
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": tab, "text": text }))).unwrap_or_else(|e| panic!("prompt: {e}"));
    wait_turn_done(rx, tab, n).await;
}

/// Picks model `id` in the box of `tab` (writes `config.toml`: off the runtime threads).
fn click_model(stack: &TestStack, tab: &str, id: &str) -> Result<(), String> {
    tokio::task::block_in_place(|| {
        stack.handle(cmd(json!({ "cmd": "show_settings", "tab": tab })))?;
        let click = json!({ "cmd": "surface_action", "tab": tab, "surface": "host-voice", "name": "agent_model",
            "context": { "id": id } });
        stack.handle(cmd(click))
    })
}

/// The model values the fake agent was asked, in order.
fn asked(log: &std::path::Path) -> Vec<String> {
    logged(log, "session/set_config_option")
        .iter()
        .map(|p| p["value"].as_str().unwrap_or_else(|| panic!("value: {p}")).to_owned())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_model_after_a_first_reply_shows_the_offered_models_in_the_box() {
    let (stack, rx, _) = stack_with_sink_and_log(script(), &["t-1"]).await;
    prompt(&stack, &rx, "t-1", "un", 1).await;
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "show_settings", "tab": "t-1" }))))
        .unwrap_or_else(|e| panic!("box: {e}"));
    wait_for(&rx, |(ch, v)| {
        *ch == "mcp" && v["tab"] == "t-1" && v.to_string().contains("a-model") && v.to_string().contains("Deep")
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_model_box_open_during_the_first_reply_lists_the_models_once_told() {
    let (stack, rx, _) = stack_with_sink_and_log(script(), &["t-1"]).await;
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "show_settings", "tab": "t-1" }))))
        .unwrap_or_else(|e| panic!("box: {e}"));
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "un" }))).unwrap_or_else(|e| panic!("prompt: {e}"));
    // Nothing reopens the box: a revision refreshes it.
    wait_for(&rx, |(ch, v)| *ch == "mcp" && v["tab"] == "t-1" && v.to_string().contains("Deep")).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_model_choice_is_saved_in_the_preset() {
    let (stack, rx, log) = stack_with_sink_and_log(script(), &["t-1"]).await;
    prompt(&stack, &rx, "t-1", "un", 1).await;
    click_model(&stack, "t-1", "deep").unwrap_or_else(|e| panic!("model: {e}"));
    let data = log.parent().unwrap_or_else(|| panic!("data dir"));
    let toml = std::fs::read_to_string(data.join("config.toml")).unwrap_or_else(|e| panic!("config.toml: {e}"));
    assert_that!(toml.contains("[agent.presets.faux]") && toml.contains("model = \"deep\"")).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_model_choice_is_asked_before_the_next_prompt() {
    let (stack, rx, log) = stack_with_sink_and_log(script(), &["t-1"]).await;
    prompt(&stack, &rx, "t-1", "un", 1).await;
    click_model(&stack, "t-1", "deep").unwrap_or_else(|e| panic!("model: {e}"));
    prompt(&stack, &rx, "t-1", "deux", 2).await;
    let methods: Vec<String> = read_log(&log)
        .iter()
        .filter_map(|l| l["method"].as_str().map(str::to_owned))
        .filter(|m| m == "session/set_config_option" || m == "session/prompt")
        .collect();
    assert_that!(methods)
        .is_equal_to(["session/prompt", "session/set_config_option", "session/prompt"].map(str::to_owned).to_vec());
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_model_in_use_is_not_asked_again() {
    let (stack, rx, log) = stack_with_sink_and_log(script(), &["t-1"]).await;
    prompt(&stack, &rx, "t-1", "un", 1).await;
    click_model(&stack, "t-1", "deep").unwrap_or_else(|e| panic!("model: {e}"));
    prompt(&stack, &rx, "t-1", "deux", 2).await;
    prompt(&stack, &rx, "t-1", "trois", 3).await;
    assert_that!(asked(&log)).is_equal_to(vec!["deep".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_model_is_asked_again_of_a_session_resumed_after_a_restart() {
    let (stack, rx, log) = stack_with_sink_and_log(script(), &["t-1"]).await;
    prompt(&stack, &rx, "t-1", "un", 1).await;
    click_model(&stack, "t-1", "deep").unwrap_or_else(|e| panic!("model: {e}"));
    // The agent dies after taking `deep`; relaunched, it resumes the session on `fast`.
    prompt(&stack, &rx, "t-1", "crash", 2).await;
    prompt(&stack, &rx, "t-1", "trois", 3).await;
    assert_that!(logged(&log, "session/resume")).has_length(1);
    assert_that!(asked(&log)).is_equal_to(vec!["deep".to_owned(), "deep".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_model_is_asked_of_a_session_resumed_without_options() {
    let mut script = script();
    script["no_options_on_reopen"] = json!(true);
    let (stack, rx, log) = stack_with_sink_and_log(script, &["t-1"]).await;
    prompt(&stack, &rx, "t-1", "un", 1).await;
    click_model(&stack, "t-1", "deep").unwrap_or_else(|e| panic!("model: {e}"));
    prompt(&stack, &rx, "t-1", "crash", 2).await;
    // The resume reply tells no options: the offer of the agent's earlier sessions still holds.
    prompt(&stack, &rx, "t-1", "trois", 3).await;
    assert_that!(asked(&log)).is_equal_to(vec!["deep".to_owned(), "deep".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_model_box_still_lists_the_models_after_a_resume_without_options() {
    let mut script = script();
    script["no_options_on_reopen"] = json!(true);
    let (stack, rx, _) = stack_with_sink_and_log(script, &["t-1"]).await;
    prompt(&stack, &rx, "t-1", "un", 1).await;
    prompt(&stack, &rx, "t-1", "crash", 2).await;
    prompt(&stack, &rx, "t-1", "trois", 3).await;
    assert_that!(click_model(&stack, "t-1", "deep")).is_ok();
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_model_not_offered_is_refused() {
    let (stack, rx, _) = stack_with_sink_and_log(script(), &["t-1"]).await;
    prompt(&stack, &rx, "t-1", "un", 1).await;
    assert_that!(click_model(&stack, "t-1", "huge"))
        .is_err_containing("Cet agent ne propose pas ce modèle.".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_model_before_any_reply_is_refused() {
    let (stack, _, _) = stack_with_sink_and_log(script(), &["t-1"]).await;
    assert_that!(click_model(&stack, "t-1", "deep"))
        .is_err_containing("Cet agent ne propose pas ce modèle.".to_owned());
}
