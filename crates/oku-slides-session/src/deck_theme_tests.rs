use oku_slides_core::{Op, ThemeRef};
use serde_json::{Map, Value};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::library_kind::LibraryKind;
use crate::test_support::{fresh_dir, session, session_on};
use crate::theme::{DEFAULT_THEME, PAPIER_THEME, PRO_THEME, ThemeManifest, Themes};
use crate::{Request, Session};

/// Les jetons de Pro, l'accent remplacé par `accent`.
fn tokens_with_accent(accent: &str) -> Map<String, Value> {
    let pro = Themes::builtins().iter().find(|b| b.manifest.id == PRO_THEME).unwrap();
    let mut tokens = pro.manifest.tokens.clone();
    tokens["color"]["accent"] = Value::String(accent.into());
    tokens
}

/// Ajoute à la bibliothèque de `s` une version du thème `id`, courante ; sa référence.
fn add_user_theme(s: &Session, id: Uuid, accent: &str, parent: Option<&ThemeRef>, created: &str) -> ThemeRef {
    let manifest = ThemeManifest {
        id,
        name: "Rouge".into(),
        description: String::new(),
        parent: parent.map(|p| p.version.clone()),
        created: created.into(),
        scope: oku_slides_core::PackScope::Library,
        tokens: tokens_with_accent(accent),
    };
    let files = manifest.to_files().unwrap();
    s.themes.add_version(id, &files).unwrap();
    s.themes.set_current(id, &files.version()).unwrap();
    ThemeRef { id, version: files.version() }
}

fn accent(s: &Session) -> Option<String> {
    s.appearance.slide.get("accent").cloned()
}

#[test]
fn new_deck_adopts_current_default_theme() {
    let data = fresh_dir("oku-slides-deck-theme-");
    let s = session_on(data.path());
    let r = s.deck().theme.clone().unwrap();
    assert_that!(r.id).is_equal_to(DEFAULT_THEME);
    assert_that!(s.doc.sources.themes.contains_key(&r.version)).is_true();
    assert_that!(s.tab_info().pristine).is_true();
}

#[test]
fn json_deck_without_theme_adopts_the_default_theme() {
    let s = session();
    let r = s.deck().theme.clone().unwrap();
    assert_that!(r.id).is_equal_to(PAPIER_THEME);
    assert_that!(s.doc.sources.themes.len()).is_equal_to(1);
}

#[test]
fn set_theme_embeds_and_is_undoable() {
    let mut s = session();
    s.set_theme(PRO_THEME).unwrap();
    let r = s.deck().theme.clone().unwrap();
    assert_that!(r.id).is_equal_to(PRO_THEME);
    assert_that!(s.doc.sources.themes.contains_key(&r.version)).is_true();
    s.undo().unwrap();
    assert_that!(s.deck().theme.as_ref().map(|t| t.id)).is_equal_to(Some(DEFAULT_THEME));
}

#[test]
fn set_theme_changes_the_rendered_accent() {
    let mut s = session();
    let before = accent(&s);
    s.set_theme(PRO_THEME).unwrap();
    assert_that!(accent(&s)).is_equal_to(Some("#00bef4".to_owned()));
    s.undo().unwrap();
    assert_that!(accent(&s)).is_equal_to(before);
}

#[test]
fn set_theme_request_sets_the_theme() {
    let mut s = session();
    let snap: Value = serde_json::from_str(&s.handle(Request::SetTheme { id: PRO_THEME }).unwrap()).unwrap();
    assert_that!(snap["theme"]["id"].as_str()).is_equal_to(Some(PRO_THEME.to_string().as_str()));
    let ids: Vec<&str> = snap["themes"].as_array().unwrap().iter().filter_map(|t| t["id"].as_str()).collect();
    assert_that!(ids).contains(PRO_THEME.to_string().as_str());
}

#[test]
fn deck_renders_with_embedded_version_not_library_current() {
    let data = fresh_dir("oku-slides-deck-theme-");
    let mut s = crate::test_support::opened_on(data.path());
    let id = Uuid::new_v4();
    let red = add_user_theme(&s, id, "#ff0000", None, "2026-10-01T10:00:00Z");
    s.set_theme(id).unwrap();
    assert_that!(accent(&s)).is_equal_to(Some("#ff0000".to_owned()));
    let blue = add_user_theme(&s, id, "#0000ff", Some(&red), "2026-10-02T10:00:00Z");
    assert_that!(s.themes.current(id)).is_equal_to(Some(blue.version));
    s.rerender().unwrap();
    assert_that!(s.deck().theme.clone()).is_equal_to(Some(red));
    assert_that!(accent(&s)).is_equal_to(Some("#ff0000".to_owned()));
}

#[test]
fn set_theme_unknown_id_is_refused() {
    let mut s = session();
    assert_that!(s.set_theme(Uuid::new_v4())).is_err();
    assert_that!(s.deck().theme.as_ref().map(|t| t.id)).is_equal_to(Some(DEFAULT_THEME));
}

#[test]
fn set_theme_op_to_null_references_the_default_theme_explicitly() {
    let mut s = session();
    s.set_theme(PRO_THEME).unwrap();
    s.apply("Défaut", vec![Op::SetTheme { theme: None }], None).unwrap();
    assert_that!(s.deck().theme.as_ref().map(|t| t.id)).is_equal_to(Some(DEFAULT_THEME));
}

#[test]
fn set_theme_op_with_an_unknown_version_returns_theme_version_missing() {
    let mut s = session();
    let r = ThemeRef { id: PRO_THEME, version: oku_slides_core::PackVersion::from_hex(&"0".repeat(64)).unwrap() };
    let e = s.apply("Thème", vec![Op::SetTheme { theme: Some(r) }], None).unwrap_err();
    assert_that!(e.kind()).is_equal_to(crate::SessionErrorKind::ThemeVersionMissing);
}

#[test]
fn deck_theme_with_a_missing_version_renders_with_the_default_theme() {
    let mut s = session();
    let papier = accent(&s);
    let mut deck = s.deck().clone();
    deck.theme =
        Some(ThemeRef { id: PRO_THEME, version: oku_slides_core::PackVersion::from_hex(&"1".repeat(64)).unwrap() });
    s.doc.sources.apply_silent(deck).unwrap();
    s.rerender().unwrap();
    assert_that!(accent(&s)).is_equal_to(papier);
}

#[test]
fn snapshot_theme_name_is_the_embedded_manifest_name() {
    let name = |s: &Session| serde_json::to_value(s.snapshot()).unwrap()["theme_name"].as_str().map(str::to_owned);
    let mut s = session();
    assert_that!(name(&s)).is_equal_to(Some("Papier".to_owned()));
    s.set_theme(PRO_THEME).unwrap();
    assert_that!(name(&s)).is_equal_to(Some("Pro".to_owned()));
}

#[test]
fn first_bad_embedded_sighting_is_true_once_per_version() {
    let r = ThemeRef { id: Uuid::new_v4(), version: Themes::builtins()[0].version.clone() };
    assert_that!(super::first_bad_embedded_sighting(&r)).is_true();
    assert_that!(super::first_bad_embedded_sighting(&r)).is_false();
}
