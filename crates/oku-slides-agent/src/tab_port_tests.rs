use std::sync::{Arc, Mutex, Weak};

use oku_slides_mcp::{Prefill, TabPort};
use oku_slides_platform::config::AgentPreset;
use oku_slides_session::agent::{NoWorkspace, Sessions, UnavailableApp};
use oku_slides_session::{Fonts, Platform, Session, Settings};
use speculoos::prelude::*;
use tokio::runtime::Handle;

use super::StackTabs;
use crate::Stack;

#[test]
fn stack_tabs_tune_without_a_stack_is_unavailable() {
    let port = StackTabs { stack: Weak::new() };
    assert_that!(port.tune("t-2").err().map(|e| e.key())).is_equal_to(Some("agent-error-tune-unavailable"));
}

#[test]
fn stack_tabs_prefill_without_a_stack_is_unavailable() {
    let port = StackTabs { stack: Weak::new() };
    assert_that!(port.prefill("t-2", "x".to_owned())).is_equal_to(Prefill::Unavailable);
}

#[test]
fn stack_tabs_page_without_a_stack_is_false() {
    let port = StackTabs { stack: Weak::new() };
    assert_that!(port.page("t-2", "s-1".to_owned())).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn stack_tabs_tune_of_an_unusable_agent_is_refused() {
    let dir = tempfile::Builder::new().prefix("oku-slides-stack-tune-").tempdir().unwrap();
    let data = dir.path().to_owned();
    let (platform, sessions) = tokio::task::spawn_blocking(move || {
        let mut settings = Settings::defaults().unwrap();
        settings.paths.decks = Some(data.join("decks"));
        settings.paths.data = Some(data);
        let absent = AgentPreset::new("Absent", "/nonexistent/oku-slides-agent-absent", Vec::new());
        settings.agent.insert_preset("absent", absent).unwrap();
        settings.agent.select("absent").unwrap();
        let platform = Platform::with_settings(settings).unwrap();
        let sessions: Arc<dyn Sessions> = Arc::new(Mutex::new(Session::new(Fonts::default(), &platform).unwrap()));
        (platform, sessions)
    })
    .await
    .unwrap();
    let stack = Stack::start(
        sessions,
        &platform,
        crate::Voices::default(),
        Arc::new(NoWorkspace),
        Arc::new(UnavailableApp),
        Handle::current(),
    );
    let port = StackTabs { stack: Arc::downgrade(&stack) };
    let refused = tokio::task::block_in_place(|| port.tune("t-1"));
    assert_that!(refused.err().map(|e| e.key())).is_equal_to(Some("agent-error-tune-not-started"));
    stack.shutdown().await;
    tokio::task::spawn_blocking(move || drop((stack, platform))).await.unwrap();
}
