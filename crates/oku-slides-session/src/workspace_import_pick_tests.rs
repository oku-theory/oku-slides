use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::Request;
use crate::test_support::{apply_ws, fresh_dir, workspace_on};
use oku_slides_platform::storage::resolve;

/// Un deck lisible : `fixtures/roadmap.json`.
fn deck() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

/// Un fichier nommé `name` dans `dir` : jamais lu par `open_files`.
fn pptx(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, b"pas lu").unwrap();
    path
}

fn notice(ws: &Mutex<Workspace>, tab: &str) -> serde_json::Value {
    let req: Request = serde_json::from_value(json!({"cmd": "snapshot"})).unwrap();
    let reply = Workspace::request(ws, tab, req).0.unwrap();
    serde_json::from_str::<serde_json::Value>(&reply).unwrap()["notice"].clone()
}

#[test]
fn open_files_pptx_opens_no_tab_and_sets_no_notice() {
    let data = fresh_dir("oku-slides-ws-pick-");
    let ws = workspace_on(data.path());
    Workspace::open_files(&ws, &[pptx(data.path(), "alto.pptx")]);
    assert_that!(Workspace::lock(&ws).tab_ids()).is_equal_to(vec!["t-1".to_owned()]);
    assert_that!(notice(&ws, "t-1")).is_equal_to(serde_json::Value::Null);
}

#[test]
fn open_files_pptx_is_the_import_pick_of_the_focused_window() {
    let data = fresh_dir("oku-slides-ws-pick-focus-");
    let ws = workspace_on(data.path());
    apply_ws(&ws, json!({"cmd": "new_window"})).unwrap();
    let file = pptx(data.path(), "alto.pptx");
    Workspace::open_files(&ws, std::slice::from_ref(&file));
    let pick = Workspace::lock(&ws).view().import_pick.unwrap();
    assert_that!(pick.window.as_str()).is_equal_to("w-2");
    assert_that!(pick.path).is_equal_to(resolve(&file));
}

#[test]
fn open_files_mixed_list_opens_the_decks_and_picks_the_pptx() {
    let data = fresh_dir("oku-slides-ws-pick-mixed-");
    let ws = workspace_on(data.path());
    let file = pptx(data.path(), "alto.pptx");
    Workspace::open_files(&ws, &[file.clone(), deck()]);
    // L'onglet vierge est remplacé par le deck (un JSON, sans fichier associé).
    let view = Workspace::lock(&ws).view();
    let tabs: Vec<&str> = view.windows[0].tabs.iter().map(|t| t.id.as_str()).collect();
    assert_that!(tabs).is_equal_to(vec!["t-2"]);
    assert_that!(view.import_pick.map(|p| p.path)).is_equal_to(Some(resolve(&file)));
    assert_that!(notice(&ws, "t-2")).is_equal_to(serde_json::Value::Null);
}

#[rstest]
#[case("alto.PPTX")]
#[case("alto.Pptx")]
fn open_files_pptx_extension_in_any_case_is_picked(#[case] name: &str) {
    let data = fresh_dir("oku-slides-ws-pick-case-");
    let ws = workspace_on(data.path());
    Workspace::open_files(&ws, &[pptx(data.path(), name)]);
    assert_that!(Workspace::lock(&ws).view().import_pick).is_some();
}

#[test]
fn open_files_second_pptx_replaces_the_pick_with_a_higher_id() {
    let data = fresh_dir("oku-slides-ws-pick-twice-");
    let ws = workspace_on(data.path());
    Workspace::open_files(&ws, &[pptx(data.path(), "a.pptx")]);
    let first = Workspace::lock(&ws).view().import_pick.unwrap();
    let b = pptx(data.path(), "b.pptx");
    Workspace::open_files(&ws, std::slice::from_ref(&b));
    let second = Workspace::lock(&ws).view().import_pick.unwrap();
    assert_that!(second.id).is_greater_than(first.id);
    assert_that!(second.path).is_equal_to(resolve(&b));
}

#[test]
fn open_files_pptx_after_settle_picks_nothing() {
    let data = fresh_dir("oku-slides-ws-pick-settled-");
    let ws = workspace_on(data.path());
    Workspace::settle(&ws);
    Workspace::open_files(&ws, &[pptx(data.path(), "alto.pptx")]);
    assert_that!(Workspace::lock(&ws).view().import_pick).is_none();
}

#[test]
fn view_without_pick_serializes_import_pick_as_null() {
    let data = fresh_dir("oku-slides-ws-pick-json-");
    let ws = workspace_on(data.path());
    let json = serde_json::to_value(Workspace::lock(&ws).view()).unwrap();
    assert_that!(json["import_pick"]).is_equal_to(serde_json::Value::Null);
    assert_that!(json.as_object().unwrap().contains_key("import_pick")).is_true();
}

#[test]
fn import_picked_with_the_pick_id_clears_it_from_the_view() {
    let data = fresh_dir("oku-slides-ws-pick-ack-");
    let ws = workspace_on(data.path());
    Workspace::open_files(&ws, &[pptx(data.path(), "alto.pptx")]);
    let id = Workspace::lock(&ws).view().import_pick.unwrap().id;
    let reply = apply_ws(&ws, json!({"cmd": "import_picked", "id": id})).unwrap().into_json();
    let json: serde_json::Value = serde_json::from_str(&reply).unwrap();
    assert_that!(json["import_pick"]).is_equal_to(serde_json::Value::Null);
    assert_that!(Workspace::lock(&ws).view().import_pick).is_none();
}

#[test]
fn import_picked_with_an_older_id_keeps_the_newer_pick() {
    let data = fresh_dir("oku-slides-ws-pick-ack-old-");
    let ws = workspace_on(data.path());
    Workspace::open_files(&ws, &[pptx(data.path(), "a.pptx")]);
    let old = Workspace::lock(&ws).view().import_pick.unwrap().id;
    let b = pptx(data.path(), "b.pptx");
    Workspace::open_files(&ws, std::slice::from_ref(&b));
    apply_ws(&ws, json!({"cmd": "import_picked", "id": old})).unwrap();
    let pick = Workspace::lock(&ws).view().import_pick.map(|p| p.path);
    assert_that!(pick).is_equal_to(Some(resolve(&b)));
}

#[test]
fn import_picked_after_clearing_keeps_ids_increasing() {
    let data = fresh_dir("oku-slides-ws-pick-ack-next-");
    let ws = workspace_on(data.path());
    Workspace::open_files(&ws, &[pptx(data.path(), "a.pptx")]);
    let first = Workspace::lock(&ws).view().import_pick.unwrap().id;
    apply_ws(&ws, json!({"cmd": "import_picked", "id": first})).unwrap();
    Workspace::open_files(&ws, &[pptx(data.path(), "b.pptx")]);
    let second = Workspace::lock(&ws).view().import_pick.unwrap().id;
    assert_that!(second).is_greater_than(first);
}
