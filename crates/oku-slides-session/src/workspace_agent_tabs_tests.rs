use std::sync::Mutex;

use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::super::workspace_fixtures::{ids, view};
use crate::agent::{AgentWorkspace, TabGate, TabOpen};
use crate::import::ImportTarget;
use crate::test_support::{apply_ws, fresh_dir, pptx_fixture, workspace_on};
use crate::{DocumentKind, Workspace};

/// A workspace whose window `w-1` holds `t-1`, `t-2` and `t-3`, `t-3` active.
fn three_tabs(data: &std::path::Path) -> Mutex<Workspace> {
    let ws = workspace_on(data);
    for _ in 0..2 {
        apply_ws(&ws, json!({ "cmd": "new_tab", "window": "w-1" })).unwrap();
    }
    ws
}

/// The key of a refusal.
fn key(e: &crate::agent::WorkspaceRefusal) -> Option<&'static str> {
    e.key()
}

#[test]
fn agent_tabs_lists_only_the_requester_window() {
    let data = fresh_dir("oku-slides-agent-tabs-");
    let ws = three_tabs(data.path());
    apply_ws(&ws, json!({ "cmd": "move_tab", "tab": "t-2" })).unwrap();
    let listed = Workspace::agent_tabs(&ws, "t-1").unwrap();
    let tabs: Vec<(&str, bool, DocumentKind)> = listed.iter().map(|t| (t.tab.as_str(), t.active, t.kind)).collect();
    assert_that!(tabs).is_equal_to(vec![("t-1", false, DocumentKind::Deck), ("t-3", true, DocumentKind::Deck)]);
}

#[test]
fn agent_tabs_of_an_unknown_tab_is_refused() {
    let data = fresh_dir("oku-slides-agent-tabs-unknown-");
    let ws = workspace_on(data.path());
    let refused = Workspace::agent_tabs(&ws, "t-9").unwrap_err();
    assert_that!(key(&refused)).is_equal_to(Some("agent-error-workspace-unknown-tab"));
}

#[test]
fn agent_tabs_after_settle_is_closing() {
    let data = fresh_dir("oku-slides-agent-tabs-settled-");
    let ws = workspace_on(data.path());
    Workspace::settle(&ws);
    let refused = Workspace::agent_tabs(&ws, "t-1").unwrap_err();
    assert_that!(key(&refused)).is_equal_to(Some("agent-error-workspace-closing"));
}

#[test]
fn agent_activate_from_the_shown_tab_shows_the_target() {
    let data = fresh_dir("oku-slides-agent-activate-");
    let ws = three_tabs(data.path());
    let opened = Workspace::agent_activate(&ws, "t-3", "t-1", &mut |g| g == TabGate::Focus).unwrap();
    assert_that!(opened).is_equal_to(TabOpen::Focused { tab: "t-1".into(), shown: true });
    assert_that!(view(&ws).windows[0].active.as_str()).is_equal_to("t-1");
}

#[test]
fn agent_activate_from_a_hidden_tab_leaves_the_bar_as_it_is() {
    let data = fresh_dir("oku-slides-agent-activate-hidden-");
    let ws = three_tabs(data.path());
    let opened = Workspace::agent_activate(&ws, "t-2", "t-1", &mut |_| true).unwrap();
    assert_that!(opened).is_equal_to(TabOpen::Focused { tab: "t-1".into(), shown: false });
    assert_that!(view(&ws).windows[0].active.as_str()).is_equal_to("t-3");
}

#[test]
fn agent_activate_refused_by_the_gate_changes_nothing() {
    let data = fresh_dir("oku-slides-agent-activate-refused-");
    let ws = three_tabs(data.path());
    let opened = Workspace::agent_activate(&ws, "t-3", "t-1", &mut |_| false).unwrap();
    assert_that!(opened).is_equal_to(TabOpen::Refused);
    assert_that!(view(&ws).windows[0].active.as_str()).is_equal_to("t-3");
}

#[test]
fn agent_activate_of_its_own_tab_is_here_without_a_gate() {
    let data = fresh_dir("oku-slides-agent-activate-here-");
    let ws = three_tabs(data.path());
    let opened = Workspace::agent_activate(&ws, "t-3", "t-3", &mut |_| panic!("no gate for its own tab")).unwrap();
    assert_that!(opened).is_equal_to(TabOpen::Here);
}

#[test]
fn agent_activate_of_another_window_tab_is_refused() {
    let data = fresh_dir("oku-slides-agent-activate-elsewhere-");
    let ws = three_tabs(data.path());
    apply_ws(&ws, json!({ "cmd": "move_tab", "tab": "t-2" })).unwrap();
    let refused = Workspace::agent_activate(&ws, "t-1", "t-2", &mut |_| true).unwrap_err();
    assert_that!(key(&refused)).is_equal_to(Some("agent-error-workspace-unknown-tab"));
}

#[test]
fn agent_reorder_moves_the_tab_to_its_index() {
    let data = fresh_dir("oku-slides-agent-reorder-");
    let ws = three_tabs(data.path());
    Workspace::agent_reorder(&ws, "t-1", "t-3", 0).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-3", "t-1", "t-2"]);
}

#[rstest]
#[case::past_the_end(4)]
#[case::max(usize::MAX)]
fn agent_reorder_index_out_of_range_is_refused(#[case] index: usize) {
    let data = fresh_dir("oku-slides-agent-reorder-range-");
    let ws = three_tabs(data.path());
    let refused = Workspace::agent_reorder(&ws, "t-1", "t-3", index).unwrap_err();
    assert_that!(key(&refused)).is_equal_to(Some("agent-error-workspace-index"));
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1", "t-2", "t-3"]);
}

#[test]
fn agent_move_to_a_new_window_opens_it_with_the_tab() {
    let data = fresh_dir("oku-slides-agent-move-");
    let ws = three_tabs(data.path());
    Workspace::agent_move(&ws, "t-1", "t-2", None, None).unwrap();
    let v = view(&ws);
    let windows: Vec<Vec<&str>> = v.windows.iter().map(ids).collect();
    assert_that!(windows).is_equal_to(vec![vec!["t-1", "t-3"], vec!["t-2"]]);
}

#[test]
fn agent_move_to_an_unknown_window_is_refused() {
    let data = fresh_dir("oku-slides-agent-move-unknown-");
    let ws = three_tabs(data.path());
    let refused = Workspace::agent_move(&ws, "t-1", "t-2", Some("w-9"), None).unwrap_err();
    assert_that!(key(&refused)).is_equal_to(Some("agent-error-workspace-unknown-window"));
}

#[test]
fn agent_move_to_a_new_window_past_its_first_place_is_refused() {
    let data = fresh_dir("oku-slides-agent-move-range-");
    let ws = three_tabs(data.path());
    let refused = Workspace::agent_move(&ws, "t-1", "t-2", None, Some(1)).unwrap_err();
    assert_that!(key(&refused)).is_equal_to(Some("agent-error-workspace-index"));
}

#[test]
fn agent_new_window_adds_a_window_with_a_blank_tab() {
    let data = fresh_dir("oku-slides-agent-new-window-");
    let ws = workspace_on(data.path());
    Workspace::agent_new_window(&ws, "t-1").unwrap();
    assert_that!(view(&ws).windows.len()).is_equal_to(2);
}

#[test]
fn agent_detach_then_attach_follows_the_agent_window() {
    let data = fresh_dir("oku-slides-agent-detach-");
    let ws = workspace_on(data.path());
    Workspace::agent_detach(&ws, false).unwrap();
    let detached = view(&ws).agent_detached;
    Workspace::agent_detach(&ws, true).unwrap();
    assert_that!((detached, view(&ws).agent_detached)).is_equal_to((true, false));
}

#[test]
fn agent_import_opens_a_new_tab_beside_a_blank_one() {
    let data = fresh_dir("oku-slides-agent-import-");
    let ws = workspace_on(data.path());
    let pptx = pptx_fixture(data.path(), "minimal");
    let target = ImportTarget::Hifi { witness: None };
    let opened = Workspace::agent_import(&ws, "t-1", &pptx, target, &mut |g| g == TabGate::NewTab).unwrap();
    assert_that!(opened).is_equal_to(TabOpen::Opened { tab: "t-2".into(), shown: true });
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1", "t-2"]);
}

#[test]
fn agent_import_refused_by_the_gate_opens_nothing() {
    let data = fresh_dir("oku-slides-agent-import-refused-");
    let ws = workspace_on(data.path());
    let pptx = pptx_fixture(data.path(), "minimal");
    let target = ImportTarget::Hifi { witness: None };
    let opened = Workspace::agent_import(&ws, "t-1", &pptx, target, &mut |_| false).unwrap();
    assert_that!(opened).is_equal_to(TabOpen::Refused);
    assert_that!(Workspace::lock(&ws).tab_ids()).is_equal_to(vec!["t-1".to_owned()]);
}

#[test]
fn agent_import_of_an_unreadable_file_is_the_session_error() {
    let data = fresh_dir("oku-slides-agent-import-missing-");
    let ws = workspace_on(data.path());
    let target = ImportTarget::Hifi { witness: None };
    let missing = data.path().join("absent.pptx");
    let refused = Workspace::agent_import(&ws, "t-1", &missing, target, &mut |_| true).unwrap_err();
    assert_that!(refused.session_error().map(crate::SessionError::kind))
        .is_equal_to(Some(crate::SessionErrorKind::Import));
    assert_that!(Workspace::lock(&ws).tab_ids()).is_equal_to(vec!["t-1".to_owned()]);
}

#[test]
fn agent_workspace_of_the_dev_server_lists_and_imports_through_the_workspace() {
    let data = fresh_dir("oku-slides-agent-serve-");
    let ws = workspace_on(data.path());
    let port: &dyn AgentWorkspace = &ws;
    let pptx = pptx_fixture(data.path(), "minimal");
    port.import_tab("t-1", &pptx, ImportTarget::Hifi { witness: None }, &mut |_| true).unwrap();
    let listed: Vec<String> = port.tabs("t-1").unwrap().into_iter().map(|t| t.tab).collect();
    assert_that!(listed).is_equal_to(vec!["t-1".to_owned(), "t-2".to_owned()]);
}

#[test]
fn agent_workspace_of_the_dev_server_has_no_agent_window() {
    let data = fresh_dir("oku-slides-agent-serve-detach-");
    let ws = workspace_on(data.path());
    let port: &dyn AgentWorkspace = &ws;
    assert_that!(port.detach_agent(false).err().and_then(|e| e.key()))
        .is_equal_to(Some("agent-error-workspace-unavailable"));
}

#[test]
fn agent_dirty_revisions_of_a_tab_names_only_it() {
    let data = fresh_dir("oku-slides-agent-revisions-tab-");
    let ws = three_tabs(data.path());
    let tabs = Workspace::agent_dirty_revisions(&ws, "t-1", crate::agent::CloseScope::Tab("t-2")).unwrap();
    let ids: Vec<(&str, bool, Option<u64>)> = tabs.tabs.iter().map(|t| (t.tab.as_str(), t.dirty, t.revision)).collect();
    assert_that!(ids).is_equal_to(vec![("t-2", false, None)]);
    assert_that!(tabs.window.as_str()).is_equal_to("w-1");
    assert_that!(tabs.last_window).is_true();
}

#[test]
fn agent_dirty_revisions_of_the_window_lists_every_tab() {
    let data = fresh_dir("oku-slides-agent-revisions-window-");
    let ws = three_tabs(data.path());
    apply_ws(&ws, json!({ "cmd": "move_tab", "tab": "t-3" })).unwrap();
    let tabs = Workspace::agent_dirty_revisions(&ws, "t-1", crate::agent::CloseScope::Window).unwrap();
    let ids: Vec<&str> = tabs.tabs.iter().map(|t| t.tab.as_str()).collect();
    assert_that!(ids).is_equal_to(vec!["t-1", "t-2"]);
    assert_that!(tabs.last_window).is_false();
}

#[test]
fn agent_dirty_revisions_of_another_window_tab_is_refused() {
    let data = fresh_dir("oku-slides-agent-revisions-other-");
    let ws = three_tabs(data.path());
    apply_ws(&ws, json!({ "cmd": "move_tab", "tab": "t-3" })).unwrap();
    let refused = Workspace::agent_dirty_revisions(&ws, "t-1", crate::agent::CloseScope::Tab("t-3")).unwrap_err();
    assert_that!(key(&refused)).is_equal_to(Some("agent-error-workspace-unknown-tab"));
}
