use oku_slides_core::Op;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::library_kind::LibraryKind;
use crate::pack::ESSENTIEL_PACK;
use crate::pack_mode::layout_slides;
use crate::test_support::{add_user_theme, builtin, fresh_dir, session_on};
use crate::theme::{PAPIER_THEME, PRO_THEME, Themes};
use crate::theme_edit::theme_edit_tests::{p, rendered_token};
use crate::{Mode, Session, SessionErrorKind, TabDocument};

/// The tokens of the built-in Pro, for a user theme that has its own.
pub(crate) fn pro_tokens() -> Map<String, Value> {
    Themes::builtins().iter().find(|b| b.manifest.id == PRO_THEME).unwrap().manifest.tokens.clone()
}

/// A fresh session on `data` whose library has the user theme `maison` (Pro's tokens); its id.
pub(crate) fn with_maison(data: &std::path::Path) -> (Session, Uuid) {
    let s = session_on(data);
    let id = Uuid::new_v4();
    add_user_theme(&s.themes, id, "maison", &[], pro_tokens(), None);
    (s, id)
}

/// A theme tab on `theme`, in a fresh session on `data`.
pub(crate) fn theme_tab(data: &std::path::Path, theme: Uuid) -> Session {
    let mut s = session_on(data);
    s.open_theme_here(theme).unwrap();
    s
}

/// The theme tab's draft.
pub(crate) fn draft_of(s: &Session) -> crate::ThemeEdit {
    match &s.mode {
        Mode::Theme { edit, .. } => edit.draft.clone(),
        _ => panic!("theme tab expected"),
    }
}

#[test]
fn open_theme_here_lists_essentiel_layouts_as_specimens() {
    let data = fresh_dir("oku-slides-theme-tab-");
    let s = theme_tab(data.path(), PAPIER_THEME);
    let essentiel = builtin(ESSENTIEL_PACK);
    let expected: Vec<String> =
        layout_slides(&essentiel.manifest, &essentiel.files, &s.localizer).into_iter().map(|s| s.id).collect();
    let ids: Vec<String> = s.snapshot().slides.iter().map(|sl| sl.id.to_owned()).collect();
    assert_that!(ids).is_equal_to(expected);
}

#[test]
fn theme_session_renders_specimens_with_draft() {
    let data = fresh_dir("oku-slides-theme-tab-");
    let mut s = theme_tab(data.path(), PAPIER_THEME);
    assert_that!(s.svg("layout-title").unwrap().to_lowercase().contains("#ff0000")).is_false();
    let op = Op::SetToken { path: p(&["color", "accent"]), value: json!("#ff0000") };
    s.handle(crate::Request::Apply { label: "x".into(), ops: vec![op], merge: None }).unwrap();
    assert_that!(s.svg("layout-title").unwrap().to_lowercase().contains("#ff0000")).is_true();
}

#[test]
fn open_theme_here_on_builtin_starts_an_empty_child_draft() {
    let data = fresh_dir("oku-slides-theme-tab-");
    let s = theme_tab(data.path(), PAPIER_THEME);
    assert_that!(s.deck().theme).is_none();
    assert_that!(s.deck().tokens.clone()).is_equal_to(Value::Null);
    assert_that!(draft_of(&s).child()).is_true();
    assert_that!(rendered_token(&s, "/color/accent")).is_equal_to(Some(json!("#3f6b5e")));
}

#[test]
fn open_theme_here_on_user_theme_loads_its_own_tokens() {
    let data = fresh_dir("oku-slides-theme-tab-");
    let (mut s, maison) = with_maison(data.path());
    s.open_theme_here(maison).unwrap();
    assert_that!(draft_of(&s).child()).is_false();
    assert_that!(s.deck().tokens.pointer("/color/brand-ink")).is_equal_to(Some(&json!("#00154e")));
}

#[test]
fn open_theme_here_on_user_theme_parents_writes_on_the_opened_version() {
    let data = fresh_dir("oku-slides-theme-tab-");
    let (mut s, maison) = with_maison(data.path());
    let current = s.themes.current_version(maison).unwrap().version;
    s.open_theme_here(maison).unwrap();
    let Mode::Theme { edit, .. } = &s.mode else { panic!("theme tab expected") };
    assert_that!(edit.parent).is_equal_to(Some(current));
    assert_that!(edit.session_version).is_none();
}

#[test]
fn open_theme_here_on_builtin_has_no_parent() {
    let data = fresh_dir("oku-slides-theme-tab-");
    let s = theme_tab(data.path(), PAPIER_THEME);
    let Mode::Theme { edit, .. } = &s.mode else { panic!("theme tab expected") };
    assert_that!(edit.parent).is_none();
}

#[test]
fn open_theme_here_pins_only_base_and_essentiel() {
    let data = fresh_dir("oku-slides-theme-tab-");
    let s = theme_tab(data.path(), PAPIER_THEME);
    let pins: Vec<String> = s.deck().all_pins().map(|(_, ns, _)| ns.to_string()).collect();
    assert_that!(pins).is_equal_to(vec!["base".to_owned(), "essentiel".to_owned()]);
}

#[test]
fn open_theme_here_on_an_unknown_theme_is_refused() {
    let data = fresh_dir("oku-slides-theme-tab-");
    let mut s = session_on(data.path());
    let err = s.open_theme_here(Uuid::new_v4()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownTheme);
    assert_that!(s.document_kind()).is_equal_to(crate::DocumentKind::Deck);
}

#[test]
fn open_theme_here_on_a_used_session_is_refused() {
    let data = fresh_dir("oku-slides-theme-tab-");
    let mut s = session_on(data.path());
    s.apply("x", vec![crate::test_support::rename_title("Autre")], None).unwrap();
    let err = s.open_theme_here(PAPIER_THEME).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackTabNotFresh);
}

#[test]
fn theme_tab_info_names_the_theme_and_its_document() {
    let data = fresh_dir("oku-slides-theme-tab-");
    let (mut s, maison) = with_maison(data.path());
    s.open_theme_here(maison).unwrap();
    let info = s.tab_info();
    assert_that!(info.title.as_str()).is_equal_to("maison");
    assert_that!(info.document).is_equal_to(TabDocument::Theme { theme: maison });
    assert_that!(info.pristine).is_false();
    assert_that!(info.theme).is_equal_to(Some(maison));
}

#[test]
fn theme_tab_snapshot_shows_the_draft_in_theming() {
    let data = fresh_dir("oku-slides-theme-tab-");
    let s = theme_tab(data.path(), PAPIER_THEME);
    let snap = s.snapshot();
    let theming = snap.theming.as_ref().unwrap();
    assert_that!(theming.theme.base.id).is_equal_to(PAPIER_THEME);
    assert_that!(theming.read_only).is_false();
    assert_that!(theming.leased).is_true();
    assert_that!(theming.display_name.as_str()).is_equal_to("Papier");
    assert_that!(snap.editing.is_none()).is_true();
}

#[test]
fn deck_snapshot_has_no_theming() {
    let data = fresh_dir("oku-slides-theme-tab-");
    let s = session_on(data.path());
    assert_that!(s.snapshot().theming.is_none()).is_true();
}
