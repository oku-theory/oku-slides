use serde_json::{Value, json};
use speculoos::prelude::*;

use super::theme_edit_tests::{theme_edit, theme_tab_on};
use crate::theme::{PAPIER_THEME, PRO_THEME};

#[test]
fn rename_edited_theme_with_blank_name_keeps_the_base_name() {
    let mut s = theme_tab_on(PAPIER_THEME);
    s.rename_edited_theme("   ").unwrap();
    assert_that!(theme_edit(&s).name.as_str()).is_equal_to("Papier");
}

#[test]
fn rename_edited_theme_request_shows_the_new_name_in_theming() {
    let mut s = theme_tab_on(PRO_THEME);
    let json = s.handle(crate::Request::RenameEditedTheme { name: "Pro à moi".into() }).unwrap();
    let snap: Value = serde_json::from_str(&json).unwrap();
    assert_that!(snap.pointer("/theming/theme/name")).is_equal_to(Some(&json!("Pro à moi")));
    assert_that!(snap.pointer("/theming/theme/dirty")).is_equal_to(Some(&json!(true)));
    assert_that!(snap.pointer("/theming/theme/base/id")).is_equal_to(Some(&json!(PRO_THEME.to_string())));
    assert_that!(snap.pointer("/editing")).is_equal_to(Some(&Value::Null));
}

#[test]
fn theme_tab_embeds_the_base_version_in_its_sources() {
    let s = theme_tab_on(PRO_THEME);
    let base = theme_edit(&s).base;
    assert_that!(s.doc.sources.themes.contains_key(&base.version)).is_true();
}

/// The edited theme's name in the snapshot's `theming`.
fn snapshot_name(s: &crate::Session) -> Option<Value> {
    let snap = serde_json::to_value(s.snapshot()).unwrap();
    snap.pointer("/theming/theme/name").cloned()
}

/// The label of the step undo would take back.
fn undo_label(s: &crate::Session) -> Option<String> {
    s.sources_for_tests().replica().undo_label()
}

#[test]
fn rename_edited_theme_then_undo_restores_the_name() {
    let mut s = theme_tab_on(PRO_THEME);
    s.handle(crate::Request::RenameEditedTheme { name: "Pro à moi".into() }).unwrap();
    s.handle(crate::Request::Undo).unwrap();
    assert_that!(snapshot_name(&s)).is_equal_to(Some(json!("Pro")));
}

#[test]
fn rename_edited_theme_undo_then_redo_renames_again() {
    let mut s = theme_tab_on(PRO_THEME);
    s.handle(crate::Request::RenameEditedTheme { name: "Pro à moi".into() }).unwrap();
    s.handle(crate::Request::Undo).unwrap();
    s.handle(crate::Request::Redo).unwrap();
    assert_that!(snapshot_name(&s)).is_equal_to(Some(json!("Pro à moi")));
}

#[test]
fn rename_edited_theme_undo_step_has_its_label() {
    let mut s = theme_tab_on(PRO_THEME);
    s.rename_edited_theme("Pro à moi").unwrap();
    let expected = oku_slides_i18n::Localizer::new(Some("fr")).tr("session-undo-theme-rename");
    assert_that!(undo_label(&s)).is_equal_to(Some(expected));
}

#[test]
fn rename_edited_theme_with_an_empty_name_takes_the_base_name_in_one_step() {
    let mut s = theme_tab_on(PRO_THEME);
    s.rename_edited_theme("X").unwrap();
    s.rename_edited_theme("  ").unwrap();
    assert_that!(theme_edit(&s).name.as_str()).is_equal_to("Pro");
    s.undo().unwrap();
    assert_that!(theme_edit(&s).name.as_str()).is_equal_to("X");
}

#[test]
fn rename_edited_theme_after_set_edited_theme_extends_undoes_in_order() {
    use crate::theme_extends::theme_extends_tests::{edit_extends, theme_tab_on_builtin};
    let mut s = theme_tab_on_builtin("papier");
    s.set_edited_theme_extends(vec![crate::test_support::ns("papier"), crate::test_support::ns("pro")]).unwrap();
    s.rename_edited_theme("Papier à moi").unwrap();
    s.undo().unwrap();
    assert_that!((theme_edit(&s).name, edit_extends(&s)))
        .is_equal_to(("Papier".to_owned(), vec!["papier".to_owned(), "pro".to_owned()]));
    s.undo().unwrap();
    assert_that!(edit_extends(&s)).is_equal_to(vec!["papier".to_owned()]);
}

#[test]
fn edited_theme_name_reads_the_draft_name() {
    let mut s = theme_tab_on(PRO_THEME);
    s.rename_edited_theme("Pro à moi").unwrap();
    assert_that!(s.edited_theme_name().unwrap()).is_equal_to("Pro à moi".to_owned());
}

#[test]
fn edited_theme_name_outside_a_theme_tab_is_wrong_document() {
    let s = crate::test_support::session();
    let e = s.edited_theme_name().unwrap_err();
    assert_that!(e.kind()).is_equal_to(crate::SessionErrorKind::WrongDocument);
}

#[test]
fn rename_edited_theme_to_the_same_name_adds_no_undo_step() {
    let mut s = theme_tab_on(PRO_THEME);
    s.rename_edited_theme("Pro à moi").unwrap();
    s.rename_edited_theme(" Pro à moi ").unwrap();
    s.undo().unwrap();
    assert_that!((theme_edit(&s).name, undo_label(&s))).is_equal_to(("Pro".to_owned(), None));
}
