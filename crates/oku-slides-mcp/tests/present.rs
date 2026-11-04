//! The `present` tool: start the presentation, then steer it while the tab's page presents.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::sync::PoisonError;

use common::*;
use oku_slides_i18n::Localizer;
use oku_slides_mcp::{InterfaceChange, McpEvent, PresentCommand, PresentMode};
use oku_slides_session::Request;
use rstest::rstest;
use rust_mcp_sdk::schema::CallToolResult;
use serde_json::{Value, json};
use speculoos::prelude::*;
use tokio::sync::broadcast::Receiver;

/// Calls `tool`, no confirmation expected.
async fn direct(h: &Harness, tool: &str, args: Value) -> CallToolResult {
    call_within(&server(h).await, tool, args).await
}

/// `req` on the session, off the async thread: `Storage` (OpenDAL) blocks.
fn handle(h: &Harness, req: Request) {
    let session = || h.session.lock().unwrap_or_else(PoisonError::into_inner);
    tokio::task::block_in_place(|| session().handle(req)).unwrap_or_else(|e| panic!("test request: {e}"));
}

/// The slide ids of the test deck, in order.
fn slide_ids(h: &Harness) -> Vec<String> {
    h.session.lock().unwrap_or_else(PoisonError::into_inner).snapshot().slides.iter().map(|s| s.id.to_owned()).collect()
}

/// The command of the hub's next present event.
async fn next_present(rx: &mut Receiver<McpEvent>) -> PresentCommand {
    wait_event(rx, |e| match e {
        McpEvent::Present { command } => Some(command),
        _ => None,
    })
    .await
}

/// No present event waits in `rx`.
fn no_present_event(rx: &mut Receiver<McpEvent>) -> bool {
    std::iter::from_fn(|| rx.try_recv().ok()).all(|e| !matches!(e, McpEvent::Present { .. }))
}

/// A harness whose tab's page reports it presents in the window.
async fn presenting() -> Harness {
    let h = start().await;
    h.hub.set_presenting(Some(PresentMode::Window), 1);
    h
}

// --- start ----------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn present_emits_present_event_with_its_mode() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    direct(&h, "present", json!({ "action": "start", "mode": "window" })).await;
    assert_that!(next_present(&mut rx).await).is_equal_to(PresentCommand::Start { mode: PresentMode::Window });
}

/// The e2e fixture's `{ "mode": "window" }` stays valid: no action is a start.
#[rstest]
#[case::default_mode(json!({}), PresentMode::Screen)]
#[case::window(json!({ "mode": "window" }), PresentMode::Window)]
#[tokio::test(flavor = "multi_thread")]
async fn present_without_action_starts(#[case] args: Value, #[case] mode: PresentMode) {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    direct(&h, "present", args).await;
    assert_that!(next_present(&mut rx).await).is_equal_to(PresentCommand::Start { mode });
}

/// A mode where the page does not present.
#[derive(Debug, Clone, Copy)]
enum NoPresent {
    History,
    Help,
    NoSlide,
}

/// A harness whose session is in `mode`; help is a help tab (read-only).
async fn start_in(mode: NoPresent) -> Harness {
    if let NoPresent::Help = mode {
        return common::kinds::start_help_tab().await;
    }
    let h = start().await;
    match mode {
        NoPresent::History => handle(&h, Request::HistoryOpen),
        NoPresent::Help => {}
        NoPresent::NoSlide => {
            let ops: Vec<Value> = slide_ids(&h).iter().map(|id| json!({ "op": "remove_slide", "slide": id })).collect();
            let req = json!({ "cmd": "apply", "label": "Vider", "ops": ops });
            handle(&h, serde_json::from_value(req).unwrap_or_else(|e| panic!("test request: {e}")));
        }
    }
    h
}

#[rstest]
#[case::history(NoPresent::History, "session-present-history")]
#[case::help(NoPresent::Help, "session-present-help")]
#[case::no_slide(NoPresent::NoSlide, "session-present-no-slide")]
#[tokio::test(flavor = "multi_thread")]
async fn present_where_the_page_cannot_present_is_refused_with_its_reason(#[case] mode: NoPresent, #[case] key: &str) {
    let h = start_in(mode).await;
    let mut rx = h.hub.subscribe();
    let r = direct(&h, "present", json!({})).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr(key));
    assert_that!(no_present_event(&mut rx)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn present_on_a_hidden_tab_is_refused_in_the_user_language() {
    let h = start_hidden().await;
    let r = direct(&h, "present", json!({})).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-not-shown"));
}

// --- steering -------------------------------------------------------------

#[rstest]
#[case::next("next", PresentCommand::Next)]
#[case::previous("previous", PresentCommand::Previous)]
#[case::first("first", PresentCommand::First)]
#[case::last("last", PresentCommand::Last)]
#[case::exit("exit", PresentCommand::Exit)]
#[tokio::test(flavor = "multi_thread")]
async fn present_step_while_presenting_emits_its_command(#[case] action: &str, #[case] command: PresentCommand) {
    let h = presenting().await;
    let mut rx = h.hub.subscribe();
    let r = direct(&h, "present", json!({ "action": action })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(next_present(&mut rx).await).is_equal_to(command);
}

#[tokio::test(flavor = "multi_thread")]
async fn present_show_known_slide_emits_show() {
    let h = presenting().await;
    let slide = slide_ids(&h).last().cloned().unwrap_or_else(|| panic!("the test deck has slides"));
    let mut rx = h.hub.subscribe();
    direct(&h, "present", json!({ "action": "show", "slide": slide })).await;
    assert_that!(next_present(&mut rx).await).is_equal_to(PresentCommand::Show { slide });
}

#[rstest]
#[case::unknown(json!({ "action": "show", "slide": "s-nowhere" }))]
#[case::missing(json!({ "action": "show" }))]
#[tokio::test(flavor = "multi_thread")]
async fn present_show_without_a_deck_slide_is_refused(#[case] args: Value) {
    let h = presenting().await;
    let mut rx = h.hub.subscribe();
    let r = direct(&h, "present", args).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-present-slide"));
    assert_that!(no_present_event(&mut rx)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn present_next_not_presenting_refused_localized() {
    let h = start().await;
    h.live.set(std::sync::Arc::new(Localizer::new(Some("fr"))));
    let mut rx = h.hub.subscribe();
    let r = direct(&h, "present", json!({ "action": "next" })).await;
    let fr = Localizer::new(Some("fr"));
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(fr.tr("agent-error-not-presenting"));
    assert_that!(text_of(&r)).is_not_equal_to(Localizer::new(Some("en")).tr("agent-error-not-presenting"));
    assert_that!(no_present_event(&mut rx)).is_true();
}

/// The page's end report ends the steering: a step after it is refused.
#[tokio::test(flavor = "multi_thread")]
async fn present_step_after_end_report_is_refused() {
    let h = presenting().await;
    h.hub.set_presenting(None, 2);
    let r = direct(&h, "present", json!({ "action": "exit" })).await;
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-not-presenting"));
}

#[tokio::test(flavor = "multi_thread")]
async fn present_unknown_argument_is_refused() {
    let h = presenting().await;
    let r = direct(&h, "present", json!({ "action": "next", "speed": 2 })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
}

// --- contract with the interface ------------------------------------------

/// Review t-988: the JSON shape of the page events, as `ui/src/api.ts` reads them.
#[test]
fn mcp_event_page_events_serialize_as_the_ui_reads_them() {
    let change = InterfaceChange { guides: Some(true), ..InterfaceChange::default() };
    let events = [
        McpEvent::Present { command: PresentCommand::Start { mode: PresentMode::Window } },
        McpEvent::Present { command: PresentCommand::Show { slide: "s-2".into() } },
        McpEvent::Present { command: PresentCommand::Next },
        McpEvent::Interface { change },
        McpEvent::OpenHistory,
    ];
    let json: Vec<Value> = events.iter().map(|e| serde_json::to_value(e).unwrap_or(Value::Null)).collect();
    assert_that!(json).is_equal_to(vec![
        json!({ "kind": "present", "command": { "action": "start", "mode": "window" } }),
        json!({ "kind": "present", "command": { "action": "show", "slide": "s-2" } }),
        json!({ "kind": "present", "command": { "action": "next" } }),
        json!({ "kind": "interface", "change": {
            "scheme": null, "locale": null, "guides": true, "folded": null, "pile": null, "notes": null, "code": null,
        } }),
        json!({ "kind": "open_history" }),
    ]);
}
