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
