//! Mode historique : lire, parcourir et restaurer une version du deck.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use common::*;
use oku_slides_session::SessionErrorKind;
use serde_json::json;
use speculoos::prelude::*;

fn ok<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("échec inattendu : {e:?}"))
}

/// Le deck de démonstration, puis deux thèmes : « pro », puis « papier ».
fn edited_session() -> Isolated {
    let mut s = session();
    ok(s.open(&fixture_path()));
    ok(s.handle(set_theme("pro")));
    ok(s.handle(set_theme("papier")));
    s
}

/// Comme `edited_session`, avec une image dans les sources.
fn edited_session_with_asset() -> Isolated {
    let deck: oku_slides_core::Deck = ok(serde_json::from_str(&ok(std::fs::read_to_string(fixture_path()))));
    let mut sources = ok(oku_slides_core::SourceBundle::new(deck, common::registry()));
    sources.add_asset(b"image".to_vec());
    let mut s = session();
    ok(s.load(sources, None));
    ok(s.handle(set_theme("pro")));
    ok(s.handle(set_theme("papier")));
    s
}

fn count(s: &mut oku_slides_session::Session) -> u64 {
    snapshot_json(s)["history"]["count"].as_u64().unwrap_or_else(|| panic!("pas d'historique ouvert"))
}

#[test]
fn history_open_shows_the_last_version_read_only() {
    let mut s = edited_session();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    let snap = snapshot_json(&mut s);
    let count = snap["history"]["count"].as_u64().unwrap();
    assert_that!(count).is_greater_than(2);
    assert_that!(snap["history"]["index"]).is_equal_to(json!(count - 1));
    assert_that!(snap["history"]["version"]["label"]).is_equal_to(json!("Thème"));
    assert_that!(snap["history"]["changes"]["changed_since"]).is_equal_to(json!([]));
    assert_that!(snap["theme"]).is_equal_to(json!("papier"));
    assert_that!(snap["can_undo"]).is_equal_to(json!(false));
    let err = s.handle(set_theme("pro")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InHistory);
}

#[test]
fn history_seek_shows_an_earlier_version() {
    let mut s = edited_session();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    let count = count(&mut s);
    s.handle(req(json!({ "cmd": "history_seek", "index": count - 2 }))).unwrap();
    let snap = snapshot_json(&mut s);
    assert_that!(snap["theme"]).is_equal_to(json!("pro"));
    assert_that!(snap["history"]["index"]).is_equal_to(json!(count - 2));
}

#[test]
fn history_seek_past_the_end_shows_the_last_version() {
    let mut s = edited_session();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    let count = count(&mut s);
    s.handle(req(json!({ "cmd": "history_seek", "index": count + 10 }))).unwrap();
    let snap = snapshot_json(&mut s);
    assert_that!(snap["history"]["index"]).is_equal_to(json!(count - 1));
    assert_that!(snap["theme"]).is_equal_to(json!("papier"));
}

#[test]
fn history_seek_to_the_first_version_marks_later_slides_as_added() {
    let mut s = edited_session();
    s.handle(req(json!({ "cmd": "apply", "label": "Ajout", "ops": [
        { "op": "add_slide", "slide": { "id": "s-neuve", "kind": oku_slides_core::kinds::essentiel::CONTENT } }
    ] })))
    .unwrap();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    s.handle(req(json!({ "cmd": "history_seek", "index": 0 }))).unwrap();
    let snap = snapshot_json(&mut s);
    assert_that!(snap["history"]["changes"]["added_since"]).is_equal_to(json!(["s-neuve"]));
}

#[test]
fn history_close_returns_to_the_current_deck_untouched() {
    let mut s = edited_session();
    let before = snapshot_json(&mut s);
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    s.handle(req(json!({ "cmd": "history_seek", "index": 0 }))).unwrap();
    s.handle(req(json!({ "cmd": "history_close" }))).unwrap();
    let after = snapshot_json(&mut s);
    assert_that!(after["theme"]).is_equal_to(&before["theme"]);
    assert_that!(after["history"].is_null()).is_true();
    assert_that!(after["dirty"]).is_equal_to(&before["dirty"]);
    assert_that!(after["can_undo"]).is_equal_to(&before["can_undo"]);
    assert_that!(after["undo_label"]).is_equal_to(&before["undo_label"]);
}

#[test]
fn history_restore_is_a_new_undoable_commit() {
    let mut s = edited_session();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    let count = count(&mut s);
    s.handle(req(json!({ "cmd": "history_seek", "index": count - 2 }))).unwrap();
    s.handle(req(json!({ "cmd": "history_restore" }))).unwrap();
    let snap = snapshot_json(&mut s);
    assert_that!(snap["history"].is_null()).is_true();
    assert_that!(snap["theme"]).is_equal_to(json!("pro"));
    assert_that!(snap["dirty"]).is_equal_to(json!(true));
    assert_that!(snap["can_undo"]).is_equal_to(json!(true));
    assert_that!(snap["undo_label"].as_str().unwrap()).starts_with("Restaurer la version du ");
    s.handle(req(json!({ "cmd": "undo" }))).unwrap();
    assert_that!(snapshot_json(&mut s)["theme"]).is_equal_to(json!("papier"));
}

#[test]
fn history_requests_outside_the_mode_return_history_not_open() {
    let mut s = edited_session();
    for cmd in [
        json!({ "cmd": "history_seek", "index": 0 }),
        json!({ "cmd": "history_restore" }),
        json!({ "cmd": "history_close" }),
        json!({ "cmd": "history_versions" }),
    ] {
        let err = s.handle(req(cmd)).unwrap_err();
        assert_that!(err.kind()).is_equal_to(SessionErrorKind::HistoryNotOpen);
    }
}

#[test]
fn history_mode_refuses_edits_saves_merges_and_pack() {
    let mut s = edited_session();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    for cmd in [
        json!({ "cmd": "history_open" }),
        json!({ "cmd": "undo" }),
        json!({ "cmd": "redo" }),
        json!({ "cmd": "save", "path": null }),
        json!({ "cmd": "merge", "path": "/nulle/part.oku-slides.pdf" }),
        json!({ "cmd": "chart_kind", "label": "x", "slide": "s", "element": "e", "kind": "bar" }),
        json!({ "cmd": "show_as", "label": "x", "slide": "s", "element": "e", "view": "table" }),
        json!({ "cmd": "edit_pack", "pack": uuid::Uuid::new_v4() }),
        json!({ "cmd": "save_pack" }),
        json!({ "cmd": "close_pack" }),
        json!({ "cmd": "delete_pack", "pack": uuid::Uuid::new_v4() }),
    ] {
        let err = s.handle(req(cmd.clone())).unwrap_err();
        assert_that!(err.kind()).named(&cmd.to_string()).is_equal_to(SessionErrorKind::InHistory);
    }
    // Toujours dans l'historique, rien n'a bougé.
    assert_that!(snapshot_json(&mut s)["history"].is_null()).is_false();
}

#[test]
fn history_versions_lists_labels_in_order() {
    let mut s = edited_session();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    let count = count(&mut s);
    let v: serde_json::Value =
        serde_json::from_str(&s.handle(req(json!({ "cmd": "history_versions" }))).unwrap()).unwrap();
    let versions = v.as_array().unwrap();
    assert_that!(versions.len() as u64).is_equal_to(count);
    assert_that!(versions[versions.len() - 1]["label"]).is_equal_to(json!("Thème"));
    assert_that!(versions[0]["timestamp"].is_i64()).is_true();
    assert_that!(versions[0]["session_start"]).is_equal_to(json!(true));
}

#[test]
fn history_keeps_assets_across_open_seek_close() {
    let mut s = edited_session_with_asset();
    let assets = s.sources_for_tests().assets.clone();
    assert_that!(assets.len()).is_equal_to(1);
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    s.handle(req(json!({ "cmd": "history_seek", "index": 0 }))).unwrap();
    s.handle(req(json!({ "cmd": "history_close" }))).unwrap();
    assert_that!(s.sources_for_tests().assets).is_equal_to(&assets);
}

#[test]
fn history_keeps_assets_across_restore() {
    let mut s = edited_session_with_asset();
    let assets = s.sources_for_tests().assets.clone();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    s.handle(req(json!({ "cmd": "history_seek", "index": 0 }))).unwrap();
    s.handle(req(json!({ "cmd": "history_restore" }))).unwrap();
    assert_that!(s.sources_for_tests().assets).is_equal_to(&assets);
}

#[test]
fn history_tab_info_shows_the_current_deck() {
    let mut s = edited_session();
    let before = s.tab_info();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    s.handle(req(json!({ "cmd": "history_seek", "index": 0 }))).unwrap();
    assert_that!(s.tab_info()).is_equal_to(&before);
}
