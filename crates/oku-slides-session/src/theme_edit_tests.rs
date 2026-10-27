use oku_slides_core::{Op, ThemePin};
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::ThemeEdit;
use crate::library_kind::LibraryKind;
use crate::test_support::{fresh_dir, ns, session};
use crate::theme::{PAPIER_THEME, PRO_THEME, ThemeManifest, Themes};
use crate::{Mode, Session, SessionErrorKind};
pub(crate) fn p(path: &[&str]) -> Vec<String> {
    path.iter().map(|s| (*s).to_owned()).collect()
}

/// L'espace de noms du thème intégré `id`.
pub(crate) fn builtin_ns(id: Uuid) -> oku_slides_core::Namespace {
    Themes::builtins().iter().find(|b| b.manifest.id == id).unwrap().manifest.namespace.clone()
}

/// Une session dont le deck a le thème intégré `id`.
pub(crate) fn session_with_theme(id: Uuid) -> Session {
    let mut s = session();
    if s.deck_theme_pin().map(|r| r.id) != Some(id) {
        s.set_theme(&builtin_ns(id)).unwrap();
    }
    s
}

pub(crate) fn set_accent(s: &mut Session, accent: &str) {
    s.apply("x", vec![Op::SetToken { path: p(&["color", "accent"]), value: json!(accent) }], None).unwrap();
}

/// Ajoute à la bibliothèque de `s` le thème utilisateur `id` (jetons de Pro), courant, sous `maison`.
pub(crate) fn add_user_theme(s: &Session, id: Uuid) -> ThemePin {
    let pro = Themes::builtins().iter().find(|b| b.manifest.id == PRO_THEME).unwrap();
    let manifest = ThemeManifest {
        id,
        name: "Maison".into(),
        namespace: ns("maison"),
        extends: Vec::new(),
        description: "Le thème de la maison".into(),
        parent: None,
        created: "2026-01-01T00:00:00Z".into(),
        tokens: pro.manifest.tokens.clone(),
    };
    let files = manifest.to_files().unwrap();
    s.themes.add_version(id, &files).unwrap();
    s.themes.set_current(id, &files.version()).unwrap();
    ThemePin { id, version: files.version(), extends: Vec::new() }
}

pub(crate) fn theme_edit(s: &Session) -> ThemeEdit {
    match &s.mode {
        Mode::Theme { edit, .. } => edit.draft.clone(),
        _ => panic!("theme tab expected"),
    }
}

/// A session and the data dir it lives in, which stays as long as the session.
pub(crate) struct TabOnDir {
    /// The session; dropped before its dir.
    s: Session,
    /// The data dir: opening and saving a theme write the library there.
    _data: tempfile::TempDir,
}

impl std::ops::Deref for TabOnDir {
    type Target = Session;
    fn deref(&self) -> &Session {
        &self.s
    }
}

impl std::ops::DerefMut for TabOnDir {
    fn deref_mut(&mut self) -> &mut Session {
        &mut self.s
    }
}

/// A theme tab on the library theme `id`, in a fresh session, with its data dir.
pub(crate) fn theme_tab_on(id: Uuid) -> TabOnDir {
    let data = fresh_dir("oku-slides-theme-tab-on-");
    let mut s = crate::test_support::session_on(data.path());
    s.open_theme_here(id).unwrap();
    TabOnDir { s, _data: data }
}

/// A theme tab on a new user theme `maison` (Pro's tokens), with the data dir it lives in.
pub(crate) fn maison_tab(data: &std::path::Path) -> Session {
    let mut s = crate::test_support::session_on(data);
    let id = Uuid::new_v4();
    add_user_theme(&s, id);
    s.open_theme_here(id).unwrap();
    s
}

/// La valeur résolue par Typst au dernier rendu, au chemin `path`.
pub(crate) fn rendered_token(s: &Session, path: &str) -> Option<Value> {
    s.compiled.as_ref()?.layout().tokens.as_ref()?.pointer(path).cloned()
}

#[test]
fn theme_tab_render_takes_keys_the_edited_theme_omits_from_the_defaults() {
    // Pro does not define `script-style`; Papier (the default theme) sets
    // `display` to `serif`. Pro's draft renders over the defaults alone.
    let s = theme_tab_on(PRO_THEME);
    assert_that!(rendered_token(&s, "/script-style/display")).is_equal_to(Some(json!("sans")));
    assert_that!(rendered_token(&s, "/color/brand-ink")).is_equal_to(Some(json!("#00154e")));
}

#[test]
fn removing_a_referenced_color_is_refused() {
    let data = fresh_dir("oku-slides-theme-edit-");
    let mut s = maison_tab(data.path());
    let r = s.apply("x", vec![Op::UnsetToken { path: p(&["color", "brand-ink"]) }], None);
    assert_that!(r).is_err();
    assert_that!(r.unwrap_err().kind()).is_equal_to(SessionErrorKind::TokenInUse);
    assert_that!(s.deck().tokens.pointer("/color/brand-ink")).is_some();
}

#[test]
fn removing_a_default_color_from_the_draft_is_allowed() {
    // `accent` reste défini par les défauts : ceux qui y renvoient le trouvent.
    let mut s = theme_tab_on(PAPIER_THEME);
    s.apply("x", vec![Op::UnsetToken { path: p(&["color", "accent"]) }], None).unwrap();
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_none();
}

#[test]
fn theme_tab_invalid_token_value_is_refused() {
    let mut s = theme_tab_on(PAPIER_THEME);
    let before = s.deck().tokens.clone();
    let r = s.apply("x", vec![Op::SetToken { path: p(&["size", "body"]), value: json!(-5) }], None);
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(SessionErrorKind::InvalidTokenValue);
    assert_that!(s.deck().tokens).is_equal_to(before);
}

#[test]
fn theme_tab_token_may_refer_to_a_color_added_earlier_in_the_same_transaction() {
    let mut s = theme_tab_on(PAPIER_THEME);
    s.apply(
        "x",
        vec![
            Op::SetToken { path: p(&["color", "brique"]), value: json!("#aa3311") },
            Op::SetToken { path: p(&["color", "accent"]), value: json!("brique") },
        ],
        None,
    )
    .unwrap();
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_equal_to(Some(&json!("brique")));
}

#[test]
fn deck_override_with_invalid_color_is_refused() {
    let mut s = session_with_theme(PAPIER_THEME);
    let r = s.apply("x", vec![Op::SetToken { path: p(&["color", "accent"]), value: json!("#zzzzzz") }], None);
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(SessionErrorKind::InvalidTokenValue);
    let r = s.apply("x", vec![Op::SetToken { path: p(&["color", "accent"]), value: json!("nulle-part") }], None);
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(SessionErrorKind::InvalidTokenValue);
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_none();
}

#[test]
fn deck_override_may_name_a_color_of_its_theme() {
    let mut s = session_with_theme(PRO_THEME);
    set_accent(&mut s, "brand-ink");
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_equal_to(Some(&json!("brand-ink")));
}

#[test]
fn deck_override_of_a_non_color_token_is_not_checked() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.apply("x", vec![Op::SetToken { path: p(&["size", "body"]), value: json!(30) }], None).unwrap();
    assert_that!(s.deck().tokens.pointer("/size/body")).is_equal_to(Some(&json!(30)));
}

#[test]
fn theme_edit_view_reports_dirty_after_a_token_edit_or_a_rename() {
    let mut s = theme_tab_on(PAPIER_THEME);
    let view = s.snapshot().theming.unwrap().theme;
    assert_that!(view.dirty).is_false();
    assert_that!(view.child).is_true();
    assert_that!(view.name.as_str()).is_equal_to("Papier");
    set_accent(&mut s, "#ff0000");
    assert_that!(s.snapshot().theming.unwrap().theme.dirty).is_true();
    s.undo().unwrap();
    assert_that!(s.snapshot().theming.unwrap().theme.dirty).is_false();
    s.rename_edited_theme("Autre").unwrap();
    let view = s.snapshot().theming.unwrap().theme;
    assert_that!((view.dirty, view.name.as_str())).is_equal_to((true, "Autre"));
}

/// La révision du brouillon vue par le snapshot.
fn revision(s: &Session) -> String {
    s.snapshot().theming.unwrap().theme.revision
}

/// Applique `op` au brouillon et rend la révision d'avant et d'après.
fn revisions_around(s: &mut Session, op: Op) -> (String, String) {
    let before = revision(s);
    s.apply("x", vec![op], None).unwrap();
    (before, revision(s))
}

fn set_body_size(size: i64) -> Op {
    Op::SetToken { path: p(&["size", "body"]), value: json!(size) }
}

#[test]
fn theme_edit_revision_changes_on_each_set_token_of_the_same_token() {
    let mut s = theme_tab_on(PAPIER_THEME);
    s.apply("x", vec![set_body_size(16)], None).unwrap();
    let (r18, r20) = (revisions_around(&mut s, set_body_size(18)), revisions_around(&mut s, set_body_size(20)));
    assert_that!(r18.0).is_not_equal_to(r18.1.clone());
    assert_that!(r20.0).is_not_equal_to(r20.1);
}

#[test]
fn theme_edit_revision_changes_on_unset_token() {
    let mut s = theme_tab_on(PAPIER_THEME);
    set_accent(&mut s, "#123456");
    let (before, after) = revisions_around(&mut s, Op::UnsetToken { path: p(&["color", "accent"]) });
    assert_that!(before).is_not_equal_to(after);
}

#[test]
fn theme_edit_revision_changes_on_set_tokens() {
    let mut s = theme_tab_on(PAPIER_THEME);
    let (before, after) = revisions_around(&mut s, Op::SetTokens { tokens: json!({ "size": { "body": 30 } }) });
    assert_that!(before).is_not_equal_to(after);
}

#[test]
fn theme_edit_revision_changes_on_undo_and_redo_between_same_label_steps() {
    let mut s = theme_tab_on(PAPIER_THEME);
    s.apply("x", vec![set_body_size(18)], None).unwrap();
    s.apply("x", vec![set_body_size(20)], None).unwrap();
    let at20 = revision(&s);
    s.undo().unwrap();
    let at18 = revision(&s);
    s.redo().unwrap();
    assert_that!((at18.as_str() != at20.as_str(), revision(&s))).is_equal_to((true, at20));
}

#[test]
fn theme_edit_revision_is_stable_without_change() {
    let s = theme_tab_on(PAPIER_THEME);
    assert_that!(revision(&s)).is_equal_to(revision(&s));
}

#[test]
fn theme_tab_set_tokens_with_an_invalid_leaf_is_refused() {
    let mut s = theme_tab_on(PAPIER_THEME);
    let before = s.deck().tokens.clone();
    let bad = [
        json!({ "size": { "body": -5 } }),
        json!({ "nope": { "x": 1 } }),
        json!({ "color": { "accent": "ailleurs" } }),
    ];
    for tokens in bad {
        let r = s.apply("x", vec![Op::SetTokens { tokens }], None);
        assert_that!(r.map_err(|e| e.kind())).is_err_containing(SessionErrorKind::InvalidTokenValue);
    }
    assert_that!(s.deck().tokens).is_equal_to(before);
}

#[test]
fn theme_tab_valid_set_tokens_is_accepted() {
    let mut s = theme_tab_on(PAPIER_THEME);
    let tokens = json!({ "color": { "brique": "#aa3311", "accent": "brique" }, "size": { "body": 30 } });
    s.apply("x", vec![Op::SetTokens { tokens: tokens.clone() }], None).unwrap();
    assert_that!(s.deck().tokens).is_equal_to(tokens);
}

#[test]
fn deck_set_tokens_with_an_invalid_color_is_refused() {
    let mut s = session_with_theme(PAPIER_THEME);
    let r = s.apply("x", vec![Op::SetTokens { tokens: json!({ "color": { "accent": "#zz" } }) }], None);
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(SessionErrorKind::InvalidTokenValue);
    let r = s.apply("x", vec![Op::SetTokens { tokens: json!({ "chart": { "palette": ["nulle-part"] } }) }], None);
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(SessionErrorKind::InvalidTokenValue);
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_none();
}

#[test]
fn deck_valid_set_tokens_is_accepted_without_checking_other_leaves() {
    let mut s = session_with_theme(PRO_THEME);
    // Une taille hors des bornes du schéma : seules les couleurs sont vérifiées ici.
    let tokens = json!({ "color": { "accent": "brand-ink" }, "size": { "body": 500 } });
    s.apply("x", vec![Op::SetTokens { tokens: tokens.clone() }], None).unwrap();
    assert_that!(s.deck().tokens).is_equal_to(tokens);
}
