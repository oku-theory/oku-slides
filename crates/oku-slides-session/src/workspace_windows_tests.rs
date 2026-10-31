use serde_json::json;
use speculoos::prelude::*;

use super::workspace_fixtures::{Data, ids, view};
use super::*;
use crate::SessionErrorKind;
use crate::test_support::{apply_ws, workspace_on};

#[test]
fn reorder_moves_the_tab_to_its_new_index_clamped() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    apply_ws(&ws, json!({"cmd": "reorder", "tab": "t-3", "index": 0})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-3", "t-1", "t-2"]);
    apply_ws(&ws, json!({"cmd": "reorder", "tab": "t-3", "index": 99})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1", "t-2", "t-3"]);
}

#[test]
fn move_tab_to_other_window_at_index_activates_it_there() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    apply_ws(&ws, json!({"cmd": "new_window"})).unwrap();
    apply_ws(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": "w-2", "index": 0})).unwrap();
    let v = view(&ws);
    assert_that!(ids(&v.windows[0])).is_equal_to(vec!["t-2"]);
    assert_that!(ids(&v.windows[1])).is_equal_to(vec!["t-1", "t-3"]);
    assert_that!(v.windows[1].active.as_str()).is_equal_to("t-1");
    assert_that!(v.focused.as_str()).is_equal_to("w-2");
}

#[test]
fn move_tab_to_its_own_window_without_index_changes_nothing() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    apply_ws(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": "w-1"})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1", "t-2"]);
    apply_ws(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": "w-1", "index": 1})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-2", "t-1"]);
}

#[test]
fn move_tab_to_new_window_creates_a_window() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    apply_ws(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": null})).unwrap();
    let v = view(&ws);
    assert_that!(v.windows).has_length(2);
    assert_that!(ids(&v.windows[0])).is_equal_to(vec!["t-2"]);
    assert_that!(v.windows[1].label.as_str()).is_equal_to("w-2");
    assert_that!(ids(&v.windows[1])).is_equal_to(vec!["t-1"]);
}

#[test]
fn move_only_tab_to_new_window_removes_its_old_window() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": null})).unwrap();
    let v = view(&ws);
    assert_that!(v.windows).has_length(1);
    assert_that!(v.windows[0].label.as_str()).is_equal_to("w-2");
}

#[test]
fn move_tab_to_unknown_window_returns_error_and_keeps_the_tab() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let err = apply_ws(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": "w-9"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWindow);
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
}

#[test]
fn unknown_tab_returns_unknown_tab_error() {
    let data = Data::new();
    let err = apply_ws(&workspace_on(&data.0), json!({"cmd": "activate", "tab": "t-9"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownTab);
}

#[test]
fn unknown_window_returns_unknown_window_error() {
    let data = Data::new();
    let err = apply_ws(&workspace_on(&data.0), json!({"cmd": "new_tab", "window": "w-9"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWindow);
}

#[test]
fn focus_unknown_window_returns_unknown_window_error() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let err = Workspace::lock(&ws).focus("w-9").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWindow);
}

fn rect(x: f64, y: f64) -> Rect {
    Rect { x, y, w: 100.0, h: 100.0 }
}

#[test]
fn window_at_inside_the_only_window_returns_it_edges_included() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let rects = vec![("w-1".to_owned(), rect(0.0, 0.0))];
    let w = Workspace::lock(&ws);
    assert_that!(w.window_at(0.0, 0.0, &rects)).is_equal_to(Some("w-1".to_owned()));
    assert_that!(w.window_at(100.0, 50.0, &rects)).is_none();
}

#[test]
fn window_at_overlap_prefers_the_most_recently_focused_window() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_window"})).unwrap();
    let rects = vec![("w-1".to_owned(), rect(0.0, 0.0)), ("w-2".to_owned(), rect(50.0, 0.0))];
    let mut w = Workspace::lock(&ws);
    w.focus("w-1").unwrap();
    assert_that!(w.window_at(75.0, 50.0, &rects)).is_equal_to(Some("w-1".to_owned()));
    w.focus("w-2").unwrap();
    assert_that!(w.window_at(75.0, 50.0, &rects)).is_equal_to(Some("w-2".to_owned()));
    // Seule w-1 contient ce point, même si w-2 est la plus récente.
    assert_that!(w.window_at(25.0, 50.0, &rects)).is_equal_to(Some("w-1".to_owned()));
}

/// Deux écrans, en points (macOS) : le portable (1512 × 982, origine du
/// repère), et un écran externe 2560 × 1440 à sa droite, haut aligné 200
/// points plus haut (y négatifs). Une fenêtre 1440 × 900 sur chacun.
fn two_screens() -> Vec<(String, Rect)> {
    vec![
        ("w-1".to_owned(), Rect { x: 36.0, y: 38.0, w: 1440.0, h: 900.0 }),
        ("w-2".to_owned(), Rect { x: 1800.0, y: -120.0, w: 1440.0, h: 900.0 }),
    ]
}

#[test]
fn window_at_two_screens_finds_the_window_on_the_other_screen() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_window"})).unwrap();
    let mut w = Workspace::lock(&ws);
    // L'onglet part de w-1, la plus récente.
    w.focus("w-1").unwrap();
    assert_that!(w.window_at(2000.0, 700.0, &two_screens())).is_equal_to(Some("w-2".to_owned()));
}

#[test]
fn window_at_two_screens_accepts_a_point_above_the_primary_screen() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_window"})).unwrap();
    let w = Workspace::lock(&ws);
    assert_that!(w.window_at(1900.0, -100.0, &two_screens())).is_equal_to(Some("w-2".to_owned()));
}

#[test]
fn window_at_two_screens_between_the_windows_returns_none() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_window"})).unwrap();
    let w = Workspace::lock(&ws);
    // Sur l'écran externe, à gauche de w-2 : une nouvelle fenêtre.
    assert_that!(w.window_at(1600.0, 300.0, &two_screens())).is_none();
}

#[test]
fn tab_window_returns_the_label_of_the_window_holding_the_tab() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_window"})).unwrap();
    let v = view(&ws);
    let tab = v.windows[1].tabs[0].id.clone();
    assert_that!(Workspace::lock(&ws).tab_window(&tab)).is_equal_to(Some(v.windows[1].label.as_str()));
}

#[test]
fn tab_window_unknown_tab_returns_none() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    assert_that!(Workspace::lock(&ws).tab_window("t-99")).is_none();
}

#[test]
fn window_at_ignores_rects_of_unknown_windows() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let rects = vec![("autre".to_owned(), rect(0.0, 0.0))];
    assert_that!(Workspace::lock(&ws).window_at(50.0, 50.0, &rects)).is_none();
}

#[test]
fn tab_shown_active_tab_of_its_window_returns_true() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let tab = view(&ws).windows[0].tabs[0].id.clone();
    assert_that!(Workspace::lock(&ws).tab_shown(&tab)).is_true();
}

#[test]
fn tab_shown_tab_behind_a_newer_one_returns_false() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let first = view(&ws).windows[0].tabs[0].id.clone();
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    assert_that!(Workspace::lock(&ws).tab_shown(&first)).is_false();
}

/// A tab built for a requester that is gone lands in a fresh window, where it is the active tab.
#[test]
fn finish_shown_in_a_fallback_window_reports_the_tab_shown() {
    use super::tabs::{Activate, Opening};
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let mut pending = Workspace::lock(&ws).pending(None, Opening::Blank, TabPlace::End);
    pending.activate = Activate::IfShown("t-404".into());
    let session = pending.build().unwrap();
    let (_, id, shown) = Workspace::finish_shown(&ws, pending, session).unwrap();
    assert_that!(Workspace::lock(&ws).tab_shown(&id)).is_true();
    assert_that!(shown).is_true();
}
