use std::sync::{Arc, Mutex};
use std::time::Duration;

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_mcp::McpEvent;
use oku_slides_platform::config::PromptImageLimits;
use oku_slides_session::agent::{AgentUi, AttachmentRef, Sessions, TabSession};
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
        drafts: Arc::new(oku_slides_mcp::NoDrafts),
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

/// A platform in French and a tab `t-404` of a workspace that has no such tab (closed on the host side).
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
async fn agent_run_on_a_closed_tab_answers_unknown_tab() {
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
        drafts: Arc::new(oku_slides_mcp::NoDrafts),
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

// --- the host's prefill of a draft (t-1277) ----------------------------------------------------

/// A tab link of a closed tab `t-404`, with `draft` as its draft, and its data dir.
async fn link_with(draft: super::Draft) -> (super::TabLink, Platform, tempfile::TempDir) {
    let dir = tempfile::Builder::new().prefix("oku-slides-tab-entry-").tempdir().unwrap();
    let (platform, tab) = closed_tab(dir.path().to_owned()).await;
    let link = Link::new(Launch {
        command: "true".into(),
        args: Vec::new(),
        env: Vec::new(),
        fallback_cwd: dir.path().to_owned(),
        ready_within: Duration::from_secs(5),
    });
    let voice = AppVoice::new(None, Arc::new(AgentChoice::new(&platform, link)), Handle::current());
    let hub = oku_slides_mcp::SurfaceHub::new(tab.clone(), platform.live_localizer.clone(), voice);
    let keeper = Keeper::spawn(tab.clone(), hub.clone());
    let (cmds, _rx) = mpsc::unbounded_channel();
    let link = super::TabLink {
        tab: "t-404".into(),
        hub,
        keeper,
        cmds,
        status: Arc::new(Mutex::new(AgentState::Idle)),
        draft: Arc::new(Mutex::new(draft)),
        session: tab,
        paste: Arc::default(),
    };
    (link, platform, dir)
}

fn text_draft(text: Option<&str>) -> super::Draft {
    super::Draft { text: text.map(str::to_owned), images: Vec::new() }
}

/// The draft events sent on `rx` so far, as JSON.
fn drafts_sent(rx: &mut mpsc::UnboundedReceiver<super::Out>) -> Vec<serde_json::Value> {
    std::iter::from_fn(|| rx.try_recv().ok())
        .map(|(_, json)| serde_json::from_str::<serde_json::Value>(&json).unwrap()["event"].clone())
        .filter(|e| e["kind"] == "draft")
        .collect()
}

/// Drops what holds the storage's blocking runtime off the test runtime's thread.
async fn drop_off_runtime(link: super::TabLink, platform: Platform) {
    tokio::task::spawn_blocking(move || drop((link, platform))).await.unwrap();
}

#[rstest::rstest]
#[case::closed(None)]
#[case::empty(Some(""))]
#[case::blank(Some(" \n "))]
#[tokio::test(flavor = "multi_thread")]
async fn prefill_draft_on_an_empty_draft_sets_it_and_emits_it_without_origin(#[case] text: Option<&str>) {
    let (link, platform, _dir) = link_with(text_draft(text)).await;
    let (out, mut rx) = mpsc::unbounded_channel();
    assert_that!(link.prefill_draft(&out, "Passe les titres en bleu.".into())).is_true();
    let expected =
        serde_json::json!({ "kind": "draft", "text": "Passe les titres en bleu.", "origin": null, "images": [] });
    assert_that!(drafts_sent(&mut rx)).is_equal_to(vec![expected]);
    drop_off_runtime(link, platform).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn prefill_draft_on_a_non_blank_draft_keeps_it_and_emits_nothing() {
    let (link, platform, _dir) = link_with(text_draft(Some("à moi"))).await;
    let (out, mut rx) = mpsc::unbounded_channel();
    assert_that!(link.prefill_draft(&out, "écrasé ?".into())).is_false();
    assert_that!(drafts_sent(&mut rx)).is_empty();
    assert_that!(link.draft.lock().unwrap().text.clone()).is_equal_to(Some("à moi".to_owned()));
    drop_off_runtime(link, platform).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn prefill_draft_on_a_draft_holding_only_images_keeps_it() {
    let image = AttachmentRef { id: "sha256:00".into(), mime: "image/png".into(), width: 1, height: 1 };
    let (link, platform, _dir) = link_with(super::Draft { text: Some(String::new()), images: vec![image] }).await;
    let (out, mut rx) = mpsc::unbounded_channel();
    assert_that!(link.prefill_draft(&out, "écrasé ?".into())).is_false();
    assert_that!(drafts_sent(&mut rx)).is_empty();
    drop_off_runtime(link, platform).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn prefill_draft_twice_at_once_leaves_exactly_one_winner() {
    let (link, platform, _dir) = link_with(text_draft(None)).await;
    let (out, mut rx) = mpsc::unbounded_channel();
    let won = tokio::task::block_in_place(|| {
        std::thread::scope(|s| {
            let a = s.spawn(|| link.prefill_draft(&out, "a".into()));
            let b = s.spawn(|| link.prefill_draft(&out, "b".into()));
            [a.join().unwrap(), b.join().unwrap()]
        })
    });
    assert_that!(won.iter().filter(|w| **w).count()).is_equal_to(1);
    assert_that!(drafts_sent(&mut rx).len()).is_equal_to(1);
    drop_off_runtime(link, platform).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_after_a_prefill_wins() {
    let (link, platform, _dir) = link_with(text_draft(None)).await;
    let (out, mut rx) = mpsc::unbounded_channel();
    link.prefill_draft(&out, "de l'agent".into());
    link.set_draft(&out, Some("de l'utilisateur".into()), Some("v-1".into()));
    link.replay(&out);
    let last = drafts_sent(&mut rx).pop().unwrap();
    assert_that!(last["text"].clone()).is_equal_to(serde_json::json!("de l'utilisateur"));
    drop_off_runtime(link, platform).await;
}
