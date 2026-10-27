use std::path::{Path, PathBuf};

use oku_slides_core::{Op, PackVersion};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::pack_lease::PackLease;
use crate::resume::Resumed;
use crate::test_support::{edit_state, fresh_dir, ns, session_on};
use crate::test_support_packs::{assert_unleased, theme_scope_of};
use crate::theme_edit::theme_edit_tests::p;
use crate::theme_tab::theme_tab_tests::{draft_of, with_maison};
use crate::{DocumentKind, Mode, Session, TabDocument};

/// A theme tab on the new user theme `maison` in `data`, its resume on `dir`: the tab and the theme.
fn persisted_maison(data: &Path, dir: &Path) -> (Session, Uuid) {
    let (mut s, id) = with_maison(data);
    s.open_theme_here(id).unwrap();
    assert_that!(s.resume_fresh(dir.to_path_buf())).is_true();
    (s, id)
}

/// Writes `s`'s resume, then drops it: nobody holds `theme` afterwards.
fn park(s: Session, theme: Uuid) {
    s.flush_resume();
    let scope = theme_scope_of(&s);
    drop(s);
    assert_unleased(&scope, theme);
}

/// A new session on `data` resumed from `dir`, cached as `cached`'s theme tab.
fn reopen(data: &Path, dir: &Path, cached: Uuid) -> (Session, Resumed) {
    let mut back = session_on(data);
    let resumed = back.resume_checked(dir.to_path_buf(), Some(&TabDocument::Theme { theme: cached }));
    (back, resumed)
}

/// Sets the draft's `color.accent` to `value`, one undo step.
fn accent(s: &mut Session, value: &str) {
    s.apply("x", vec![Op::SetToken { path: p(&["color", "accent"]), value: json!(value) }], None).unwrap();
}

/// The data folder and the tab's resume folder in it.
fn folders() -> (tempfile::TempDir, PathBuf) {
    let data = fresh_dir("oku-slides-resume-theme-");
    let dir = data.path().join("tab");
    (data, dir)
}

/// The resumed state of `dir`.
fn state_of(dir: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(dir.join("state.json")).unwrap()).unwrap()
}

/// A version no library holds.
fn stray_version() -> Value {
    json!(format!("sha256:{}", "b".repeat(64)))
}

#[test]
fn theme_tab_survives_resume_with_its_draft() {
    let (data, dir) = folders();
    let (mut s, id) = persisted_maison(data.path(), &dir);
    accent(&mut s, "#123456");
    let saved = s.save_theme(false).unwrap().unwrap();
    accent(&mut s, "#ff0000");
    s.rename_edited_theme("Maison 2").unwrap();
    s.set_edited_theme_extends(vec![ns("pro")]).unwrap();
    let specimen = s.snapshot().slides[1].id.to_owned();
    s.ui.slide = Some(specimen.clone());
    s.persist(false);
    park(s, id);
    let (back, resumed) = reopen(data.path(), &dir, id);
    let draft = draft_of(&back);
    let Mode::Theme { edit, .. } = &back.mode else { panic!("theme tab expected") };
    let found = (
        resumed,
        back.deck().tokens.pointer("/color/accent").cloned(),
        draft.name.clone(),
        draft.extends.clone(),
        edit.session_version.clone(),
        back.ui.slide.clone(),
        back.doc.sources.replica().can_undo(),
    );
    let expected = (
        Resumed::Restored,
        Some(json!("#ff0000")),
        "Maison 2".to_owned(),
        vec![ns("pro")],
        Some(saved.version),
        Some(specimen),
        false,
    );
    assert_that!(found).is_equal_to(expected);
}

#[test]
fn theme_tab_state_is_version_5_without_documents() {
    let (data, dir) = folders();
    let (s, _) = persisted_maison(data.path(), &dir);
    s.flush_resume();
    let mut names: Vec<String> =
        std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    let state = state_of(&dir);
    let found =
        (names, state["version"].clone(), state["mode"]["theme"].is_object(), state["draft_tokens"].is_object());
    assert_that!(found).is_equal_to((vec!["state.json".to_owned()], json!(5), true, true));
}

#[test]
fn resumed_theme_tab_with_changes_is_dirty() {
    let (data, dir) = folders();
    let (mut s, id) = persisted_maison(data.path(), &dir);
    accent(&mut s, "#ff0000");
    park(s, id);
    let (back, _) = reopen(data.path(), &dir, id);
    assert_that!(back.tab_info().dirty).is_true();
}

#[test]
fn resumed_theme_tab_without_changes_is_clean() {
    let (data, dir) = folders();
    let (s, id) = persisted_maison(data.path(), &dir);
    park(s, id);
    let (back, _) = reopen(data.path(), &dir, id);
    assert_that!(back.tab_info().dirty).is_false();
}

#[rstest]
#[case::cached_document_of_another_theme(|_: &mut Value| {}, true)]
#[case::unknown_base_version(|v: &mut Value| v["mode"]["theme"]["draft"]["base"]["version"] = stray_version(), false)]
#[case::parent_not_an_ancestor(|v: &mut Value| v["mode"]["theme"]["parent"] = stray_version(), false)]
#[case::namespace_unlike_the_base(|v: &mut Value| v["mode"]["theme"]["draft"]["namespace"] = json!("autre"), false)]
#[case::unknown_extends(|v: &mut Value| v["mode"]["theme"]["draft"]["extends"] = json!(["inconnu"]), false)]
#[case::invalid_draft_tokens(|v: &mut Value| v["draft_tokens"]["color"]["accent"] = json!("pas une couleur"), false)]
#[case::no_draft_tokens(|v: &mut Value| { v.as_object_mut().unwrap().remove("draft_tokens"); }, false)]
#[case::version_4(|v: &mut Value| v["version"] = json!(4), false)]
#[case::version_3(|v: &mut Value| v["version"] = json!(3), false)]
fn resume_theme_tab_refused(#[case] forge: fn(&mut Value), #[case] other_cached: bool) {
    let (data, dir) = folders();
    let (s, id) = persisted_maison(data.path(), &dir);
    let scope = theme_scope_of(&s);
    park(s, id);
    edit_state(&dir, forge);
    let cached = if other_cached { Uuid::new_v4() } else { id };
    let (back, resumed) = reopen(data.path(), &dir, cached);
    let found = (resumed, back.document_kind(), dir.join("state.json").exists());
    assert_that!(found).is_equal_to((Resumed::Unreadable, DocumentKind::Deck, false));
    assert_unleased(&scope, id);
}

#[test]
fn resume_theme_tab_refused_leaves_its_base_unfrozen() {
    let (data, dir) = folders();
    let (s, id) = persisted_maison(data.path(), &dir);
    let base = s.themes.current(id).unwrap();
    park(s, id);
    unfreeze(data.path(), id, &base);
    edit_state(&dir, |v| v["mode"]["theme"]["draft"]["namespace"] = json!("autre"));
    let (back, _) = reopen(data.path(), &dir, id);
    assert_that!(back.themes.is_frozen(id, &base)).is_false();
}

#[test]
fn resume_theme_tab_held_elsewhere_is_read_only() {
    let (data, dir) = folders();
    let (s, id) = persisted_maison(data.path(), &dir);
    let scope = theme_scope_of(&s);
    park(s, id);
    let held = PackLease::acquire(&scope, id, "maison", "Ailleurs").unwrap();
    let (back, resumed) = reopen(data.path(), &dir, id);
    let expected = "Le thème maison est en cours d'édition dans l'onglet « Ailleurs » : \
                    cet onglet a rouvert à côté, en lecture seule jusqu'à la fermeture de cet onglet.";
    let args = [("theme", "maison".into()), ("tab", "Ailleurs".into())];
    let localized = back.localizer.tr_with("notice-theme-edited-elsewhere", &args);
    let snap = back.snapshot();
    let read_only = snap.theming.as_ref().map(|t| t.read_only);
    assert_that!((resumed, read_only, snap.notice)).is_equal_to((Resumed::Restored, Some(true), Some(expected)));
    assert_that!(localized.as_str()).is_equal_to(expected);
    drop(held);
}

/// Removes the adoption mark of theme `id`'s version `v` from the library under `data`.
fn unfreeze(data: &Path, id: Uuid, v: &PackVersion) {
    let marker = data.join("themes").join(id.to_string()).join("versions").join(v.hex()).join("adopted");
    std::fs::remove_file(&marker).unwrap_or_else(|e| panic!("{}: {e}", marker.display()));
}

#[test]
fn resume_unwritten_theme_tab_freezes_its_base() {
    let (data, dir) = folders();
    let (s, id) = persisted_maison(data.path(), &dir);
    let base = s.themes.current(id).unwrap();
    park(s, id);
    unfreeze(data.path(), id, &base);
    let (back, resumed) = reopen(data.path(), &dir, id);
    assert_that!((resumed, back.themes.is_frozen(id, &base))).is_equal_to((Resumed::Restored, true));
}

#[test]
fn theme_tab_state_survives_with_a_deleted_base_token() {
    let (data, dir) = folders();
    let (s, id) = persisted_maison(data.path(), &dir);
    park(s, id);
    edit_state(&dir, |v| v["draft_tokens"]["obsolete"] = json!({"size": 3}));
    let (back, resumed) = reopen(data.path(), &dir, id);
    let kept = back.deck().tokens.pointer("/obsolete/size").cloned();
    assert_that!((resumed, kept)).is_equal_to((Resumed::Restored, Some(json!(3))));
}

#[test]
fn save_theme_persists_the_tab_state() {
    let (data, dir) = folders();
    let (mut s, id) = persisted_maison(data.path(), &dir);
    accent(&mut s, "#123456");
    assert_that!(state_of(&dir)["mode"]["theme"]["session_version"].clone()).is_equal_to(Value::Null);
    // Called directly, as the agent does: `save_theme` persists the state itself, not `handle`.
    let pin = s.save_theme(false).unwrap().unwrap();
    s.flush_resume();
    let listed = s.themes.versions(id).into_iter().any(|v| v.version == pin.version);
    assert_that!((pin.id, listed)).is_equal_to((id, true));
    let saved = serde_json::to_value(&pin.version).unwrap();
    assert_that!(state_of(&dir)["mode"]["theme"]["session_version"].clone()).is_equal_to(saved);
}
