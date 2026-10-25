//! L'agent est-il utilisable ? Programme introuvable, lancement raté, puis réussi (t-901).
#![cfg(unix)]
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use common::{
    Relayed, TestStack, cmd, logged, relayed_len, stack_with_builtin, stack_with_command, stack_with_sink,
    stack_with_sink_and_log, wait_n_after, wait_turn_done,
};
use oku_slides_i18n::Localizer;
use oku_slides_platform::local_agent::Target;
use serde_json::{Value, json};
use speculoos::prelude::*;

/// Le texte de `key` en français (la langue des piles de test), pour le programme `command`.
fn fr(key: &str, command: &str) -> String {
    Localizer::new(Some("fr")).tr_with(key, &[("command", command.into())])
}

/// Demande la disponibilité de l'agent pour `t-1` et rend le `unusable` de la réponse.
///
/// `handle` lit le disque : hors des fils du runtime, comme dans les hôtes.
async fn availability(stack: &TestStack, rx: &Relayed) -> Value {
    let from = relayed_len(rx);
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "availability", "tab": "t-1" }))))
        .unwrap_or_else(|e| panic!("availability : {e}"));
    let got = wait_n_after(rx, from, 1, is_availability).await;
    got[0].1.1["event"]["unusable"].clone()
}

fn is_availability((ch, v): &(&'static str, Value)) -> bool {
    *ch == "agent" && v["tab"] == "t-1" && v["event"]["kind"] == "availability"
}

/// Pose les droits `mode` sur `path`.
fn chmod(path: &Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap_or_else(|e| panic!("droits : {e}"));
}

#[tokio::test(flavor = "multi_thread")]
async fn availability_with_the_fake_agent_reports_none() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    assert_that!(availability(&stack, &rx).await).is_equal_to(Value::Null);
}

#[tokio::test(flavor = "multi_thread")]
async fn availability_with_a_missing_preset_command_reports_not_found() {
    let command = "/nonexistent/oku-slides-agent-absent";
    let (stack, rx) = stack_with_command(json!({ "turns": [] }), command).await;
    assert_that!(availability(&stack, &rx).await).is_equal_to(json!(fr("agent-not-found", command)));
}

#[tokio::test(flavor = "multi_thread")]
async fn availability_with_a_bare_command_missing_from_path_reports_not_found() {
    let command = "oku-slides-agent-absent-du-path";
    let (stack, rx) = stack_with_command(json!({ "turns": [] }), command).await;
    assert_that!(availability(&stack, &rx).await).is_equal_to(json!(fr("agent-not-found", command)));
}

#[tokio::test(flavor = "multi_thread")]
async fn tune_with_an_unusable_agent_tells_the_tab_why() {
    let command = "/nonexistent/oku-slides-agent-absent";
    let (stack, rx) = stack_with_command(json!({ "turns": [{}] }), command).await;
    tokio::task::block_in_place(|| stack.tune("t-1"));
    let told = wait_n_after(&rx, 0, 1, is_availability).await;
    assert_that!(told[0].1.1["event"]["unusable"].clone()).is_equal_to(json!(fr("agent-not-found", command)));
}

#[tokio::test(flavor = "multi_thread")]
async fn tune_with_an_unusable_agent_starts_no_session() {
    // Un agent sans MCP en HTTP : son lancement raté est retenu, il est donc inutilisable.
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "http": false, "turns": [{}] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "un" }))).unwrap();
    // Le lancement raté le dit, puis la boîte des réglages, rafraîchie à la fin de la requête.
    wait_n_after(&rx, 0, 2, is_availability).await;
    let from = relayed_len(&rx);
    tokio::task::block_in_place(|| stack.tune("t-1"));
    // Témoin : la réponse à une demande faite après `tune` arrive après tout ce que `tune` a relayé.
    assert_that!(availability(&stack, &rx).await).is_equal_to(json!(fr("agent-no-http", "")));
    let after: Vec<_> = rx.lock().unwrap()[from..].to_vec();
    assert_that!(after.iter().all(is_availability)).is_true();
    assert_that!(after.len()).is_equal_to(2);
    assert_that!(logged(&log, "session/new")).is_empty();
    assert_that!(logged(&log, "initialize").len()).is_equal_to(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn availability_after_a_failed_launch_says_why_until_a_launch_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let agent = dir.path().join("agent");
    // Le faux agent, présent mais pas exécutable : son lancement échoue (pas « introuvable »).
    std::fs::copy(env!("CARGO_BIN_EXE_oku-slides-fake-agent"), &agent).unwrap();
    chmod(&agent, 0o644);
    let command = agent.to_string_lossy().into_owned();
    let (stack, rx) = stack_with_command(json!({ "turns": [{ "chunks": ["ok"] }] }), &command).await;
    let spawn = json!(fr("agent-spawn-failed", &command));
    // Avant tout lancement, le programme existe : rien à dire.
    assert_that!(availability(&stack, &rx).await).is_equal_to(Value::Null);
    let from = relayed_len(&rx);
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "un" }))).unwrap();
    // L'échec du lancement est annoncé de lui-même : un dialogue ouvert se met à jour.
    let told = wait_n_after(&rx, from, 1, is_availability).await;
    assert_that!(told[0].1.1["event"]["unusable"].clone()).is_equal_to(spawn.clone());
    assert_that!(availability(&stack, &rx).await).is_equal_to(spawn);
    chmod(&agent, 0o755);
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "deux" }))).unwrap();
    wait_turn_done(&rx, "t-1", 2).await;
    assert_that!(availability(&stack, &rx).await).is_equal_to(Value::Null);
}

#[tokio::test(flavor = "multi_thread")]
async fn availability_builtin_without_program_names_it() {
    let (stack, rx) = stack_with_builtin(json!({ "turns": [] })).await;
    let expected = Localizer::new(Some("fr")).tr("agent-local-no-program");
    assert_that!(availability(&stack, &rx).await).is_equal_to(json!(expected));
}

/// La boîte des réglages, envoyée entière à `t-1`.
fn is_voice_box((ch, v): &(&'static str, Value)) -> bool {
    *ch == "mcp" && v["tab"] == "t-1" && v["event"]["kind"] == "surface" && v["event"]["surface"]["id"] == "host-voice"
}

/// Le texte du composant `id` de la boîte des réglages relayée, s'il y est.
fn box_text(event: &Value, id: &str) -> Option<String> {
    let messages = event["event"]["surface"]["messages"].as_array()?;
    let found = messages
        .iter()
        .filter_map(|m| m["updateComponents"]["components"].as_array())
        .flatten()
        .find(|c| c["id"] == id)?;
    found["text"].as_str().map(str::to_owned)
}

/// Ouvre la boîte des réglages de `t-1`, puis y clique `name` ; rend la réponse et l'index d'après l'ouverture.
async fn click_in_box(stack: &TestStack, rx: &Relayed, name: &str) -> (Result<(), String>, usize) {
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "show_settings", "tab": "t-1" }))))
        .unwrap_or_else(|e| panic!("show_settings : {e}"));
    wait_n_after(rx, 0, 1, is_voice_box).await;
    let from = relayed_len(rx);
    let click = json!({ "cmd": "surface_action", "tab": "t-1", "surface": "host-voice", "name": name, "context": {} });
    (tokio::task::block_in_place(|| stack.handle(cmd(click))), from)
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_agent_model_download_without_download_is_refused() {
    let (stack, rx) = stack_with_builtin(json!({ "turns": [] })).await;
    let (r, _) = click_in_box(&stack, &rx, "cancel_agent_model_download").await;
    assert_that!(r).is_err_containing("Aucun téléchargement n'est en cours".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_agent_model_download_refused_resends_the_whole_box() {
    let (stack, rx) = stack_with_builtin(json!({ "turns": [] })).await;
    let (_, from) = click_in_box(&stack, &rx, "cancel_agent_model_download").await;
    let got = wait_n_after(&rx, from, 1, is_voice_box).await;
    assert_that!(box_text(&got[0].1.1, "a-error")).is_none();
}

#[tokio::test(flavor = "multi_thread")]
async fn download_agent_model_unreachable_hub_shows_the_failure_and_stays_unusable() {
    let (stack, rx) = stack_with_builtin(json!({ "turns": [] })).await;
    let (r, from) = click_in_box(&stack, &rx, "download_agent_model").await;
    if Target::current() != Target::AppleSilicon {
        // Rien à télécharger ici : refusé, et la boîte revient entière, sans erreur.
        assert_that!(r).is_err_containing("Rien à télécharger pour cet agent".to_owned());
        let got = wait_n_after(&rx, from, 1, is_voice_box).await;
        assert_that!(box_text(&got[0].1.1, "a-error")).is_none();
        return;
    }
    let failed = Localizer::new(Some("fr")).tr("agent-model-download-failed");
    let shown = common::wait_for(&rx, |r| is_voice_box(r) && box_text(&r.1, "a-error").is_some()).await;
    assert_that!(box_text(&shown.1, "a-error")).is_equal_to(Some(failed));
    // Après l'échec, l'agent reste inutilisable : la disponibilité le dit toujours (seul un
    // changement est réémis de lui-même, voir `voice_refresh`).
    let unusable = Localizer::new(Some("fr")).tr("agent-local-no-program");
    assert_that!(availability(&stack, &rx).await).is_equal_to(json!(unusable));
}
