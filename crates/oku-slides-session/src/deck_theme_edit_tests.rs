use oku_slides_core::ThemeRef;
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use super::TokenOp;
use crate::library_kind::LibraryKind;
use crate::pack::DEFAULT_PACK;
use crate::pack::PackScope;
use crate::test_support::{adopt_deck_only_theme, fresh_dir, opened_on};
use crate::theme::{PAPIER_THEME, PRO_THEME, ThemeManifest, Themes};
use crate::{Request, Session, SessionErrorKind};

fn p(path: &[&str]) -> Vec<String> {
    path.iter().map(|s| (*s).to_owned()).collect()
}

fn accent(value: &str) -> TokenOp {
    TokenOp::SetToken { path: p(&["color", "accent"]), value: json!(value) }
}

/// Une session sur un dossier de données gardé, son deck sur le thème `id`.
fn session_with_theme(data: &tempfile::TempDir, id: Uuid) -> Session {
    let mut s = opened_on(data.path());
    if s.deck().theme.as_ref().map(|r| r.id) != Some(id) {
        s.set_theme(id).unwrap();
    }
    s
}

/// Ajoute à la bibliothèque de `s` le thème utilisateur « Maison » (jetons de Pro), courant.
fn add_user_theme(s: &Session, id: Uuid) -> ThemeRef {
    let pro = Themes::builtins().iter().find(|b| b.manifest.id == PRO_THEME).unwrap();
    let manifest = ThemeManifest {
        id,
        name: "Maison".into(),
        description: String::new(),
        parent: None,
        created: "2026-01-01T00:00:00Z".into(),
        scope: oku_slides_core::PackScope::Library,
        tokens: pro.manifest.tokens.clone(),
    };
    let files = manifest.to_files().unwrap();
    s.themes.add_version(id, &files).unwrap();
    s.themes.set_current(id, &files.version()).unwrap();
    ThemeRef { id, version: files.version() }
}

#[test]
fn edit_theme_of_a_builtin_creates_a_copy_adopted_by_the_deck() {
    let data = fresh_dir("oku-slides-deck-theme-edit-");
    let mut s = session_with_theme(&data, PAPIER_THEME);
    let edited = s.edit_theme(PAPIER_THEME, None, vec![accent("#c0392b")]).unwrap();
    assert_that!(edited.id).is_not_equal_to(PAPIER_THEME);
    assert_that!(edited.name.as_str()).is_equal_to("Papier (copie)");
    assert_that!(edited.adopted).is_true();
    let r = s.deck().theme.clone().unwrap();
    assert_that!(r).is_equal_to(ThemeRef { id: edited.id, version: edited.version.clone() });
    let manifest = s.theme_manifest(&r).unwrap();
    assert_that!(manifest.tokens["color"]["accent"]).is_equal_to(json!("#c0392b"));
}

#[test]
fn edit_theme_renames_a_user_theme_in_a_new_current_version() {
    let data = fresh_dir("oku-slides-deck-theme-edit-");
    let mut s = opened_on(data.path());
    let id = Uuid::new_v4();
    let old = add_user_theme(&s, id);
    s.set_theme(id).unwrap();
    let edited = s.edit_theme(id, Some("Vinci".into()), vec![]).unwrap();
    assert_that!(edited.id).is_equal_to(id);
    assert_that!(edited.version).is_not_equal_to(old.version.clone());
    assert_that!(edited.name.as_str()).is_equal_to("Vinci");
    assert_that!(s.themes.current(id)).is_equal_to(Some(edited.version.clone()));
    let manifest = s.theme_manifest(s.deck().theme.as_ref().unwrap()).unwrap();
    assert_that!(manifest.name.as_str()).is_equal_to("Vinci");
    assert_that!(manifest.parent).is_equal_to(Some(old.version));
}

#[test]
fn edit_theme_of_another_theme_leaves_the_deck_theme() {
    let data = fresh_dir("oku-slides-deck-theme-edit-");
    let mut s = session_with_theme(&data, PAPIER_THEME);
    let before = s.deck().theme.clone();
    let id = Uuid::new_v4();
    add_user_theme(&s, id);
    let edited = s.edit_theme(id, None, vec![accent("#c0392b")]).unwrap();
    assert_that!(edited.adopted).is_false();
    assert_that!(s.deck().theme.clone()).is_equal_to(before);
    assert_that!(s.themes.current(id)).is_equal_to(Some(edited.version));
}

#[test]
fn edit_theme_without_change_adds_no_version() {
    let data = fresh_dir("oku-slides-deck-theme-edit-");
    let mut s = session_with_theme(&data, PAPIER_THEME);
    let themes = s.themes.themes().len();
    let edited = s.edit_theme(PAPIER_THEME, Some("  ".into()), vec![]).unwrap();
    assert_that!(edited.changed).is_false();
    assert_that!(edited.adopted).is_false();
    assert_that!(edited.id).is_equal_to(PAPIER_THEME);
    assert_that!(s.themes.themes()).has_length(themes);
}

#[test]
fn edit_theme_with_an_invalid_token_is_refused_without_a_version() {
    let data = fresh_dir("oku-slides-deck-theme-edit-");
    let mut s = session_with_theme(&data, PAPIER_THEME);
    let themes = s.themes.themes().len();
    let err = s.edit_theme(PAPIER_THEME, None, vec![accent("pas une couleur")]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InvalidTokenValue);
    assert_that!(s.themes.themes()).has_length(themes);
}

#[test]
fn edit_theme_is_refused_while_editing_templates() {
    let data = fresh_dir("oku-slides-deck-theme-edit-");
    let mut s = session_with_theme(&data, PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    let err = s.handle(Request::EditTheme { id: PAPIER_THEME, name: None, ops: vec![accent("#c0392b")] });
    assert_that!(err.unwrap_err().kind()).is_equal_to(SessionErrorKind::EditingTemplates);
}

#[test]
fn edit_theme_undo_brings_back_the_previous_deck_theme() {
    let data = fresh_dir("oku-slides-deck-theme-edit-");
    let mut s = session_with_theme(&data, PAPIER_THEME);
    let before = s.deck().theme.clone();
    s.edit_theme(PAPIER_THEME, Some("Vinci".into()), vec![]).unwrap();
    s.undo().unwrap();
    assert_that!(s.deck().theme.clone()).is_equal_to(before);
}

#[test]
fn edit_theme_request_answers_the_edited_theme_and_lists_it() {
    let data = fresh_dir("oku-slides-deck-theme-edit-");
    let mut s = session_with_theme(&data, PAPIER_THEME);
    let out = s.handle(Request::EditTheme { id: PAPIER_THEME, name: Some("Vinci".into()), ops: vec![] }).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_that!(v["name"]).is_equal_to(json!("Vinci"));
    assert_that!(v["adopted"]).is_equal_to(json!(true));
    let listed = s.snapshot().themes.iter().any(|t| t.name == "Vinci");
    assert_that!(listed).is_true();
}

#[test]
fn edit_theme_of_a_deck_only_theme_embeds_a_deck_only_version_and_leaves_the_library_unchanged() {
    let data = fresh_dir("oku-slides-deck-theme-edit-only-");
    let mut s = opened_on(data.path());
    let before = adopt_deck_only_theme(&mut s);
    let themes = s.themes.themes();
    let info = s.theme_info(before.id).unwrap();
    assert_that!((info.version.clone(), info.deck_only)).is_equal_to((before.version.clone(), true));
    let edited = s.edit_theme(before.id, None, vec![accent("#c0392b")]).unwrap();
    assert_that!((edited.id, edited.adopted, edited.changed)).is_equal_to((before.id, true, true));
    assert_that!(s.themes.themes()).is_equal_to(&themes);
    assert_that!(s.themes.versions(before.id).is_empty()).is_true();
    let r = s.deck().theme.clone().unwrap();
    assert_that!(r.version.clone()).is_equal_to(&edited.version);
    let manifest = s.theme_manifest(&r).unwrap();
    assert_that!(manifest.scope).is_equal_to(PackScope::Deck);
    assert_that!(manifest.parent).is_equal_to(Some(before.version.clone()));
    assert_that!(manifest.tokens["color"]["accent"]).is_equal_to(json!("#c0392b"));
    s.undo().unwrap();
    assert_that!(s.deck().theme.clone()).is_equal_to(Some(before));
}

#[test]
fn theme_info_of_a_library_theme_is_not_deck_only() {
    let data = fresh_dir("oku-slides-deck-theme-info-");
    let s = opened_on(data.path());
    assert_that!(s.theme_info(crate::PAPIER_THEME).unwrap().deck_only).is_false();
}

#[test]
fn theme_tokens_of_a_deck_only_theme_are_those_of_its_deck_version() {
    let data = fresh_dir("oku-slides-deck-theme-tokens-");
    let mut s = opened_on(data.path());
    let r = adopt_deck_only_theme(&mut s);
    let manifest = s.theme_manifest(&r).unwrap();
    assert_that!(s.theme_tokens_of(r.id).unwrap()).is_equal_to(manifest.tokens);
}

#[test]
fn edit_theme_returns_the_tokens_of_the_saved_version() {
    let data = fresh_dir("oku-slides-deck-theme-edit-tokens-");
    let mut s = session_with_theme(&data, PAPIER_THEME);
    let edited = s.edit_theme(PAPIER_THEME, None, vec![accent("#c0392b")]).unwrap();
    assert_that!(edited.tokens["color"]["accent"]).is_equal_to(json!("#c0392b"));
    assert_that!(s.theme_tokens_of(edited.id).unwrap()).is_equal_to(edited.tokens);
}

#[test]
fn edit_theme_of_a_deck_only_theme_the_deck_no_longer_uses_readopts_its_new_version() {
    let data = fresh_dir("oku-slides-deck-theme-edit-only-unused-");
    let mut s = opened_on(data.path());
    let before = adopt_deck_only_theme(&mut s);
    s.set_theme(PAPIER_THEME).unwrap();
    assert_that!(s.theme_info(before.id).unwrap().version).is_equal_to(&before.version);
    let edited = s.edit_theme(before.id, None, vec![accent("#c0392b")]).unwrap();
    assert_that!((edited.adopted, edited.changed)).is_equal_to((true, true));
    assert_that!(s.deck().theme.clone()).is_equal_to(Some(ThemeRef { id: before.id, version: edited.version }));
}
