//! `import`: a `.pptx` in a new tab of the agent's window, confirmed, one tab move per turn.
#![expect(clippy::panic, reason = "test failures panic")]

mod common;

use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use common::refusals::{CONTINUE_FR, LIMIT_FR, NO_TURN_FR, assert_refused};
use common::workspace_harness::{start_on_agent_workspace, tab_ids};
use common::*;
use oku_slides_core::Namespace;
use oku_slides_mcp::SurfaceHub;
use oku_slides_session::agent::{AgentWorkspace, FocusedTab, TabGate, TabOpen, WorkspaceRefusal};
use oku_slides_session::import::ImportTarget;
use oku_slides_session::{ESSENTIEL_PACK, PAPIER_NAMESPACE, Workspace};
use rust_mcp_sdk::mcp_client::ClientRuntime;
use rust_mcp_sdk::schema::CallToolResult;
use serde_json::json;
use speculoos::prelude::*;

/// The `.pptx` of fixture `minimal` in the harness's data dir; its path as the agent gives it.
fn minimal(h: &Harness) -> String {
    pptx(h.data(), "minimal").to_string_lossy().into_owned()
}

/// The tab a successful import names.
fn tab_of(r: &CallToolResult) -> String {
    json_of(r)["tab"].as_str().unwrap_or_else(|| panic!("a tab: {r:?}")).to_owned()
}

/// Reads tab `tab`'s session through `f`, off the async thread.
fn read_tab<T: Send + 'static>(
    ws: &Arc<Mutex<Workspace>>,
    tab: &str,
    f: impl FnOnce(&mut oku_slides_session::Session) -> T + Send + 'static,
) -> T {
    tokio::task::block_in_place(|| Workspace::with_session(ws, tab, f).0.unwrap_or_else(|e| panic!("tab {tab}: {e}")))
}

/// `r` is an error whose text holds `fr`, the French wording, and differs from English.
fn assert_said_in_french(r: &CallToolResult, fr: &str, en: &str) {
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(fr).is_not_equal_to(en);
    assert_that!(text_of(r).as_str()).contains(fr);
}

#[tokio::test(flavor = "multi_thread")]
async fn import_hifi_opens_a_new_tab() {
    let (h, ws) = start_on_agent_workspace().await;
    let path = minimal(&h);
    h.hub.begin_turn();
    let (message, danger, r) = guarded_message(&h, "import", json!({ "path": path, "mode": "hifi" }), true).await;
    assert_that!(message).is_equal_to(h.localizer.tr_with("agent-confirm-import", &[("path", path.into())]));
    assert_that!(danger).is_true();
    let tab = tab_of(&r);
    assert_that!(tab_ids(&ws)).is_equal_to(vec!["t-1".to_owned(), tab.clone()]);
    // A hi-fi import draws on a pack generated for the presentation, the deck's own.
    let own_pack = read_tab(&ws, &tab, |s| s.deck().layouts.contains_key(&Namespace::deck_layouts()));
    assert_that!(own_pack).is_true();
    let args = [("tab", tab.as_str().into()), ("shown", "yes".into()), ("tune", "no".into())];
    assert_that!(json_of(&r)["note"].as_str().map(str::to_owned))
        .is_equal_to(Some(h.localizer.tr_with("agent-import-opened-in-tab", &args)));
}

#[tokio::test(flavor = "multi_thread")]
async fn import_relative_path_refused() {
    let (h, ws) = start_on_agent_workspace().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = call_within(&c, "import", json!({ "path": "deck.pptx", "mode": "hifi" })).await;
    assert_refused(&h, &r, "agent-error-path-relative", "Le chemin doit être absolu.");
    assert_that!(tab_ids(&ws)).is_equal_to(vec!["t-1".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn import_of_another_extension_is_refused_without_asking() {
    let (h, _ws) = start_on_agent_workspace().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = call_within(&c, "import", json!({ "path": "/tmp/deck.key", "mode": "hifi" })).await;
    let en = oku_slides_i18n::Localizer::new(Some("en")).tr("agent-error-import-extension");
    assert_said_in_french(&r, &h.localizer.tr("agent-error-import-extension"), &en);
}

#[tokio::test(flavor = "multi_thread")]
async fn import_hifi_with_a_theme_is_refused_without_asking() {
    let (h, _ws) = start_on_agent_workspace().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = call_within(&c, "import", json!({ "path": "/tmp/deck.pptx", "mode": "hifi", "theme": "papier" })).await;
    let en = oku_slides_i18n::Localizer::new(Some("en")).tr("agent-error-import-hifi-target");
    assert_said_in_french(&r, &h.localizer.tr("agent-error-import-hifi-target"), &en);
}

#[tokio::test(flavor = "multi_thread")]
async fn import_outside_a_turn_is_refused_without_asking() {
    let (h, ws) = start_on_agent_workspace().await;
    let c = server(&h).await;
    let r = call_within(&c, "import", json!({ "path": minimal(&h), "mode": "hifi" })).await;
    assert_refused(&h, &r, "agent-error-tab-no-turn", NO_TURN_FR);
    assert_that!(tab_ids(&ws)).is_equal_to(vec!["t-1".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn import_second_tab_move_in_turn_refused() {
    let (h, ws) = start_on_agent_workspace().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let theme = call(&c, "open_theme", json!({ "theme": "papier" })).await;
    assert_that!(theme.is_error).is_not_equal_to(Some(true));
    let r = call_within(&c, "import", json!({ "path": minimal(&h), "mode": "hifi" })).await;
    assert_refused(&h, &r, "agent-error-tab-limit", LIMIT_FR);
    assert_that!(tab_ids(&ws).len()).is_equal_to(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn import_denied_gives_the_turn_move_back() {
    let (h, ws) = start_on_agent_workspace().await;
    let path = minimal(&h);
    h.hub.begin_turn();
    let r = guarded(&h, "import", json!({ "path": path, "mode": "hifi" }), false).await;
    assert_that!(text_of(&r).as_str()).is_equal_to("L'utilisateur a refusé.");
    let c = server(&h).await;
    let theme = call(&c, "open_theme", json!({ "theme": "papier" })).await;
    assert_that!(theme.is_error).is_not_equal_to(Some(true));
    assert_that!(tab_ids(&ws).len()).is_equal_to(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn import_tune_starts_tune_through_port() {
    let (h, _ws) = start_on_agent_workspace().await;
    let path = minimal(&h);
    h.hub.begin_turn();
    let r = guarded(&h, "import", json!({ "path": path, "mode": "hifi", "tune": true }), true).await;
    let tab = tab_of(&r);
    assert_that!(h.tabs.tunes()).is_equal_to(vec![tab]);
    assert_that!(json_of(&r)["tune"].as_bool()).is_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn import_retrofit_unknown_pack_refused_localized() {
    let (h, ws) = start_on_agent_workspace().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let args = json!({ "path": minimal(&h), "mode": "retrofit", "pack": ["nulle-part"] });
    let r = call_within(&c, "import", args).await;
    let wording = [("pack", "nulle-part".into())];
    let en = oku_slides_i18n::Localizer::new(Some("en")).tr_with("session-import-unknown-pack", &wording);
    assert_said_in_french(&r, &h.localizer.tr_with("session-import-unknown-pack", &wording), &en);
    assert_that!(tab_ids(&ws)).is_equal_to(vec!["t-1".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn import_retrofit_with_pack_uses_its_layouts() {
    let (h, ws) = start_on_agent_workspace().await;
    let path = minimal(&h);
    h.hub.begin_turn();
    let args = json!({ "path": path, "mode": "retrofit", "pack": [ESSENTIEL_PACK.to_string(), "maison"] });
    let tab = tab_of(&guarded(&h, "import", args, true).await);
    let layouts = read_tab(&ws, &tab, |s| s.deck().layouts.keys().map(|ns| ns.as_str().to_owned()).collect::<Vec<_>>());
    assert_that!(layouts).contains("maison".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn import_with_theme_applies_it() {
    let (h, ws) = start_on_agent_workspace().await;
    let path = minimal(&h);
    h.hub.begin_turn();
    let args = json!({ "path": path, "mode": "retrofit", "theme": "papier" });
    let tab = tab_of(&guarded(&h, "import", args, true).await);
    let theme = read_tab(&ws, &tab, |s| s.deck().theme.clone());
    assert_that!(theme.as_ref().map(Namespace::as_str)).is_equal_to(Some(PAPIER_NAMESPACE));
}

#[tokio::test(flavor = "multi_thread")]
async fn import_unknown_theme_refused_localized() {
    let (h, ws) = start_on_agent_workspace().await;
    let c = server(&h).await;
    h.hub.begin_turn();
    let r = call_within(&c, "import", json!({ "path": minimal(&h), "mode": "retrofit", "theme": "aucun" })).await;
    let wording = [("theme", "aucun".into())];
    let en = oku_slides_i18n::Localizer::new(Some("en")).tr_with("session-unknown-theme", &wording);
    assert_said_in_french(&r, &h.localizer.tr_with("session-unknown-theme", &wording), &en);
    assert_that!(tab_ids(&ws)).is_equal_to(vec!["t-1".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn import_on_a_host_without_tab_bar_is_refused_localized() {
    let h = start().await;
    h.hub.begin_turn();
    let r = guarded(&h, "import", json!({ "path": "/tmp/deck.pptx", "mode": "hifi" }), true).await;
    let en = oku_slides_i18n::Localizer::new(Some("en")).tr("agent-error-workspace-unavailable");
    assert_said_in_french(&r, &h.localizer.tr("agent-error-workspace-unavailable"), &en);
}

/// A workspace that checks, under its own lock, what the tool did before and asks after.
#[derive(Default)]
struct LockOrderWorkspace {
    /// The workspace's own lock, held for the whole import.
    lock: Mutex<()>,
    /// The hub and a client of the tools, set once the server runs.
    probe: OnceLock<(Arc<SurfaceHub>, Arc<ClientRuntime>)>,
    /// What a widget write said under the lock, and the gate's answer once the turn moved on.
    seen: Mutex<Option<(String, bool)>>,
}

impl std::fmt::Debug for LockOrderWorkspace {
    /// `ClientRuntime` does not implement `Debug`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LockOrderWorkspace").finish_non_exhaustive()
    }
}

impl AgentWorkspace for LockOrderWorkspace {
    fn focused_tab(&self) -> Option<FocusedTab> {
        None
    }

    fn unsaved_tabs(&self) -> usize {
        0
    }

    fn import_tab(
        &self,
        _: &str,
        _: &Path,
        _: ImportTarget,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> Result<TabOpen, WorkspaceRefusal> {
        let _guard = lock(&self.lock);
        let (hub, client) = self.probe.get().unwrap_or_else(|| panic!("probe set before the call"));
        // The claim is already held: this tab leaves its widgets alone.
        let widget = tokio::runtime::Handle::current().block_on(call(
            client,
            "set_widget",
            json!({ "name": "badge", "source": "#let x = 1" }),
        ));
        // A new turn: a gate that asked the hub now would refuse.
        hub.begin_turn();
        let granted = gate(TabGate::NewTab);
        *lock(&self.seen) = Some((text_of(&widget), granted));
        Ok(TabOpen::Opened { tab: "t-9".into(), shown: true })
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn reserve_tab_before_workspace_lock() {
    let fake = Arc::new(LockOrderWorkspace::default());
    let h = start_with_port(fake.clone()).await;
    let probe = (h.hub.clone(), server(&h).await);
    fake.probe.set(probe).unwrap_or_else(|_| panic!("probe set once"));
    h.hub.begin_turn();
    let r = guarded(&h, "import", json!({ "path": "/tmp/deck.pptx", "mode": "hifi" }), true).await;
    assert_that!(tab_of(&r)).is_equal_to("t-9".to_owned());
    let seen = lock(&fake.seen).clone();
    assert_that!(seen).is_equal_to(Some((CONTINUE_FR.to_owned(), true)));
}

/// A workspace whose import task is lost: it panics, maybe after the tab went in.
#[derive(Debug, Default)]
struct PanickingWorkspace;

impl AgentWorkspace for PanickingWorkspace {
    fn focused_tab(&self) -> Option<FocusedTab> {
        None
    }

    fn unsaved_tabs(&self) -> usize {
        0
    }

    fn import_tab(
        &self,
        _: &str,
        _: &Path,
        _: ImportTarget,
        _: &mut dyn FnMut(TabGate) -> bool,
    ) -> Result<TabOpen, WorkspaceRefusal> {
        panic!("import task lost on purpose")
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn import_lost_task_keeps_the_turn_move_spent() {
    let h = start_with_port(Arc::new(PanickingWorkspace)).await;
    h.hub.begin_turn();
    let lost = guarded(&h, "import", json!({ "path": "/tmp/deck.pptx", "mode": "hifi" }), true).await;
    assert_that!(lost.is_error).is_equal_to(Some(true));
    let c = server(&h).await;
    let again = call_within(&c, "import", json!({ "path": "/tmp/other.pptx", "mode": "hifi" })).await;
    assert_refused(&h, &again, "agent-error-tab-limit", LIMIT_FR);
}
