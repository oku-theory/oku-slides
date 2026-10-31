use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::test_support::{apply_ws, fresh_dir, workspace_on};
use crate::{Request, WorkspaceView};
use oku_slides_platform::storage::resolve;

/// A readable deck: `fixtures/roadmap.json`.
fn deck() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

/// A file named `name` in `dir`: never read by `open_files`.
fn pptx(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, b"not read").unwrap();
    path
}

/// The queued paths, in order.
fn paths(view: &WorkspaceView) -> Vec<PathBuf> {
    view.import_picks.iter().map(|p| p.path.clone()).collect()
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
    let pick = Workspace::lock(&ws).view().import_picks[0].clone();
    assert_that!(pick.window.as_str()).is_equal_to("w-2");
    assert_that!(pick.path).is_equal_to(resolve(&file));
}

#[test]
fn open_files_mixed_list_opens_the_decks_and_picks_the_pptx() {
    let data = fresh_dir("oku-slides-ws-pick-mixed-");
    let ws = workspace_on(data.path());
    let file = pptx(data.path(), "alto.pptx");
    Workspace::open_files(&ws, &[file.clone(), deck()]);
    // The blank tab gives way to the deck (a JSON, with no associated file).
    let view = Workspace::lock(&ws).view();
    let tabs: Vec<&str> = view.windows[0].tabs.iter().map(|t| t.id.as_str()).collect();
    assert_that!(tabs).is_equal_to(vec!["t-2"]);
    assert_that!(paths(&view)).is_equal_to(vec![resolve(&file)]);
    assert_that!(notice(&ws, "t-2")).is_equal_to(serde_json::Value::Null);
}

#[rstest]
#[case("alto.PPTX")]
#[case("alto.Pptx")]
fn open_files_pptx_extension_in_any_case_is_picked(#[case] name: &str) {
    let data = fresh_dir("oku-slides-ws-pick-case-");
    let ws = workspace_on(data.path());
    Workspace::open_files(&ws, &[pptx(data.path(), name)]);
    assert_that!(Workspace::lock(&ws).view().import_picks).has_length(1);
}

#[test]
fn open_files_second_pptx_queues_after_the_first_with_a_higher_id() {
    let data = fresh_dir("oku-slides-ws-pick-twice-");
    let ws = workspace_on(data.path());
    let a = pptx(data.path(), "a.pptx");
    let b = pptx(data.path(), "b.pptx");
    Workspace::open_files(&ws, std::slice::from_ref(&a));
    Workspace::open_files(&ws, std::slice::from_ref(&b));
    let view = Workspace::lock(&ws).view();
    assert_that!(paths(&view)).is_equal_to(vec![resolve(&a), resolve(&b)]);
    assert_that!(view.import_picks[1].id).is_greater_than(view.import_picks[0].id);
}

#[test]
fn open_files_several_pptx_at_once_queues_them_all_in_order() {
    let data = fresh_dir("oku-slides-ws-pick-many-");
    let ws = workspace_on(data.path());
    let files = [pptx(data.path(), "a.pptx"), pptx(data.path(), "b.pptx"), pptx(data.path(), "c.pptx")];
    Workspace::open_files(&ws, &files);
    let expected: Vec<PathBuf> = files.iter().map(|f| resolve(f)).collect();
    assert_that!(paths(&Workspace::lock(&ws).view())).is_equal_to(expected);
}

#[test]
fn open_files_pptx_already_queued_is_not_queued_twice() {
    let data = fresh_dir("oku-slides-ws-pick-dup-");
    let ws = workspace_on(data.path());
    let a = pptx(data.path(), "a.pptx");
    Workspace::open_files(&ws, &[a.clone(), a.clone()]);
    Workspace::open_files(&ws, std::slice::from_ref(&a));
    assert_that!(Workspace::lock(&ws).view().import_picks).has_length(1);
}

#[test]
fn open_files_beyond_the_queue_cap_drops_the_extra_pptx() {
    let data = fresh_dir("oku-slides-ws-pick-cap-");
    let ws = workspace_on(data.path());
    let files: Vec<PathBuf> = (0..=MAX_PICKS).map(|i| pptx(data.path(), &format!("{i}.pptx"))).collect();
    Workspace::open_files(&ws, &files);
    let view = Workspace::lock(&ws).view();
    assert_that!(view.import_picks).has_length(MAX_PICKS);
    assert_that!(paths(&view).contains(&resolve(&files[MAX_PICKS]))).is_false();
}

#[test]
fn closing_a_window_moves_its_picks_to_the_focused_window_with_new_ids() {
    let data = fresh_dir("oku-slides-ws-pick-close-");
    let ws = workspace_on(data.path());
    apply_ws(&ws, json!({"cmd": "new_window"})).unwrap();
    Workspace::open_files(&ws, &[pptx(data.path(), "a.pptx")]);
    let before = Workspace::lock(&ws).view().import_picks[0].clone();
    assert_that!(before.window.as_str()).is_equal_to("w-2");
    apply_ws(&ws, json!({"cmd": "close_window", "window": "w-2"})).unwrap();
    let view = Workspace::lock(&ws).view();
    let after = view.import_picks[0].clone();
    assert_that!(after.window).is_equal_to(view.windows[0].label.clone());
    assert_that!(after.id).is_greater_than(before.id);
}

#[test]
fn closing_a_window_queues_its_picks_after_the_waiting_ones_in_id_order() {
    let data = fresh_dir("oku-slides-ws-pick-order-");
    let ws = workspace_on(data.path());
    apply_ws(&ws, json!({"cmd": "new_window"})).unwrap();
    Workspace::open_files(&ws, &[pptx(data.path(), "a.pptx")]);
    Workspace::lock(&ws).touch("w-1");
    Workspace::open_files(&ws, &[pptx(data.path(), "b.pptx")]);
    apply_ws(&ws, json!({"cmd": "close_window", "window": "w-2"})).unwrap();
    let view = Workspace::lock(&ws).view();
    let ids: Vec<u64> = view.import_picks.iter().map(|p| p.id).collect();
    assert_that!(paths(&view))
        .is_equal_to(vec![resolve(&data.path().join("b.pptx")), resolve(&data.path().join("a.pptx"))]);
    assert_that!(ids.is_sorted()).is_true();
}

#[test]
fn open_files_pptx_after_settle_picks_nothing() {
    let data = fresh_dir("oku-slides-ws-pick-settled-");
    let ws = workspace_on(data.path());
    Workspace::settle(&ws);
    Workspace::open_files(&ws, &[pptx(data.path(), "alto.pptx")]);
    assert_that!(Workspace::lock(&ws).view().import_picks).is_empty();
}

#[test]
fn view_without_pick_serializes_import_picks_as_an_empty_list() {
    let data = fresh_dir("oku-slides-ws-pick-json-");
    let ws = workspace_on(data.path());
    let json = serde_json::to_value(Workspace::lock(&ws).view()).unwrap();
    assert_that!(json["import_picks"]).is_equal_to(json!([]));
}

#[test]
fn import_picked_with_the_pick_id_clears_it_from_the_view() {
    let data = fresh_dir("oku-slides-ws-pick-ack-");
    let ws = workspace_on(data.path());
    Workspace::open_files(&ws, &[pptx(data.path(), "alto.pptx")]);
    let id = Workspace::lock(&ws).view().import_picks[0].clone().id;
    let reply = apply_ws(&ws, json!({"cmd": "import_picked", "id": id})).unwrap().into_json();
    let json: serde_json::Value = serde_json::from_str(&reply).unwrap();
    assert_that!(json["import_picks"]).is_equal_to(json!([]));
    assert_that!(Workspace::lock(&ws).view().import_picks).is_empty();
}

#[test]
fn import_picked_with_the_first_id_keeps_the_next_pick() {
    let data = fresh_dir("oku-slides-ws-pick-ack-old-");
    let ws = workspace_on(data.path());
    Workspace::open_files(&ws, &[pptx(data.path(), "a.pptx")]);
    let old = Workspace::lock(&ws).view().import_picks[0].clone().id;
    let b = pptx(data.path(), "b.pptx");
    Workspace::open_files(&ws, std::slice::from_ref(&b));
    apply_ws(&ws, json!({"cmd": "import_picked", "id": old})).unwrap();
    assert_that!(paths(&Workspace::lock(&ws).view())).is_equal_to(vec![resolve(&b)]);
}

#[test]
fn import_picked_after_clearing_keeps_ids_increasing() {
    let data = fresh_dir("oku-slides-ws-pick-ack-next-");
    let ws = workspace_on(data.path());
    Workspace::open_files(&ws, &[pptx(data.path(), "a.pptx")]);
    let first = Workspace::lock(&ws).view().import_picks[0].clone().id;
    apply_ws(&ws, json!({"cmd": "import_picked", "id": first})).unwrap();
    Workspace::open_files(&ws, &[pptx(data.path(), "b.pptx")]);
    let second = Workspace::lock(&ws).view().import_picks[0].clone().id;
    assert_that!(second).is_greater_than(first);
}
