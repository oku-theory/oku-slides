//! L'agent détaché dans sa fenêtre : un état de l'espace de travail, repris au lancement.

use serde_json::json;
use speculoos::prelude::*;

use crate::test_support::{apply_ws, fresh_dir, resumed_on, workspace_on};
use crate::{Workspace, is_window_label};

#[test]
fn new_workspace_keeps_the_agent_attached() {
    let data = fresh_dir("oku-slides-workspace-detach-");
    let ws = workspace_on(data.path());
    assert_that!(Workspace::lock(&ws).view().agent_detached).is_false();
}

/// Applique une requête qui doit réussir.
fn apply(ws: &std::sync::Mutex<Workspace>, v: serde_json::Value) {
    apply_ws(ws, v).unwrap_or_else(|e| panic!("requête refusée : {e}"));
}

#[test]
fn detach_agent_sets_the_view_flag() {
    let data = fresh_dir("oku-slides-workspace-detach-");
    let ws = workspace_on(data.path());
    apply(&ws, json!({"cmd": "detach_agent"}));
    assert_that!(Workspace::lock(&ws).view().agent_detached).is_true();
}

#[test]
fn attach_agent_clears_the_view_flag() {
    let data = fresh_dir("oku-slides-workspace-detach-");
    let ws = workspace_on(data.path());
    apply(&ws, json!({"cmd": "detach_agent"}));
    apply(&ws, json!({"cmd": "attach_agent"}));
    assert_that!(Workspace::lock(&ws).agent_detached()).is_false();
}

#[test]
fn detached_agent_is_restored_by_resume() {
    let data = fresh_dir("oku-slides-workspace-detach-");
    let dir = data.path().join("resume");
    let ws = resumed_on(data.path(), dir.clone());
    apply(&ws, json!({"cmd": "detach_agent"}));
    Workspace::settle(&ws);
    drop(ws);
    let again = resumed_on(data.path(), dir);
    assert_that!(Workspace::lock(&again).agent_detached()).is_true();
}

#[test]
fn focus_of_the_agent_window_is_refused_and_keeps_the_focused_window() {
    let data = fresh_dir("oku-slides-workspace-detach-");
    let ws = workspace_on(data.path());
    let mut w = Workspace::lock(&ws);
    let before = w.focused().to_owned();
    assert_that!(w.focus("agent")).is_err();
    assert_that!(w.focused()).is_equal_to(before.as_str());
}

#[test]
fn is_only_window_for_the_agent_window_is_false() {
    let data = fresh_dir("oku-slides-workspace-detach-");
    let ws = workspace_on(data.path());
    assert_that!(Workspace::lock(&ws).is_only_window("agent")).is_false();
}

#[test]
fn is_window_label_numbered_w_label_is_true() {
    assert_that!(is_window_label("w-3")).is_true();
}

#[test]
fn is_window_label_agent_label_is_false() {
    assert_that!(is_window_label("agent")).is_false();
}
