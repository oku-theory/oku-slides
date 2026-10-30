use std::sync::{Arc, Mutex};
use std::time::Duration;

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_mcp::McpEvent;
use oku_slides_platform::config::PromptImageLimits;
use oku_slides_session::agent::{AgentUi, Sessions, TabSession};
use oku_slides_session::{Fonts, Platform, Session, Settings, Workspace};
use serde::ser::{Error as _, Serialize, Serializer};
use speculoos::prelude::*;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{Agent, Relay, emit, next};
use crate::choice::AgentChoice;
use crate::keeper::Keeper;
use crate::settings_port::AppSettings;
use crate::tab_agent::{Command, Instruction};
use crate::turns::Turns;
use crate::voice_port::AppVoice;
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
    emit(&tx, "agent", "t-1", &crate::AgentEvent::Conversation);
    let (channel, json) = rx.try_recv().unwrap();
    assert_that!(channel).is_equal_to("agent");
    assert_that!(json.as_str()).is_equal_to(r#"{"tab":"t-1","event":{"kind":"conversation"}}"#);
}

fn prompt() -> Command {
    Command::Prompt(Instruction { text: "perdue".into(), ui: AgentUi::default(), tune: false, images: Vec::new() })
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
    // Un agent lancé laisserait cette trace : la consigne lui serait parvenue.
    let launched = dir.path().join("launched");
    let link = Link::new(Launch {
        command: "sh".into(),
        args: vec!["-c".into(), format!("touch '{}'", launched.display())],
        env: Vec::new(),
        fallback_cwd: dir.path().to_owned(),
        ready_within: Duration::from_secs(5),
    });
    let voice = AppVoice::new(None, Arc::new(AgentChoice::new(&platform, link.clone())), Handle::current());
    let hub = oku_slides_mcp::SurfaceHub::new(tab.clone(), platform.live_localizer.clone(), voice.clone());
    let settings = Arc::new(AppSettings::new(&platform, None));
    let mut mcp = hub.subscribe();
    let keeper = Keeper::spawn(tab.clone(), hub.clone());
    let localizer = platform.live_localizer.clone();
    let agent = Agent {
        session: tab,
        hub: hub.clone(),
        link,
        localizer: localizer.clone(),
        keeper: keeper.clone(),
        voice,
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
        status: Arc::new(Mutex::new(AgentState::Idle)),
        out,
        closed: CancellationToken::new(),
    };
    let cancel = CancellationToken::new();
    cancel.cancel();
    let (cmds, rx) = mpsc::unbounded_channel();
    cmds.send(prompt()).unwrap();
    agent.run(settings, platform.clone(), cancel, rx, relay).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let surfaces = std::iter::from_fn(|| mcp.try_recv().ok()).filter(|e| matches!(e, McpEvent::Surface { .. })).count();
    assert_that!(surfaces).is_equal_to(0);
    assert_that!(sent.try_recv().is_err()).is_true();
    assert_that!(launched.exists()).is_false();
    // Le stockage tient un runtime bloquant : le lâcher hors du fil du runtime de test.
    tokio::task::spawn_blocking(move || drop((platform, hub))).await.unwrap();
}

/// A platform in French and a tab `t-1` of a workspace that has no such tab (closed on the host side).
async fn closed_tab(data: std::path::PathBuf) -> (Platform, TabSession) {
    // Storage blocks: off the runtime's thread.
    tokio::task::spawn_blocking(move || {
        let mut settings = Settings::defaults().unwrap();
        settings.paths.decks = Some(data.join("decks"));
        settings.paths.data = Some(data);
        settings.i18n.locale = Some("fr".into());
        let platform = Platform::with_settings(settings).unwrap();
        let workspace: Arc<dyn Sessions> =
            Arc::new(Mutex::new(Workspace::new(Fonts::default(), &platform, None).unwrap()));
        (platform, TabSession::new("t-404", workspace))
    })
    .await
    .unwrap()
}

/// A tab gone before its agent starts has no kind: each prompt answers that, and no agent launches.
#[tokio::test]
async fn agent_run_on_a_closed_tab_answers_tools_unavailable() {
    let dir = tempfile::Builder::new().prefix("oku-slides-tab-entry-").tempdir().unwrap();
    let (platform, tab) = closed_tab(dir.path().to_owned()).await;
    let expected = {
        let tab = tab.clone();
        tokio::task::spawn_blocking(move || tab.document_kind().unwrap_err()).await.unwrap()
    };
    let launched = dir.path().join("launched");
    let link = Link::new(Launch {
        command: "sh".into(),
        args: vec!["-c".into(), format!("touch '{}'", launched.display())],
        env: Vec::new(),
        fallback_cwd: dir.path().to_owned(),
        ready_within: Duration::from_secs(5),
    });
    let voice = AppVoice::new(None, Arc::new(AgentChoice::new(&platform, link.clone())), Handle::current());
    let hub = oku_slides_mcp::SurfaceHub::new(tab.clone(), platform.live_localizer.clone(), voice.clone());
    let mut mcp = hub.subscribe();
    let keeper = Keeper::spawn(tab.clone(), hub.clone());
    let localizer = platform.live_localizer.clone();
    let agent = Agent {
        session: tab,
        hub: hub.clone(),
        link,
        localizer: localizer.clone(),
        keeper: keeper.clone(),
        voice,
        prompt_limits: PromptImageLimits { max_bytes: 0, max_side: 0, total_bytes: 0 },
    };
    let (out, _sent) = mpsc::unbounded_channel();
    let relay = Relay {
        tab: "t-404".into(),
        hub: hub.clone(),
        keeper,
        localizer,
        turns: Turns::default(),
        status: Arc::new(Mutex::new(AgentState::Idle)),
        out,
        closed: CancellationToken::new(),
    };
    let (cmds, rx) = mpsc::unbounded_channel();
    cmds.send(prompt()).unwrap();
    // The queue ends after the prompt: `run` returns once it has answered it.
    drop(cmds);
    let settings = Arc::new(AppSettings::new(&platform, None));
    tokio::time::timeout(
        Duration::from_secs(10),
        agent.run(settings, platform.clone(), CancellationToken::new(), rx, relay),
    )
    .await
    .unwrap();
    let shown: Vec<String> = std::iter::from_fn(|| mcp.try_recv().ok())
        .filter_map(|e| match e {
            McpEvent::Surface { surface } => Some(serde_json::to_string(&surface).unwrap()),
            McpEvent::SurfacePatch { messages, .. } => Some(serde_json::to_string(&messages).unwrap()),
            _ => None,
        })
        .collect();
    // The French message, as the surface's JSON writes it (quotes stripped).
    let fr = serde_json::to_string(&expected.localize(&Localizer::new(Some("fr")))).unwrap();
    let fr = &fr[1..fr.len() - 1];
    assert_that!(expected.kind()).is_equal_to(oku_slides_session::SessionErrorKind::UnknownTab);
    assert_that!(shown.iter().any(|s| s.contains(fr))).named("the error in the thread").is_true();
    assert_that!(launched.exists()).is_false();
    // Storage holds a blocking runtime: drop it off the test runtime's thread.
    tokio::task::spawn_blocking(move || drop((platform, hub))).await.unwrap();
}
