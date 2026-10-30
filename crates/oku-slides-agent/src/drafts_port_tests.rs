use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use oku_slides_mcp::{DraftPort, Prefill};
use oku_slides_session::agent::Sessions;
use oku_slides_session::{Fonts, Platform, Session, Settings, Workspace};
use speculoos::prelude::*;
use tokio::runtime::Handle;
use tokio::sync::mpsc;

use super::{Starter, StarterParts};
use crate::choice::AgentChoice;
use crate::settings_port::AppSettings;
use crate::tab_entry::Out;
use crate::voice_port::AppVoice;
use crate::{Launch, Link};

const REQUEST: &str = "Passe les titres en bleu.";

/// A starter over `sessions`, whose agent would leave `launched` if it ever started.
struct Fixture {
    starter: Arc<Starter>,
    rx: mpsc::UnboundedReceiver<Out>,
    launched: PathBuf,
    platform: Platform,
    _dir: tempfile::TempDir,
}

impl Fixture {
    /// The port the starter gives its tabs.
    fn drafts(&self) -> Arc<dyn DraftPort> {
        self.starter.ports.drafts.clone()
    }

    /// The draft events relayed so far, as JSON `{tab, event}`.
    fn drafts_sent(&mut self) -> Vec<serde_json::Value> {
        std::iter::from_fn(|| self.rx.try_recv().ok())
            .map(|(_, json)| serde_json::from_str::<serde_json::Value>(&json).unwrap())
            .filter(|v| v["event"]["kind"] == "draft")
            .collect()
    }

    /// Stops the started tabs, then drops the storage off the runtime's threads.
    async fn finish(self) {
        let Self { starter, platform, _dir, .. } = self;
        tokio::task::spawn_blocking(move || drop((starter, platform))).await.unwrap();
    }
}

/// A platform in French on `data`.
fn platform_on(data: PathBuf) -> Platform {
    let mut settings = Settings::defaults().unwrap();
    settings.paths.decks = Some(data.join("decks"));
    settings.paths.data = Some(data);
    settings.i18n.locale = Some("fr".into());
    Platform::with_settings(settings).unwrap()
}

/// A starter on one deck session (any tab id is that deck), or on an empty workspace (`known: false`).
async fn fixture(known: bool) -> Fixture {
    let dir = tempfile::Builder::new().prefix("oku-slides-drafts-port-").tempdir().unwrap();
    let data = dir.path().to_owned();
    let (platform, sessions) = tokio::task::spawn_blocking(move || {
        let platform = platform_on(data);
        let sessions: Arc<dyn Sessions> = if known {
            Arc::new(Mutex::new(Session::new(Fonts::default(), &platform).unwrap()))
        } else {
            Arc::new(Mutex::new(Workspace::new(Fonts::default(), &platform, None).unwrap()))
        };
        (platform, sessions)
    })
    .await
    .unwrap();
    let launched = dir.path().join("launched");
    let link = Link::new(Launch {
        command: "sh".into(),
        args: vec!["-c".into(), format!("touch '{}'", launched.display())],
        env: Vec::new(),
        fallback_cwd: dir.path().to_owned(),
        ready_within: Duration::from_secs(5),
    });
    let voice = AppVoice::new(None, Arc::new(AgentChoice::new(&platform, link.clone())), Handle::current());
    let (out_tx, rx) = mpsc::unbounded_channel();
    let starter = Starter::new(StarterParts {
        sessions,
        platform: platform.clone(),
        settings: Arc::new(AppSettings::new(&platform, None)),
        voice,
        link,
        rt: Handle::current(),
        out_tx,
        table: Arc::default(),
    });
    Fixture { starter, rx, launched, platform, _dir: dir }
}

/// `drafts.prefill`, off the runtime's threads as the tools call it.
async fn prefill(drafts: &Arc<dyn DraftPort>, tab: &str, text: &str) -> Prefill {
    let (drafts, tab, text) = (drafts.clone(), tab.to_owned(), text.to_owned());
    tokio::task::spawn_blocking(move || drafts.prefill(&tab, text)).await.unwrap()
}

fn started(f: &Fixture, tab: &str) -> bool {
    f.starter.table.lock().unwrap().tabs.contains_key(tab)
}

#[tokio::test(flavor = "multi_thread")]
async fn prefill_on_a_never_started_tab_starts_its_entry_and_sets_the_draft() {
    let f = fixture(true).await;
    assert_that!(prefill(&f.drafts(), "t-2", REQUEST).await).is_equal_to(Prefill::Set);
    assert_that!(started(&f, "t-2")).is_true();
    f.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn prefill_on_a_never_started_tab_never_launches_the_acp_agent() {
    let f = fixture(true).await;
    prefill(&f.drafts(), "t-2", REQUEST).await;
    // The tab's tools and keeper start at once; an agent launch would follow them.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_that!(f.launched.exists()).is_false();
    f.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn prefill_on_a_never_started_tab_starts_no_turn() {
    let f = fixture(true).await;
    prefill(&f.drafts(), "t-2", REQUEST).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let link = f.starter.open("t-2").unwrap();
    assert_that!(link.hub.turn()).is_none();
    f.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn prefill_emits_the_draft_to_the_tab_without_origin() {
    let mut f = fixture(true).await;
    prefill(&f.drafts(), "t-2", REQUEST).await;
    let expected = serde_json::json!({
        "tab": "t-2",
        "event": { "kind": "draft", "text": REQUEST, "origin": null, "images": [] },
    });
    assert_that!(f.drafts_sent()).is_equal_to(vec![expected]);
    f.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn prefill_of_a_tab_whose_draft_holds_text_returns_kept() {
    let f = fixture(true).await;
    prefill(&f.drafts(), "t-2", "à moi").await;
    assert_that!(prefill(&f.drafts(), "t-2", REQUEST).await).is_equal_to(Prefill::Kept);
    f.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn prefill_then_replay_as_follow_does_replays_the_draft() {
    let mut f = fixture(true).await;
    prefill(&f.drafts(), "t-2", REQUEST).await;
    f.drafts_sent();
    // `Follow` replays a started tab's link.
    f.starter.open("t-2").unwrap().replay(&f.starter.out_tx);
    let replayed = f.drafts_sent();
    assert_that!(replayed.len()).is_equal_to(1);
    assert_that!(replayed[0]["event"]["text"].clone()).is_equal_to(serde_json::json!(REQUEST));
    f.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn prefill_on_an_unknown_tab_returns_unavailable_and_starts_nothing() {
    let f = fixture(false).await;
    assert_that!(prefill(&f.drafts(), "t-404", REQUEST).await).is_equal_to(Prefill::Unavailable);
    assert_that!(started(&f, "t-404")).is_false();
    f.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn prefill_on_a_closed_tab_returns_unavailable() {
    let f = fixture(true).await;
    f.starter.table.lock().unwrap().closed.insert("t-2".to_owned());
    assert_that!(prefill(&f.drafts(), "t-2", REQUEST).await).is_equal_to(Prefill::Unavailable);
    f.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn prefill_once_the_stack_is_gone_returns_unavailable() {
    let f = fixture(true).await;
    let drafts = f.drafts();
    f.finish().await;
    assert_that!(prefill(&drafts, "t-2", REQUEST).await).is_equal_to(Prefill::Unavailable);
}
