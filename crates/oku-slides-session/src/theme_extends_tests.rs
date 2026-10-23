use std::ops::{Deref, DerefMut};

use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::ExtendsEdit;
use crate::pack::ESSENTIEL_PACK;
use crate::test_support::{add_user_theme, fresh_dir, ns, opened_on, session};
use crate::theme::ThemeManifest;
use crate::theme_edit::theme_edit_tests::{rendered_token, set_accent, theme_edit, user_pin};
use crate::{Request, Session, SessionErrorKind};

/// Une session en mode pack et le dossier de données qu'elle écrit (bibliothèque).
pub(crate) struct Draft {
    s: Session,
    _data: tempfile::TempDir,
}

impl Deref for Draft {
    type Target = Session;
    fn deref(&self) -> &Session {
        &self.s
    }
}

impl DerefMut for Draft {
    fn deref_mut(&mut self) -> &mut Session {
        &mut self.s
    }
}

/// Une session sur un dossier neuf, gardé avec elle.
fn on_fresh_data() -> Draft {
    let data = fresh_dir("oku-slides-theme-extends-");
    Draft { s: opened_on(data.path()), _data: data }
}

/// Le deck prend le thème `namespace`, puis la session passe en mode pack.
fn enter_pack_mode(d: &mut Draft, namespace: &str) {
    if d.deck().theme.as_ref() != Some(&ns(namespace)) {
        d.set_theme(&ns(namespace)).unwrap();
    }
    d.edit_pack(ESSENTIEL_PACK).unwrap();
}

/// Mode pack sur l'intégré `namespace` : le brouillon en est un enfant.
pub(crate) fn pack_mode_on_builtin(namespace: &str) -> Draft {
    let mut d = on_fresh_data();
    enter_pack_mode(&mut d, namespace);
    d
}

/// Mode pack sur le thème utilisateur `namespace` (sans jetons), qui n'étend rien.
fn pack_mode_on_user_theme(namespace: &str) -> Draft {
    let mut d = on_fresh_data();
    add_user_theme(&d.themes, Uuid::new_v4(), namespace, &[], Map::new(), None);
    enter_pack_mode(&mut d, namespace);
    d
}

/// Mode pack sur le thème utilisateur `marque` (sans jetons), qui étend `extends`.
pub(crate) fn pack_mode_on_user_theme_extending(extends: &[&str]) -> Draft {
    let mut d = on_fresh_data();
    add_user_theme(&d.themes, Uuid::new_v4(), "marque", extends, Map::new(), None);
    enter_pack_mode(&mut d, "marque");
    d
}

/// Ajoute à la bibliothèque le thème `namespace`, qui étend `extends`.
fn add_library_theme(s: &Session, namespace: &str, extends: &[&str]) {
    add_user_theme(&s.themes, Uuid::new_v4(), namespace, extends, Map::new(), None);
}

/// La liste `extends` du brouillon.
pub(crate) fn edit_extends(s: &Session) -> Vec<String> {
    theme_edit(s).extends.iter().map(|n| n.as_str().to_owned()).collect()
}

/// L'accent résolu au dernier rendu, sous le brouillon.
fn draft_accent(s: &Session) -> String {
    rendered_token(s, "/color/accent").and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default()
}

/// L'accent posé dans le brouillon lui-même.
fn edited_accent(s: &Session) -> Option<String> {
    s.deck().tokens.pointer("/color/accent").and_then(Value::as_str).map(str::to_owned)
}

/// Enregistre le pack, puis relit la version courante du thème `namespace`.
fn save_and_read_theme(s: &mut Session, namespace: &str) -> ThemeManifest {
    s.save_pack(false).unwrap();
    s.themes.by_namespace(&ns(namespace)).unwrap().manifest
}

/// Enregistre le pack, puis relit le thème que le deck a adopté.
fn save_and_read_new_theme(s: &mut Session) -> ThemeManifest {
    s.save_pack(false).unwrap();
    s.theme_manifest(&user_pin(s)).unwrap()
}

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

// Papier et Pro posent color.accent à #3f6b5e et #00bef4 : le dernier parent l'emporte.
#[test]
fn set_edited_theme_extends_reorder_then_undo_redo_follow() {
    let mut s = pack_mode_on_user_theme_extending(&["papier", "pro"]);
    assert_that!(draft_accent(&s)).is_equal_to("#00bef4".to_owned());
    s.set_edited_theme_extends(vec![ns("pro"), ns("papier")]).unwrap();
    assert_that!((edit_extends(&s), draft_accent(&s))).is_equal_to((strings(&["pro", "papier"]), "#3f6b5e".to_owned()));
    s.undo().unwrap();
    assert_that!((edit_extends(&s), draft_accent(&s))).is_equal_to((strings(&["papier", "pro"]), "#00bef4".to_owned()));
    s.redo().unwrap();
    assert_that!(edit_extends(&s)).is_equal_to(strings(&["pro", "papier"]));
}

#[test]
fn set_edited_theme_extends_two_edits_two_undos_return_to_start() {
    let mut s = pack_mode_on_builtin("papier");
    s.edit_theme_extends(ExtendsEdit::Add { theme: ns("pro") }).unwrap();
    s.edit_theme_extends(ExtendsEdit::Up { theme: ns("pro") }).unwrap();
    assert_that!(edit_extends(&s)).is_equal_to(strings(&["pro", "papier"]));
    s.undo().unwrap();
    s.undo().unwrap();
    assert_that!(edit_extends(&s)).is_equal_to(strings(&["papier"]));
}

#[test]
fn set_edited_theme_extends_after_token_edit_undo_keeps_the_token() {
    let mut s = pack_mode_on_builtin("papier");
    set_accent(&mut s, "#ff0000");
    s.edit_theme_extends(ExtendsEdit::Add { theme: ns("pro") }).unwrap();
    s.undo().unwrap();
    assert_that!((edit_extends(&s), edited_accent(&s))).is_equal_to((strings(&["papier"]), Some("#ff0000".to_owned())));
}

#[test]
fn set_edited_theme_extends_own_namespace_is_a_cycle() {
    let mut s = pack_mode_on_user_theme("marque");
    let err = s.set_edited_theme_extends(vec![ns("marque")]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ThemeCycle);
    assert_that!(edit_extends(&s)).is_empty();
}

#[test]
fn set_edited_theme_extends_parent_reaching_draft_is_a_cycle() {
    let mut s = pack_mode_on_user_theme("marque");
    add_library_theme(&s, "sombre", &["marque"]);
    let err = s.set_edited_theme_extends(vec![ns("sombre")]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ThemeCycle);
}

#[test]
fn set_edited_theme_extends_child_listing_its_base_and_pro_is_accepted() {
    let mut s = pack_mode_on_builtin("papier");
    assert_that!(s.set_edited_theme_extends(vec![ns("papier"), ns("pro")])).is_ok();
    assert_that!(draft_accent(&s)).is_equal_to("#00bef4".to_owned());
}

#[rstest]
#[case::duplicate(vec!["papier", "papier"], SessionErrorKind::ThemeExtendsDuplicate)]
#[case::unknown(vec!["nowhere"], SessionErrorKind::UnknownTheme)]
fn set_edited_theme_extends_bad_list_is_refused(#[case] list: Vec<&str>, #[case] expected: SessionErrorKind) {
    let mut s = pack_mode_on_builtin("papier");
    let err = s.set_edited_theme_extends(list.into_iter().map(ns).collect()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(expected);
    assert_that!(err.localize(&Localizer::new(Some("fr")))).is_not_equal_to(err.localize(&Localizer::new(Some("en"))));
}

#[test]
fn set_edited_theme_extends_longer_than_the_library_is_refused() {
    let mut s = pack_mode_on_builtin("papier");
    let count = s.themes.themes().len();
    let list = std::iter::repeat_n(ns("pro"), count + 5).collect();
    assert_that!(s.set_edited_theme_extends(list).unwrap_err().kind())
        .is_equal_to(SessionErrorKind::ThemeExtendsDuplicate);
}

#[test]
fn set_edited_theme_extends_outside_pack_mode_is_refused() {
    let mut s = session();
    assert_that!(s.set_edited_theme_extends(vec![]).unwrap_err().kind()).is_equal_to(SessionErrorKind::NotEditingPack);
}

#[test]
fn set_edited_theme_extends_empty_list_is_accepted() {
    let mut s = pack_mode_on_user_theme_extending(&["papier"]);
    assert_that!(s.set_edited_theme_extends(vec![])).is_ok();
    assert_that!(edit_extends(&s)).is_empty();
}

#[test]
fn edit_theme_extends_first_up_is_refused() {
    let mut s = pack_mode_on_builtin("papier");
    let err = s.edit_theme_extends(ExtendsEdit::Up { theme: ns("papier") }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ThemeExtendsMove);
    assert_that!(err.localize(&Localizer::new(Some("fr")))).is_not_equal_to(err.localize(&Localizer::new(Some("en"))));
}

#[test]
fn edit_theme_extends_last_down_is_refused() {
    let mut s = pack_mode_on_builtin("papier");
    let err = s.edit_theme_extends(ExtendsEdit::Down { theme: ns("papier") }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ThemeExtendsMove);
}

#[test]
fn edit_theme_extends_remove_then_down_follow_the_current_list() {
    let mut s = pack_mode_on_user_theme_extending(&["papier", "pro", "neutre"]);
    s.edit_theme_extends(ExtendsEdit::Remove { theme: ns("pro") }).unwrap();
    s.edit_theme_extends(ExtendsEdit::Down { theme: ns("papier") }).unwrap();
    assert_that!(edit_extends(&s)).is_equal_to(strings(&["neutre", "papier"]));
}

#[test]
fn edit_theme_extends_request_reads_the_ui_shape() {
    let mut s = pack_mode_on_builtin("papier");
    let req: Request =
        serde_json::from_value(json!({"cmd": "edit_theme_extends", "edit": "add", "theme": "pro"})).unwrap();
    s.handle(req).unwrap();
    assert_that!(edit_extends(&s)).is_equal_to(strings(&["papier", "pro"]));
}

#[test]
fn set_edited_theme_extends_request_reads_a_list() {
    let mut s = pack_mode_on_builtin("papier");
    let req: Request = serde_json::from_value(json!({"cmd": "set_edited_theme_extends", "extends": ["pro"]})).unwrap();
    s.handle(req).unwrap();
    assert_that!(edit_extends(&s)).is_equal_to(strings(&["pro"]));
}

#[test]
fn saved_extends_only_change_writes_a_version() {
    let mut s = pack_mode_on_user_theme_extending(&["papier", "pro"]);
    s.set_edited_theme_extends(vec![ns("pro"), ns("papier")]).unwrap();
    assert_that!(s.editing_view().unwrap().theme.dirty).is_true();
    let saved = save_and_read_theme(&mut s, "marque");
    assert_that!(saved.extends).is_equal_to(vec![ns("pro"), ns("papier")]);
}

#[test]
fn saved_child_keeps_edited_extends() {
    let mut s = pack_mode_on_builtin("papier");
    s.set_edited_theme_extends(vec![ns("papier"), ns("pro")]).unwrap();
    set_accent(&mut s, "#ff0000");
    let saved = save_and_read_new_theme(&mut s);
    assert_that!(saved.extends).is_equal_to(vec![ns("papier"), ns("pro")]);
}

#[test]
fn editing_view_exposes_base_namespace() {
    let s = pack_mode_on_builtin("pro");
    assert_that!(s.editing_view().unwrap().theme.namespace.as_str()).is_equal_to("pro");
}

#[test]
fn editing_view_names_each_parent() {
    let s = pack_mode_on_user_theme_extending(&["papier", "pro"]);
    let parents: Vec<(String, String)> =
        s.editing_view().unwrap().theme.extends.into_iter().map(|p| (p.namespace, p.name)).collect();
    assert_that!(parents)
        .is_equal_to(vec![("papier".to_owned(), "Papier".to_owned()), ("pro".to_owned(), "Pro".to_owned())]);
}
