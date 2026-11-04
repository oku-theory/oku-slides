//! The app stack on its built-in preset: `[program, --local-agent]`, fake engine, a report.
#![cfg(all(unix, debug_assertions))]
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use oku_slides_agent::{AgentCommand, Stack};
use oku_slides_platform::config::{AgentPreset, Builtin, LocalEngine};
use oku_slides_session::{Fonts, Platform, Session, Settings};
use serde_json::{Value, json};
use speculoos::prelude::*;

type Relayed = Arc<Mutex<Vec<(&'static str, Value)>>>;

/// A shell script standing for the app binary: the test's environment, then the dev launcher.
///
/// The stack launches a built-in preset as `<program> --local-agent` with the app's own
/// environment; the script gives the child its fake engine and its temp dirs.
fn program(dir: &Path, script: &Path) -> PathBuf {
    let quoted = |p: &Path| format!("'{}'", p.display());
    let text = format!(
        "#!/bin/sh\nexport OKU_SLIDES_LOCAL_AGENT__ENGINE=fake\nexport OKU_SLIDES_LOCAL_AGENT__FAKE_SCRIPT={}\n\
         export OKU_SLIDES_PATHS__DATA={}\nexport OKU_SLIDES_PATHS__DECKS={}\nexec {} \"$@\"\n",
        quoted(script),
        quoted(dir),
        quoted(&dir.join("decks")),
        quoted(Path::new(env!("CARGO_BIN_EXE_oku-slides-local-agent-dev"))),
    );
    let path = dir.join("oku-slides");
    std::fs::write(&path, text).unwrap_or_else(|e| panic!("programme : {e}"));
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap_or_else(|e| panic!("droits : {e}"));
    path
}

/// The stack on the built-in preset `local`, one session on the roadmap fixture, relayed into a list.
async fn stack(dir: &Path, script: Value) -> (Arc<Stack>, Relayed) {
    let script_path = dir.join("script.json");
    std::fs::write(&script_path, script.to_string()).unwrap_or_else(|e| panic!("scénario : {e}"));
    let program = program(dir, &script_path);
    let data = dir.to_owned();
    let (session, platform) = tokio::task::spawn_blocking(move || {
        let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
        settings.paths.data = Some(data.clone());
        settings.paths.decks = Some(data.join("decks"));
        settings.i18n.locale = Some("fr".into());
        settings.models.hub_url = "http://127.0.0.1:9".into();
        settings.local_agent.engine = LocalEngine::Fake;
        settings.local_agent.fake_script = Some(script_path);
        let agent = &mut settings.agent;
        agent.insert_preset("local", AgentPreset::builtin("Local", Builtin::Local)).unwrap_or_else(|e| panic!("{e}"));
        agent.select("local").unwrap_or_else(|e| panic!("agent local : {e}"));
        let mut platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"));
        platform.program = Some(program);
        let mut session = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session : {e}"));
        session
            .open(&PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json")))
            .unwrap_or_else(|e| panic!("ouverture : {e}"));
        // A tab's resume folder: the agent works in its `work/`, never next to the deck.
        assert!(session.resume(data.join("tabs").join("local")), "reprise de l'onglet");
        (session, platform)
    })
    .await
    .unwrap_or_else(|e| panic!("tâche bloquante : {e}"));
    let sessions: Arc<dyn oku_slides_session::agent::Sessions> = Arc::new(Mutex::new(session));
    let stack = Stack::start(
        sessions,
        &platform,
        oku_slides_agent::Voices::default(),
        Arc::new(oku_slides_session::agent::NoWorkspace),
        Arc::new(oku_slides_session::agent::UnavailableApp),
        tokio::runtime::Handle::current(),
    );
    // `Storage` owns a runtime: its last copy must not drop inside the async test.
    std::mem::forget(platform);
    let seen: Relayed = Arc::default();
    let sink = seen.clone();
    stack.forward(move |channel, text| {
        let value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("JSON relayé : {e}"));
        sink.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push((channel, value));
    });
    (stack, seen)
}

/// The first relayed message that `pred` accepts, within 30 s.
async fn wait_for(rx: &Relayed, pred: impl Fn(&(&'static str, Value)) -> bool) -> (&'static str, Value) {
    for _ in 0..1500 {
        let found = rx.lock().unwrap_or_else(std::sync::PoisonError::into_inner).iter().find(|r| pred(r)).cloned();
        if let Some(found) = found {
            return found;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("relais attendu non reçu");
}

#[tokio::test(flavor = "multi_thread")]
async fn builtin_preset_prompt_runs_the_local_agent_and_ends_with_a_report() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("dossier : {e}"));
    let script = json!({"turns": [{"route": "edit", "replies": [
        {"call": "apply", "args": {"ops": [{"op": "set_deck_param", "key": "title", "value": "Feuille de route 2027"}]}},
        {"call": "report", "args": {"title": "Titre changé", "facts": ["Le titre du deck a changé"]}}
    ]}]});
    let (stack, rx) = stack(dir.path(), script).await;
    let prompt: AgentCommand =
        serde_json::from_value(json!({"cmd": "prompt", "tab": "t-1", "text": "renomme le deck"}))
            .unwrap_or_else(|e| panic!("commande : {e}"));
    tokio::task::block_in_place(|| stack.handle(prompt)).unwrap_or_else(|e| panic!("prompt : {e}"));
    let receipt = wait_for(&rx, |(ch, v)| {
        *ch == "mcp"
            && v["event"]["kind"] == "surface"
            && v["event"]["surface"]["id"].as_str().is_some_and(|id| id.starts_with("report"))
    })
    .await;
    assert_that!(receipt.1.to_string()).contains("Titre changé");
    stack.shutdown().await;
}
