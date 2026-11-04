use std::path::Path;

use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use super::{AgentWorkspace, NoWorkspace, WorkspaceRefusal};
use crate::agent::TabGate;
use crate::import::ImportTarget;

#[test]
fn no_workspace_focused_tab_is_none() {
    assert_that!(NoWorkspace.focused_tab()).is_none();
}

#[test]
fn no_workspace_page_focus_keeps_no_tab() {
    NoWorkspace.page_focus("t-1", true);
    assert_that!(NoWorkspace.focused_tab()).is_none();
}

#[test]
fn no_workspace_unsaved_tabs_is_zero() {
    assert_that!(NoWorkspace.unsaved_tabs()).is_equal_to(0);
}

/// Each change of `NoWorkspace`, its refusal's key.
fn no_workspace_keys() -> Vec<Option<&'static str>> {
    let ws = NoWorkspace;
    let mut gate = |_: TabGate| true;
    let target = ImportTarget::Hifi { witness: None };
    vec![
        ws.tabs("t-1").err().and_then(|e| e.key()),
        ws.activate("t-1", "t-2", &mut gate).err().and_then(|e| e.key()),
        ws.reorder("t-1", "t-2", 0).err().and_then(|e| e.key()),
        ws.move_tab("t-1", "t-2", None, None).err().and_then(|e| e.key()),
        ws.new_window("t-1").err().and_then(|e| e.key()),
        ws.detach_agent(false).err().and_then(|e| e.key()),
        ws.import_tab("t-1", Path::new("/tmp/a.pptx"), target, &mut gate).err().and_then(|e| e.key()),
    ]
}

#[test]
fn no_workspace_refuses_localized() {
    let (fr, en) = (Localizer::new(Some("fr")), Localizer::new(Some("en")));
    let key = "agent-error-workspace-unavailable";
    assert_that!(no_workspace_keys()).is_equal_to(vec![Some(key); 7]);
    let said = WorkspaceRefusal::unavailable().localize(&fr);
    assert_that!(said).is_equal_to(fr.tr(key));
    assert_that!(said).is_not_equal_to(en.tr(key));
}

#[rstest]
#[case::unavailable(WorkspaceRefusal::unavailable(), "agent-error-workspace-unavailable")]
#[case::unknown_tab(WorkspaceRefusal::unknown_tab(), "agent-error-workspace-unknown-tab")]
#[case::unknown_window(WorkspaceRefusal::unknown_window(), "agent-error-workspace-unknown-window")]
#[case::index(WorkspaceRefusal::index_out_of_range(), "agent-error-workspace-index")]
#[case::closing(WorkspaceRefusal::closing(), "agent-error-workspace-closing")]
fn workspace_refusal_key_is_translated_in_french(#[case] refusal: WorkspaceRefusal, #[case] key: &str) {
    let (fr, en) = (Localizer::new(Some("fr")), Localizer::new(Some("en")));
    assert_that!(refusal.key()).is_equal_to(Some(key));
    assert_that!(fr.has(key)).is_true();
    assert_that!(refusal.localize(&fr)).is_equal_to(fr.tr(key));
    assert_that!(refusal.localize(&fr)).is_not_equal_to(en.tr(key));
}

#[test]
fn workspace_refusal_of_a_session_error_is_worded_by_it() {
    let fr = Localizer::new(Some("fr"));
    let e = crate::error::UnknownTabSnafu { id: "t-9" }.build();
    let expected = crate::SessionError::from(e).localize(&fr);
    let refusal = WorkspaceRefusal::session(crate::error::UnknownTabSnafu { id: "t-9" }.build().into());
    assert_that!(refusal.key()).is_none();
    assert_that!(refusal.session_error().is_some()).is_true();
    assert_that!(refusal.localize(&fr)).is_equal_to(expected);
}
