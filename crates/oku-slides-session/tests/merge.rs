use oku_slides_session::{Request, Saved};
use serde_json::json;
use speculoos::prelude::*;

mod common;
use common::{data_dir, fixture_path, req, session, set_theme, snapshot_json};

/// Change le titre de la troisième slide de la fixture (`s-ch1`, « Contexte »).
fn retitle_third_slide(title: &str) -> Request {
    req(json!({
        "cmd": "apply", "label": "Titre",
        "ops": [{"op": "set_param", "target": {"slide": "s-ch1"}, "param": "title", "value": title}]
    }))
}

// ---- Fusion avec un PDF du même deck (t-238) ----------------------------------------

/// Le deck de la fixture enregistré dans `dir/deck.oku-slides.pdf` par une
/// session, puis une autre copie, `dir/<other>`, où le thème est passé à `pro`.
fn saved_deck_and_modified_copy(dir: &std::path::Path, other: &str) -> (common::Isolated, std::path::PathBuf) {
    fn ok<T>(r: oku_slides_session::Result<T>) -> T {
        r.unwrap_or_else(|e| panic!("préparation impossible : {}", e.one_line()))
    }
    let path = dir.join("deck.oku-slides.pdf");
    let mut a = session();
    ok(a.open(&fixture_path()));
    ok(a.save(Some(&path)));
    let mut b = session();
    ok(b.open(&path));
    ok(b.handle(set_theme("pro")));
    let other = dir.join(other);
    ok(b.save(Some(&other)));
    (a, other)
}

#[test]
fn merge_file_brings_the_other_copy_changes() {
    let dir = data_dir();
    let (mut a, other) = saved_deck_and_modified_copy(&dir, "julie.oku-slides.pdf");
    let report = a.merge_file(&other).unwrap_or_else(|e| panic!("fusion refusée : {}", e.one_line()));
    assert_that!(report.changes).is_greater_than(0);
    assert_that!(a.deck().theme.as_ref().and_then(common::theme_name)).is_equal_to(Some("pro"));
    let snap = snapshot_json(&mut a);
    assert_that!(snap["dirty"]).is_equal_to(json!(true));
    assert_that!(snap["notice"].as_str().unwrap_or_default()).contains("julie.oku-slides.pdf");
}

#[test]
fn merge_request_returns_the_merged_snapshot_with_the_notice_once() {
    let dir = data_dir();
    let (mut a, other) = saved_deck_and_modified_copy(&dir, "marc.oku-slides.pdf");
    let reply = a.handle(req(json!({"cmd": "merge", "path": other}))).unwrap();
    let snap: serde_json::Value = serde_json::from_str(&reply).unwrap();
    assert_that!(snap["theme"]["id"]).is_equal_to(json!(oku_slides_session::PRO_THEME));
    assert_that!(snap["notice"].as_str().unwrap_or_default()).contains("marc.oku-slides.pdf");
    asserting("l'avis ne s'affiche qu'une fois").that(&snapshot_json(&mut a)["notice"]).is_equal_to(json!(null));
}

#[test]
fn merge_file_of_another_deck_returns_merge_unrelated() {
    let dir = data_dir();
    let other = dir.join("autre.oku-slides.pdf");
    let mut b = session();
    b.open(&fixture_path()).unwrap();
    b.save(Some(&other)).unwrap();
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    let err = a.merge_file(&other).unwrap_err();
    assert_that!(err.kind()).is_equal_to(oku_slides_session::SessionErrorKind::MergeUnrelated);
}

#[test]
fn merge_file_of_a_json_deck_returns_not_oku_slides() {
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    let err = a.merge_file(&fixture_path()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(oku_slides_session::SessionErrorKind::NotOkuSlides);
}

#[test]
fn merge_request_while_editing_pack_is_refused() {
    let dir = data_dir();
    let path = dir.join("deck.oku-slides.pdf");
    let mut a = session();
    a.save(Some(&path)).unwrap();
    a.edit_pack(oku_slides_session::BASE_PACK).unwrap();
    let err = a.handle(req(json!({"cmd": "merge", "path": path}))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(oku_slides_session::SessionErrorKind::EditingPack);
}

#[test]
fn merge_file_bringing_nothing_new_keeps_the_notice_but_not_dirty() {
    let dir = data_dir();
    let path = dir.join("deck.oku-slides.pdf");
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    a.save(Some(&path)).unwrap();
    let report = a.merge_file(&path).unwrap_or_else(|e| panic!("fusion refusée : {}", e.one_line()));
    assert_that!(report.changes).is_equal_to(0);
    let snap = snapshot_json(&mut a);
    assert_that!(snap["dirty"]).is_equal_to(json!(false));
    assert_that!(snap["notice"].as_str().unwrap_or_default()).contains("0");
}

// ---- Enregistrer : fusion ou fichier remplacé (t-239) --------------------------------

#[test]
fn save_over_a_file_changed_by_another_copy_merges_first() {
    let dir = data_dir();
    let path = dir.join("deck.oku-slides.pdf");
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    a.save(Some(&path)).unwrap();
    let mut b = session();
    b.open(&path).unwrap();
    b.handle(set_theme("pro")).unwrap();
    b.save(Some(&path)).unwrap();
    // A modifie un titre puis enregistre au même endroit.
    a.handle(retitle_third_slide("Contexte (A)")).unwrap();
    let saved = a.save_checked(None, false).unwrap();
    assert_that!(matches!(saved, Saved::Merged(_))).is_true();
    let mut c = session();
    c.open(&path).unwrap();
    assert_that!(c.deck().theme.as_ref().and_then(common::theme_name)).is_equal_to(Some("pro"));
    assert_that!(c.deck().slides[2].params.text("title")).is_equal_to(Some("Contexte (A)"));
}

#[test]
fn save_over_a_file_replaced_by_another_deck_is_refused_until_overwrite() {
    let dir = data_dir();
    let path = dir.join("deck.oku-slides.pdf");
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    a.save(Some(&path)).unwrap();
    let mut other = session();
    other.open(&fixture_path()).unwrap();
    other.save(Some(&path)).unwrap(); // autre deck_uid au même chemin
    a.handle(set_theme("pro")).unwrap();
    assert_that!(matches!(a.save_checked(None, false).unwrap(), Saved::DiskReplaced)).is_true();
    let snap: serde_json::Value = serde_json::from_str(&a.handle(Request::Snapshot).unwrap()).unwrap();
    assert_that!(snap["disk_replaced"]).is_equal_to(serde_json::json!(true));
    asserting("le signal ne part qu'une fois").that(&snapshot_json(&mut a)["disk_replaced"]).is_equal_to(json!(false));
    assert_that!(matches!(a.save_checked(None, true).unwrap(), Saved::Written)).is_true();
}

#[test]
fn save_request_over_a_replaced_file_writes_nothing_and_flags_the_snapshot() {
    let dir = data_dir();
    let path = dir.join("deck.oku-slides.pdf");
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    a.save(Some(&path)).unwrap();
    let mut other = session();
    other.open(&fixture_path()).unwrap();
    other.save(Some(&path)).unwrap();
    let theirs = std::fs::read(&path).unwrap();
    a.handle(set_theme("pro")).unwrap();
    let snap: serde_json::Value =
        serde_json::from_str(&a.handle(req(json!({"cmd": "save", "path": null}))).unwrap()).unwrap();
    assert_that!(snap["disk_replaced"]).is_equal_to(json!(true));
    assert_that!(snap["dirty"]).is_equal_to(json!(true));
    asserting("le fichier de l'autre deck est intact").that(&(std::fs::read(&path).unwrap() == theirs)).is_true();
    let snap: serde_json::Value =
        serde_json::from_str(&a.handle(req(json!({"cmd": "save", "path": null, "overwrite": true}))).unwrap()).unwrap();
    assert_that!(snap["disk_replaced"]).is_equal_to(json!(false));
    assert_that!(snap["dirty"]).is_equal_to(json!(false));
}

#[test]
fn save_over_an_unchanged_file_just_writes() {
    let dir = data_dir();
    let path = dir.join("deck.oku-slides.pdf");
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    a.save(Some(&path)).unwrap();
    a.handle(set_theme("pro")).unwrap();
    assert_that!(matches!(a.save_checked(None, false).unwrap(), Saved::Written)).is_true();
    assert_that!(snapshot_json(&mut a)["dirty"]).is_equal_to(json!(false));
    let mut c = session();
    c.open(&path).unwrap();
    assert_that!(c.deck().theme.as_ref().and_then(common::theme_name)).is_equal_to(Some("pro"));
}

#[test]
fn save_after_the_file_was_deleted_just_writes() {
    let dir = data_dir();
    let path = dir.join("deck.oku-slides.pdf");
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    a.save(Some(&path)).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_that!(matches!(a.save_checked(None, false).unwrap(), Saved::Written)).is_true();
    assert_that!(snapshot_json(&mut a)["dirty"]).is_equal_to(json!(false));
    let mut c = session();
    c.open(&path).unwrap();
    assert_that!(c.deck().slides.len()).is_equal_to(a.deck().slides.len());
}

#[test]
fn save_request_merging_a_changed_file_ends_clean_with_the_merge_notice() {
    let dir = data_dir();
    let path = dir.join("deck.oku-slides.pdf");
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    a.save(Some(&path)).unwrap();
    let mut b = session();
    b.open(&path).unwrap();
    b.handle(set_theme("pro")).unwrap();
    b.save(Some(&path)).unwrap();
    a.handle(retitle_third_slide("Contexte (A)")).unwrap();
    let snap: serde_json::Value =
        serde_json::from_str(&a.handle(req(json!({"cmd": "save", "path": null}))).unwrap()).unwrap();
    assert_that!(snap["dirty"]).is_equal_to(json!(false));
    assert_that!(snap["disk_replaced"]).is_equal_to(json!(false));
    assert_that!(snap["theme"]["id"]).is_equal_to(json!(oku_slides_session::PRO_THEME));
    assert_that!(snap["notice"].as_str().unwrap_or_default()).contains("deck.oku-slides.pdf");
}

#[test]
fn save_over_a_path_that_cannot_be_read_is_refused() {
    let dir = data_dir();
    let path = dir.join("deck.oku-slides.pdf");
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    a.save(Some(&path)).unwrap();
    // Un dossier à la place du fichier : illisible, mais présent.
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    a.handle(set_theme("pro")).unwrap();
    assert_that!(matches!(a.save_checked(None, false).unwrap(), Saved::DiskReplaced)).is_true();
    assert_that!(path.is_dir()).is_true();
}

#[cfg(unix)]
#[test]
fn save_over_a_file_without_read_permission_is_refused() {
    use std::os::unix::fs::PermissionsExt;
    let dir = data_dir();
    let path = dir.join("deck.oku-slides.pdf");
    let mut a = session();
    a.open(&fixture_path()).unwrap();
    a.save(Some(&path)).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    a.handle(set_theme("pro")).unwrap();
    let saved = a.save_checked(None, false).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_that!(matches!(saved, Saved::DiskReplaced)).is_true();
}
