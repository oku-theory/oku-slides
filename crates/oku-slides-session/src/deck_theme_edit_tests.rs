use oku_slides_core::ThemePin;
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use super::TokenOp;
use crate::library_kind::LibraryKind;
use crate::pack::ESSENTIEL_PACK;
use crate::test_support::{fresh_dir, ns, opened_on};
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
    if s.deck_theme_pin().map(|r| r.id) != Some(id) {
        let namespace = Themes::builtins().iter().find(|b| b.manifest.id == id).unwrap().manifest.namespace.clone();
        s.set_theme(&namespace).unwrap();
    }
    s
}

/// L'épinglage du thème du deck.
fn deck_pin(s: &Session) -> ThemePin {
    s.deck_theme_pin().cloned().unwrap()
}

/// Ajoute à la bibliothèque de `s` le thème utilisateur « Maison » (jetons de Pro), courant, sous `maison`.
fn add_user_theme(s: &Session, id: Uuid) -> ThemePin {
    let pro = Themes::builtins().iter().find(|b| b.manifest.id == PRO_THEME).unwrap();
    let manifest = ThemeManifest {
        id,
        name: "Maison".into(),
        namespace: ns("maison"),
        extends: Vec::new(),
        description: String::new(),
        parent: None,
        created: "2026-01-01T00:00:00Z".into(),
        tokens: pro.manifest.tokens.clone(),
    };
    let files = manifest.to_files().unwrap();
    s.themes.add_version(id, &files).unwrap();
    s.themes.set_current(id, &files.version()).unwrap();
    ThemePin { id, version: files.version(), extends: Vec::new() }
}

#[test]
fn edit_theme_of_a_builtin_creates_a_child_adopted_by_the_deck() {
    let data = fresh_dir("oku-slides-deck-theme-edit-");
    let mut s = session_with_theme(&data, PAPIER_THEME);
    let edited = s.edit_theme(PAPIER_THEME, None, vec![accent("#c0392b")]).unwrap();
    assert_that!(edited.id).is_not_equal_to(PAPIER_THEME);
    assert_that!(edited.name.as_str()).is_equal_to("Papier (variante)");
    assert_that!(edited.adopted).is_true();
    let r = deck_pin(&s);
    let extends = vec![ns("papier")];
    assert_that!(r).is_equal_to(ThemePin { id: edited.id, version: edited.version.clone(), extends });
    assert_that!(s.deck().theme.clone()).is_equal_to(Some(edited.namespace.clone()));
    assert_that!(s.deck().themes.contains_key(&ns("papier"))).is_true();
    let manifest = s.theme_manifest(&r).unwrap();
    assert_that!(serde_json::Value::Object(manifest.tokens)).is_equal_to(json!({"color": {"accent": "#c0392b"}}));
}

#[test]
fn edit_theme_of_a_builtin_without_change_creates_no_child() {
    let data = fresh_dir("oku-slides-deck-theme-edit-");
    let mut s = session_with_theme(&data, PAPIER_THEME);
    let edited = s.edit_theme(PAPIER_THEME, None, vec![]).unwrap();
    assert_that!((edited.id, edited.changed)).is_equal_to((PAPIER_THEME, false));
}

#[test]
fn edit_theme_of_a_builtin_may_name_a_color_of_the_builtin() {
    let data = fresh_dir("oku-slides-deck-theme-edit-");
    let mut s = session_with_theme(&data, PRO_THEME);
    let edited = s.edit_theme(PRO_THEME, None, vec![accent("brand-ink")]).unwrap();
    assert_that!(edited.tokens["color"]["accent"].clone()).is_equal_to(json!("brand-ink"));
}

#[test]
fn edit_theme_renames_a_user_theme_in_a_new_current_version() {
    let data = fresh_dir("oku-slides-deck-theme-edit-");
    let mut s = opened_on(data.path());
    let id = Uuid::new_v4();
    let old = add_user_theme(&s, id);
    s.set_theme(&ns("maison")).unwrap();
    let edited = s.edit_theme(id, Some("Vinci".into()), vec![]).unwrap();
    assert_that!(edited.namespace.as_str()).is_equal_to("maison");
    assert_that!(edited.id).is_equal_to(id);
    assert_that!(edited.version).is_not_equal_to(old.version.clone());
    assert_that!(edited.name.as_str()).is_equal_to("Vinci");
    assert_that!(s.themes.current(id)).is_equal_to(Some(edited.version.clone()));
    let manifest = s.theme_manifest(&deck_pin(&s)).unwrap();
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
fn edit_theme_is_refused_while_editing_pack() {
    let data = fresh_dir("oku-slides-deck-theme-edit-");
    let mut opener = session_with_theme(&data, PAPIER_THEME);
    let mut s = crate::test_support_packs::pack_tab_from(&mut opener, data.path(), ESSENTIEL_PACK);
    let err = s.handle(Request::EditTheme { id: PAPIER_THEME, name: None, ops: vec![accent("#c0392b")] });
    assert_that!(err.unwrap_err().kind()).is_equal_to(SessionErrorKind::EditingPack);
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
fn edit_theme_returns_the_tokens_of_the_saved_version() {
    let data = fresh_dir("oku-slides-deck-theme-edit-tokens-");
    let mut s = session_with_theme(&data, PAPIER_THEME);
    let edited = s.edit_theme(PAPIER_THEME, None, vec![accent("#c0392b")]).unwrap();
    assert_that!(edited.tokens["color"]["accent"]).is_equal_to(json!("#c0392b"));
    assert_that!(s.theme_tokens_of(edited.id).unwrap()).is_equal_to(edited.tokens);
}

#[test]
fn find_theme_info_resolves_namespace_id_and_name_to_the_same_theme() {
    let data = fresh_dir("oku-slides-find-theme-info-");
    let s = opened_on(data.path());
    let found: Vec<_> =
        ["pro", &PRO_THEME.to_string(), "PRO"].iter().map(|w| s.find_theme_info(w).unwrap().id).collect();
    assert_that!(found).is_equal_to(vec![PRO_THEME; 3]);
}

#[test]
fn find_theme_info_of_an_unknown_theme_is_refused() {
    let data = fresh_dir("oku-slides-find-theme-info-unknown-");
    let s = opened_on(data.path());
    assert_that!(s.find_theme_info("inconnu").unwrap_err().kind()).is_equal_to(SessionErrorKind::UnknownTheme);
}
