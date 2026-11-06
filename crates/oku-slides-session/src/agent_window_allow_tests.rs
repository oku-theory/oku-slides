//! The detached agent window's allowlist: what it may ask, and what is refused.

use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{AGENT_WINDOW_LABEL, check_agent_request, check_agent_workspace};
use crate::test_support::{fresh_dir, workspace_on};
use crate::{Request, SessionErrorKind, Workspace, WorkspaceRequest};

/// A workspace request parsed as a page would send it.
fn ws_req(v: Value) -> WorkspaceRequest {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("workspace request: {e}"))
}

/// A tab request parsed as a page would send it.
fn tab_req(v: Value) -> Request {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("tab request: {e}"))
}

/// A fresh workspace and the id of its first window's active tab.
fn workspace_with_tab() -> (tempfile::TempDir, std::sync::Mutex<Workspace>, String) {
    let data = fresh_dir("oku-slides-agent-window-allow-");
    let ws = workspace_on(data.path());
    let tab = Workspace::lock(&ws).active_tab("w-1").map(str::to_owned);
    let tab = tab.unwrap_or_else(|| panic!("no active tab in w-1"));
    (data, ws, tab)
}

/// An element as the clipboard keeps it.
fn element() -> Value {
    json!({ "id": "e-1", "kind": "base/text" })
}

const PACK: &str = "00000000-0000-4000-8000-000000000001";

#[test]
fn agent_window_label_is_the_agent_windows() {
    assert_that!(AGENT_WINDOW_LABEL).is_equal_to("agent");
}

#[test]
fn agent_window_label_is_not_a_workspace_window() {
    assert_that!(crate::is_window_label(AGENT_WINDOW_LABEL)).is_false();
}

#[rstest]
#[case::view(json!({ "cmd": "view" }))]
#[case::messages(json!({ "cmd": "messages" }))]
#[case::scheme(json!({ "cmd": "scheme" }))]
#[case::attach_agent(json!({ "cmd": "attach_agent" }))]
fn check_agent_workspace_allowed_request_passes(#[case] req: Value) {
    let (_data, ws, _tab) = workspace_with_tab();
    let ws = Workspace::lock(&ws);
    assert_that!(check_agent_workspace(&ws, &ws_req(req))).is_ok();
}

#[rstest]
#[case::open_theme("open_theme", json!({ "theme": PACK }))]
#[case::open_pack("open_pack", json!({ "pack": PACK }))]
#[case::open_pack_with_widget("open_pack", json!({ "pack": PACK, "widget": "card" }))]
fn check_agent_workspace_open_on_a_held_tab_passes(#[case] cmd: &str, #[case] rest: Value) {
    let (_data, ws, tab) = workspace_with_tab();
    let mut req = rest;
    req["cmd"] = json!(cmd);
    req["tab"] = json!(tab);
    let ws = Workspace::lock(&ws);
    assert_that!(check_agent_workspace(&ws, &ws_req(req))).is_ok();
}

#[rstest]
#[case::open_theme_unknown_tab(json!({ "cmd": "open_theme", "tab": "t-999", "theme": PACK }))]
#[case::open_pack_unknown_tab(json!({ "cmd": "open_pack", "tab": "t-999", "pack": PACK }))]
#[case::open_theme_window_label(json!({ "cmd": "open_theme", "tab": "w-1", "theme": PACK }))]
#[case::open_pack_agent_label(json!({ "cmd": "open_pack", "tab": "agent", "pack": PACK }))]
fn check_agent_workspace_open_on_a_tab_no_window_holds_is_forbidden(#[case] req: Value) {
    let (_data, ws, _tab) = workspace_with_tab();
    let ws = Workspace::lock(&ws);
    let refused = check_agent_workspace(&ws, &ws_req(req)).map_err(|e| e.kind());
    assert_that!(refused).is_err_containing(SessionErrorKind::AgentWindowForbidden);
}

#[rstest]
#[case::set_locale(json!({ "cmd": "set_locale", "locale": "fr" }))]
#[case::set_scheme(json!({ "cmd": "set_scheme", "scheme": "dark" }))]
#[case::new_tab(json!({ "cmd": "new_tab", "window": "w-1" }))]
#[case::open_tab(json!({ "cmd": "open_tab", "window": "w-1", "path": "/x.okuslides.pdf" }))]
#[case::import_tab(json!({ "cmd": "import_tab", "window": "w-1", "path": "/x.pptx", "mode": "hifi" }))]
#[case::open_help(json!({ "cmd": "open_help", "window": "w-1" }))]
#[case::same_deck(json!({ "cmd": "same_deck", "path": "/x.okuslides.pdf" }))]
#[case::carry_assets(json!({ "cmd": "carry_assets", "from": "t-1", "to": "t-1", "element": element() }))]
#[case::clip(json!({ "cmd": "clip" }))]
#[case::set_clip(json!({ "cmd": "set_clip", "clip": { "tab": "t-1", "text": "x", "element": element(), "origin": "w-1" } }))]
#[case::activate(json!({ "cmd": "activate", "tab": "t-1" }))]
#[case::close(json!({ "cmd": "close", "tab": "t-1", "discard": true }))]
#[case::reorder(json!({ "cmd": "reorder", "tab": "t-1", "index": 0 }))]
#[case::move_tab(json!({ "cmd": "move_tab", "tab": "t-1" }))]
#[case::new_window(json!({ "cmd": "new_window" }))]
#[case::close_window(json!({ "cmd": "close_window", "window": "w-1", "discard": ["t-1"] }))]
#[case::detach_agent(json!({ "cmd": "detach_agent" }))]
#[case::import_picked(json!({ "cmd": "import_picked", "id": 1 }))]
#[case::dismiss_import_picks(json!({ "cmd": "dismiss_import_picks", "window": "w-1", "through": 1 }))]
#[case::new_pack(json!({ "cmd": "new_pack", "tab": "t-1", "kind": "widgets" }))]
#[case::new_theme(json!({ "cmd": "new_theme", "tab": "t-1" }))]
#[case::pack_opener(json!({ "cmd": "pack_opener", "tab": "t-1" }))]
fn check_agent_workspace_other_request_is_forbidden(#[case] req: Value) {
    let (_data, ws, _tab) = workspace_with_tab();
    let ws = Workspace::lock(&ws);
    let refused = check_agent_workspace(&ws, &ws_req(req)).map_err(|e| e.kind());
    assert_that!(refused).is_err_containing(SessionErrorKind::AgentWindowForbidden);
}

#[rstest]
#[case::snapshot(json!({ "cmd": "snapshot" }))]
#[case::svg(json!({ "cmd": "svg", "id": "s-1" }))]
fn check_agent_request_allowed_request_on_a_held_tab_passes(#[case] req: Value) {
    let (_data, ws, tab) = workspace_with_tab();
    let ws = Workspace::lock(&ws);
    assert_that!(check_agent_request(&ws, &tab, &tab_req(req))).is_ok();
}

#[rstest]
#[case::unknown_tab("t-999")]
#[case::window_label("w-1")]
#[case::agent_label("agent")]
#[case::empty("")]
fn check_agent_request_snapshot_on_a_tab_no_window_holds_is_forbidden(#[case] tab: &str) {
    let (_data, ws, _tab) = workspace_with_tab();
    let ws = Workspace::lock(&ws);
    let refused = check_agent_request(&ws, tab, &tab_req(json!({ "cmd": "snapshot" }))).map_err(|e| e.kind());
    assert_that!(refused).is_err_containing(SessionErrorKind::AgentWindowForbidden);
}

#[rstest]
#[case::save(json!({ "cmd": "save" }))]
#[case::save_overwrite(json!({ "cmd": "save", "path": "/x.okuslides.pdf", "overwrite": true }))]
#[case::move_(json!({ "cmd": "move", "path": "/x.okuslides.pdf" }))]
#[case::save_copy(json!({ "cmd": "save_copy", "path": "/x.okuslides.pdf" }))]
#[case::merge(json!({ "cmd": "merge", "path": "/x.okuslides.pdf" }))]
#[case::apply(json!({ "cmd": "apply", "label": "x", "ops": [] }))]
#[case::undo(json!({ "cmd": "undo" }))]
#[case::redo(json!({ "cmd": "redo" }))]
#[case::undo_turn(json!({ "cmd": "undo_turn", "turn": 1 }))]
#[case::slide(json!({ "cmd": "slide", "id": "s-1" }))]
#[case::search(json!({ "cmd": "search", "query": "x" }))]
#[case::schema(json!({ "cmd": "schema" }))]
#[case::delete_pack(json!({ "cmd": "delete_pack", "pack": PACK }))]
#[case::history_open(json!({ "cmd": "history_open" }))]
#[case::history_restore(json!({ "cmd": "history_restore" }))]
#[case::save_pack(json!({ "cmd": "save_pack" }))]
#[case::save_theme(json!({ "cmd": "save_theme" }))]
#[case::approve_guides(json!({ "cmd": "approve_guides", "versions": [] }))]
#[case::accept_received(json!({ "cmd": "accept_received", "versions": [] }))]
fn check_agent_request_other_request_is_forbidden(#[case] req: Value) {
    let (_data, ws, tab) = workspace_with_tab();
    let ws = Workspace::lock(&ws);
    let refused = check_agent_request(&ws, &tab, &tab_req(req)).map_err(|e| e.kind());
    assert_that!(refused).is_err_containing(SessionErrorKind::AgentWindowForbidden);
}

#[test]
fn agent_window_forbidden_localizes_in_french() {
    let (_data, ws, _tab) = workspace_with_tab();
    let ws = Workspace::lock(&ws);
    let refused = check_agent_workspace(&ws, &ws_req(json!({ "cmd": "new_window" })));
    let e = refused.err().unwrap_or_else(|| panic!("new_window passed"));
    let fr = Localizer::new(Some("fr"));
    let en = Localizer::new(Some("en"));
    let shown = e.localize(&fr);
    assert_that!(shown.as_str()).is_equal_to(fr.tr("session-agent-window-forbidden").as_str());
    assert_that!(shown.as_str()).is_not_equal_to(en.tr("session-agent-window-forbidden").as_str());
}

#[test]
fn agent_window_forbidden_names_the_refused_op_in_its_log_line() {
    let (_data, ws, _tab) = workspace_with_tab();
    let ws = Workspace::lock(&ws);
    let refused = check_agent_workspace(&ws, &ws_req(json!({ "cmd": "close_window", "window": "w-1" })));
    let e = refused.err().unwrap_or_else(|| panic!("close_window passed"));
    assert_that!(e.one_line()).contains("CloseWindow");
}
