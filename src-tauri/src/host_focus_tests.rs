use std::sync::{Arc, Mutex};

use oku_slides_session::agent::FocusSource;
use oku_slides_session::{Fonts, Platform, Settings, Workspace, WorkspaceOutcome, WorkspaceRequest};
use rstest::rstest;
use speculoos::prelude::*;

use super::{HostFocus, listening_window};

/// A host focus over a fresh workspace (one window `w-1`, tab `t-1`) under a temp dir.
fn host() -> (HostFocus, tempfile::TempDir) {
    let dir = tempfile::Builder::new().prefix("oku-slides-app-focus-").tempdir().unwrap_or_else(|e| panic!("{e}"));
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
    settings.paths.data = Some(dir.path().to_owned());
    settings.paths.decks = Some(dir.path().join("decks"));
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}"));
    let ws = Workspace::new(Fonts::default(), &platform, None).unwrap_or_else(|e| panic!("workspace: {e}"));
    (HostFocus::new(Arc::new(Mutex::new(ws))), dir)
}

/// A host focus over two windows: `w-1` showing `t-1`, `w-2` showing `t-2`, `w-2` in front.
fn two_windows() -> (HostFocus, tempfile::TempDir) {
    let (focus, dir) = host();
    let opened = Workspace::apply(&focus.workspace, WorkspaceRequest::NewWindow).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(matches!(opened, WorkspaceOutcome::Applied(_))).is_true();
    assert_that!(Workspace::lock(&focus.workspace).active_tab("w-2").map(str::to_owned))
        .is_equal_to(Some("t-2".to_owned()));
    (focus, dir)
}

/// The tab the port names and whether it is focused.
fn listening(focus: &HostFocus) -> Option<(String, bool)> {
    focus.focused_tab().map(|f| (f.tab, f.focused))
}

#[rstest]
#[case::workspace_window(Some("w-2"), "w-2")]
#[case::agent_window(Some("agent"), "w-1")]
#[case::other_window(Some("settings"), "w-1")]
#[case::background(None, "w-1")]
fn listening_window_by_focused_label_names_its_window(#[case] focused: Option<&str>, #[case] expected: &str) {
    assert_that!(listening_window(focused, "w-1")).is_equal_to(expected);
}

#[test]
fn agent_window_focus_targets_main_active_tab() {
    let (focus, _dir) = two_windows();
    Workspace::lock(&focus.workspace).focus("w-2").unwrap_or_else(|e| panic!("{e}"));
    focus.gained(crate::agent_window::LABEL);
    let tab = focus.focused_tab().map(|f| (f.tab, f.source, f.focused));
    assert_that!(tab).is_equal_to(Some(("t-2".to_owned(), FocusSource::Host, true)));
}

#[test]
fn focused_tab_follows_the_focused_window_of_two() {
    let (focus, _dir) = two_windows();
    focus.gained("w-1");
    assert_that!(listening(&focus)).is_equal_to(Some(("t-1".to_owned(), true)));
    focus.gained("w-2");
    assert_that!(listening(&focus)).is_equal_to(Some(("t-2".to_owned(), true)));
}

#[test]
fn focus_lost_by_every_window_keeps_last_tab_unfocused() {
    let (focus, _dir) = two_windows();
    Workspace::lock(&focus.workspace).focus("w-2").unwrap_or_else(|e| panic!("{e}"));
    focus.gained("w-2");
    focus.gained(crate::agent_window::LABEL);
    focus.lost("w-2");
    assert_that!(listening(&focus)).is_equal_to(Some(("t-2".to_owned(), true)));
    focus.lost(crate::agent_window::LABEL);
    assert_that!(listening(&focus)).is_equal_to(Some(("t-2".to_owned(), false)));
}

#[test]
fn focused_tab_before_any_focus_is_the_front_tab_unfocused() {
    let (focus, _dir) = host();
    assert_that!(listening(&focus)).is_equal_to(Some(("t-1".to_owned(), false)));
}
