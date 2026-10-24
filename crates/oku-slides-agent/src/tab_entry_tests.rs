use std::sync::{Arc, Mutex};
use std::time::Duration;

use oku_slides_mcp::{McpEvent, Tools};
use oku_slides_platform::config::PromptImageLimits;
use oku_slides_session::agent::{AgentScope, AgentUi, TabSession};
use oku_slides_session::{Fonts, Platform, Session, Settings};
use serde::ser::{Error as _, Serialize, Serializer};
use speculoos::prelude::*;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{Agent, Relay, emit, next};
use crate::keeper::Keeper;
use crate::settings_port::AppSettings;
use crate::tab_agent::{Command, Instruction};
use crate::turns::Turns;
use crate::{AgentState, Launch, Link};

/// Un évènement qui ne se sérialise jamais.
struct Broken;

impl Serialize for Broken {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("cassé"))
    }
}

#[test]
fn emit_skips_an_event_that_does_not_serialize() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    emit(&tx, "agent", "t-1", &Broken);
    assert_that!(rx.try_recv().is_err()).is_true();
}

#[test]
fn emit_wraps_the_event_with_its_tab() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    emit(
        &tx,
        "agent",
        "t-1",
        &crate::AgentEvent::Conversation { scope: oku_slides_session::agent::AgentScope::Slides },
    );
    let (channel, json) = rx.try_recv().unwrap();
    assert_that!(channel).is_equal_to("agent");
    assert_that!(json.as_str())
        .is_equal_to(r#"{"tab":"t-1","event":{"kind":"conversation","scope":{"kind":"slides"}}}"#);
}

fn prompt() -> Command {
    Command::Prompt(Instruction {
        scope: AgentScope::Slides,
        text: "perdue".into(),
        ui: AgentUi::default(),
        tune: false,
        images: Vec::new(),
    })
}

/// Onglet fermé, commandes encore en file : `next` rend `None`, jamais la commande (sélection `biased`).
#[tokio::test]
async fn next_after_close_with_commands_queued_returns_none() {
    let closed = CancellationToken::new();
    closed.cancel();
    let (tx, mut rx) = mpsc::unbounded_channel();
    // Sans `biased`, chaque appel tirerait au sort : 64 essais rendent l'échec certain (2⁻⁶⁴ près).
    for _ in 0..64 {
        let _ = tx.send(prompt());
        assert_that!(next(&closed, &mut rx).await.is_none()).is_true();
    }
}

/// t-815 : un onglet déjà fermé au démarrage de ses outils ne relaie à l'agent aucune consigne en file.
#[tokio::test]
async fn agent_run_with_tab_already_closed_sends_no_queued_prompt() {
    let dir = tempfile::Builder::new().prefix("oku-slides-tab-entry-").tempdir().unwrap();
    let data = dir.path().to_owned();
    // Le stockage est bloquant : hors du fil du runtime.
    let (platform, session) = tokio::task::spawn_blocking(move || {
        let mut settings = Settings::defaults().unwrap();
        settings.paths.decks = Some(data.join("decks"));
        settings.paths.data = Some(data);
        let platform = Platform::with_settings(settings).unwrap();
        let session = Session::new(Fonts::default(), &platform).unwrap();
        (platform, session)
    })
    .await
    .unwrap();
    let tab = TabSession::new("t-1", Arc::new(Mutex::new(session)));
    let hub = oku_slides_mcp::SurfaceHub::new(tab.clone(), platform.live_localizer.clone());
    let tools = Tools::new(tab.clone(), hub.clone(), Arc::new(AppSettings::new(&platform, None)), &platform);
    let mut mcp = hub.subscribe();
    // Un agent lancé laisserait cette trace : la consigne lui serait parvenue.
    let launched = dir.path().join("launched");
    let link = Link::new(Launch {
        command: "sh".into(),
        args: vec!["-c".into(), format!("touch '{}'", launched.display())],
        env: Vec::new(),
        fallback_cwd: dir.path().to_owned(),
        ready_within: Duration::from_secs(5),
    });
    let keeper = Keeper::spawn(tab.clone(), hub.clone());
    let localizer = platform.live_localizer.clone();
    let agent = Agent {
        session: tab,
        hub: hub.clone(),
        link,
        localizer: localizer.clone(),
        keeper: keeper.clone(),
        prompt_limits: PromptImageLimits { max_bytes: 0, max_side: 0, total_bytes: 0 },
    };
    let (out, mut sent) = mpsc::unbounded_channel();
    // Le relais reste ouvert : un tour ouvert par la consigne se verrait sur le hub.
    let relay = Relay {
        tab: "t-1".into(),
        hub: hub.clone(),
        keeper,
        localizer,
        turns: Turns::default(),
        status: Arc::new(Mutex::new((AgentState::Idle, None))),
        out,
        closed: CancellationToken::new(),
    };
    let cancel = CancellationToken::new();
    cancel.cancel();
    let (cmds, rx) = mpsc::unbounded_channel();
    cmds.send(prompt()).unwrap();
    agent.run(tools, cancel, rx, relay).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let surfaces = std::iter::from_fn(|| mcp.try_recv().ok()).filter(|e| matches!(e, McpEvent::Surface { .. })).count();
    assert_that!(surfaces).is_equal_to(0);
    assert_that!(sent.try_recv().is_err()).is_true();
    assert_that!(launched.exists()).is_false();
    // Le stockage tient un runtime bloquant : le lâcher hors du fil du runtime de test.
    tokio::task::spawn_blocking(move || drop((platform, hub))).await.unwrap();
}
