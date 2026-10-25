use oku_slides_core::{Op, ThemePin};
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::theme_edit_tests::{add_user_theme, p, rendered_token, session_with_theme, set_accent, theme_edit};
use crate::pack::ESSENTIEL_PACK;
use crate::test_support::{fresh_dir, ns, opened_on, session};
use crate::theme::{NEUTRE_THEME, PAPIER_THEME, PRO_THEME};
use crate::{Session, SessionErrorKind};

#[test]
fn choosing_another_theme_replaces_draft_in_one_undo_step() {
    let data = fresh_dir("oku-slides-theme-edit-");
    let mut s = opened_on(data.path());
    add_user_theme(&s, Uuid::new_v4());
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.set_edited_theme(&ns("maison")).unwrap();
    assert_that!(s.deck().tokens.pointer("/font/body/0")).is_equal_to(Some(&json!("Inter")));
    s.undo().unwrap();
    assert_that!(s.deck().tokens.clone()).is_equal_to(Value::Null);
    assert_that!(theme_edit(&s).base.id).is_equal_to(PAPIER_THEME);
}

#[test]
fn choosing_another_builtin_from_an_empty_draft_is_one_undo_step() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let steps = s.doc.sources.undo_steps();
    s.set_edited_theme(&ns("pro")).unwrap();
    assert_that!(s.doc.sources.undo_steps()).is_equal_to(steps + 1);
    assert_that!(theme_edit(&s).base.id).is_equal_to(PRO_THEME);
    assert_that!(rendered_token(&s, "/color/brand-ink")).is_equal_to(Some(json!("#00154e")));
    s.undo().unwrap();
    assert_that!(theme_edit(&s).base.id).is_equal_to(PAPIER_THEME);
}

#[test]
fn undoing_builtin_choices_after_a_user_theme_brings_back_each_base_with_its_tokens() {
    let data = fresh_dir("oku-slides-theme-edit-");
    let mut s = opened_on(data.path());
    let id = Uuid::new_v4();
    add_user_theme(&s, id);
    s.set_theme(&ns("maison")).unwrap();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let own = s.deck().tokens.clone();
    s.set_edited_theme(&ns("papier")).unwrap();
    s.set_edited_theme(&ns("neutre")).unwrap();
    let base_and_tokens = |s: &Session| (theme_edit(s).base.id, theme_edit(s).child(), s.deck().tokens.clone());
    assert_that!(base_and_tokens(&s)).is_equal_to((NEUTRE_THEME, true, Value::Null));
    s.undo().unwrap();
    assert_that!(base_and_tokens(&s)).is_equal_to((PAPIER_THEME, true, Value::Null));
    s.undo().unwrap();
    assert_that!(base_and_tokens(&s)).is_equal_to((id, false, own.clone()));
    s.redo().unwrap();
    assert_that!(base_and_tokens(&s)).is_equal_to((PAPIER_THEME, true, Value::Null));
    s.redo().unwrap();
    assert_that!(base_and_tokens(&s)).is_equal_to((NEUTRE_THEME, true, Value::Null));
}

#[test]
fn a_theme_choice_never_reaches_the_stashed_deck_variables() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.set_edited_theme(&ns("pro")).unwrap();
    s.save_pack(false).unwrap();
    s.close_pack().unwrap();
    assert_that!(s.deck().vars.contains_key(super::DRAFT_SWITCH_VAR)).is_false();
}

#[test]
fn undoing_a_theme_choice_brings_back_the_previous_base() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    set_accent(&mut s, "#ff0000");
    s.set_edited_theme(&ns("pro")).unwrap();
    assert_that!(theme_edit(&s).base.id).is_equal_to(PRO_THEME);
    assert_that!(theme_edit(&s).name.as_str()).is_equal_to("Pro");
    set_accent(&mut s, "#00ff00");
    s.undo().unwrap();
    assert_that!(theme_edit(&s).base.id).is_equal_to(PRO_THEME);
    s.undo().unwrap();
    assert_that!(theme_edit(&s).base.id).is_equal_to(PAPIER_THEME);
    assert_that!(theme_edit(&s).name.as_str()).is_equal_to("Papier");
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_equal_to(Some(&json!("#ff0000")));
    s.redo().unwrap();
    assert_that!(theme_edit(&s).base.id).is_equal_to(PRO_THEME);
}

#[test]
fn set_edited_theme_outside_pack_mode_is_refused() {
    let mut s = session_with_theme(PAPIER_THEME);
    let r = s.set_edited_theme(&ns("pro"));
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(SessionErrorKind::NotEditingPack);
}

#[test]
fn set_theme_in_pack_mode_chooses_the_edited_theme() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.set_theme(&ns("pro")).unwrap();
    assert_that!(s.deck().theme).is_none();
    assert_that!(theme_edit(&s).base.id).is_equal_to(PRO_THEME);
}

#[test]
fn rename_edited_theme_with_blank_name_keeps_the_base_name() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.rename_edited_theme("   ").unwrap();
    assert_that!(theme_edit(&s).name.as_str()).is_equal_to("Papier");
}

#[test]
fn theme_requests_choose_and_rename_the_edited_theme() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let json = s.handle(crate::Request::SetEditedTheme { id: PRO_THEME }).unwrap();
    let snap: Value = serde_json::from_str(&json).unwrap();
    assert_that!(snap.pointer("/editing/theme/base/id")).is_equal_to(Some(&json!(PRO_THEME.to_string())));
    let json = s.handle(crate::Request::RenameEditedTheme { name: "Pro à moi".into() }).unwrap();
    let snap: Value = serde_json::from_str(&json).unwrap();
    assert_that!(snap.pointer("/editing/theme/name")).is_equal_to(Some(&json!("Pro à moi")));
    assert_that!(snap.pointer("/editing/theme/dirty")).is_equal_to(Some(&json!(true)));
    assert_that!(snap.pointer("/editing/pack")).is_equal_to(Some(&json!(ESSENTIEL_PACK.to_string())));
}

#[test]
fn switching_packs_in_pack_mode_keeps_the_theme_draft() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    set_accent(&mut s, "#ff0000");
    s.rename_edited_theme("Rouge").unwrap();
    s.edit_pack(crate::BASE_PACK).unwrap();
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_equal_to(Some(&json!("#ff0000")));
    assert_that!(theme_edit(&s).name.as_str()).is_equal_to("Rouge");
}

#[test]
fn set_edited_theme_embeds_the_base_version_in_the_working_sources() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.set_edited_theme(&ns("pro")).unwrap();
    let base = theme_edit(&s).base;
    assert_that!(s.doc.sources.themes.contains_key(&base.version)).is_true();
}

#[test]
fn entering_pack_mode_without_deck_theme_reads_the_default_base_from_the_sources() {
    let mut s = session();
    s.apply("x", vec![Op::SetTheme { theme: None, themes: std::collections::BTreeMap::default() }], None).unwrap();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let base = theme_edit(&s).base;
    // Embarquée par le deck de travail, ou déjà par le deck mis de côté : en mémoire.
    assert_that!(s.theme_manifest(&base).is_some()).is_true();
}

#[test]
fn pack_mode_set_theme_op_alone_chooses_the_edited_theme() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let pro = ThemePin { id: PRO_THEME, version: s.themes.current(PRO_THEME).unwrap(), extends: Vec::new() };
    s.apply("x", vec![crate::test_support::set_theme_op("pro")], None).unwrap();
    assert_that!(s.deck().theme).is_none();
    assert_that!(theme_edit(&s).base).is_equal_to(pro);
}

#[test]
fn pack_mode_set_theme_op_with_other_ops_is_refused() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let before = theme_edit(&s);
    let ops = vec![
        crate::test_support::set_theme_op("pro"),
        Op::SetToken { path: p(&["color", "accent"]), value: json!("#ff0000") },
    ];
    let r = s.apply("x", ops, None);
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(SessionErrorKind::ThemeChoiceAlone);
    assert_that!(theme_edit(&s)).is_equal_to(before);
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_not_equal_to(Some(&json!("#ff0000")));
}

#[test]
fn theme_choices_after_a_resume_in_pack_mode_stay_one_step_each_with_their_base() {
    let data = fresh_dir("oku-slides-theme-edit-resume-");
    let dir = fresh_dir("oku-slides-theme-edit-resume-state-");
    let mut s = opened_on(data.path());
    add_user_theme(&s, Uuid::new_v4());
    assert_that!(s.resume(dir.path().to_owned())).is_true();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.set_edited_theme(&ns("maison")).unwrap();
    s.set_edited_theme(&ns("pro")).unwrap();
    s.flush_resume();
    drop(s);
    let mut back = crate::test_support::session_on(data.path());
    assert_that!(back.resume(dir.path().to_owned())).is_true();
    let state = |s: &Session| (theme_edit(s).base.id, s.deck().tokens.clone());
    assert_that!(state(&back)).is_equal_to((PRO_THEME, Value::Null));
    let steps = back.doc.sources.undo_steps();
    back.set_edited_theme(&ns("neutre")).unwrap();
    assert_that!(back.doc.sources.undo_steps()).is_equal_to(steps + 1);
    assert_that!(state(&back)).is_equal_to((NEUTRE_THEME, Value::Null));
    back.undo().unwrap();
    assert_that!(state(&back)).is_equal_to((PRO_THEME, Value::Null));
    back.redo().unwrap();
    assert_that!(state(&back)).is_equal_to((NEUTRE_THEME, Value::Null));
    back.undo().unwrap();
    back.set_edited_theme(&ns("papier")).unwrap();
    assert_that!(state(&back)).is_equal_to((PAPIER_THEME, Value::Null));
    back.undo().unwrap();
    assert_that!(state(&back)).is_equal_to((PRO_THEME, Value::Null));
}
