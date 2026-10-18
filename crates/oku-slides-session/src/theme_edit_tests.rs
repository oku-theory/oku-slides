use oku_slides_core::{Op, ThemeRef};
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::ThemeEdit;
use crate::library_kind::LibraryKind;
use crate::pack::DEFAULT_PACK;
use crate::test_support::{adopt_deck_only_theme, fresh_dir, opened_on, session};
use crate::theme::{PAPIER_THEME, PRO_THEME, ThemeLibrary, ThemeManifest, Themes};
use crate::{Mode, Session, SessionErrorKind};

fn p(path: &[&str]) -> Vec<String> {
    path.iter().map(|s| (*s).to_owned()).collect()
}

/// Une session dont le deck a le thème intégré `id`.
fn session_with_theme(id: Uuid) -> Session {
    let mut s = session();
    if s.deck().theme.as_ref().map(|r| r.id) != Some(id) {
        s.set_theme(id).unwrap();
    }
    s
}

fn set_accent(s: &mut Session, accent: &str) {
    s.apply("x", vec![Op::SetToken { path: p(&["color", "accent"]), value: json!(accent) }], None).unwrap();
}

/// Ajoute à la bibliothèque de `s` le thème utilisateur `id` (jetons de Pro), courant.
fn add_user_theme(s: &Session, id: Uuid) -> ThemeRef {
    let pro = Themes::builtins().iter().find(|b| b.manifest.id == PRO_THEME).unwrap();
    let manifest = ThemeManifest {
        id,
        name: "Maison".into(),
        description: "Le thème de la maison".into(),
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

fn theme_edit(s: &Session) -> ThemeEdit {
    match &s.mode {
        Mode::Templates { theme, .. } => theme.clone(),
        _ => panic!("pas en mode modèles"),
    }
}

/// La valeur résolue par Typst au dernier rendu, au chemin `path`.
fn rendered_token(s: &Session, path: &str) -> Option<Value> {
    s.compiled.as_ref()?.layout().tokens.as_ref()?.pointer(path).cloned()
}

#[test]
fn entering_templates_loads_full_theme_tokens_without_theme_ref() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    assert_that!(s.deck().theme).is_none();
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_equal_to(Some(&json!("#3f6b5e")));
}

#[test]
fn entering_templates_drops_the_deck_overrides() {
    let mut s = session_with_theme(PAPIER_THEME);
    set_accent(&mut s, "#123456");
    s.edit_templates(DEFAULT_PACK).unwrap();
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_equal_to(Some(&json!("#3f6b5e")));
    let edit = theme_edit(&s);
    assert_that!(edit.base.id).is_equal_to(PAPIER_THEME);
    assert_that!(edit.name.as_str()).is_equal_to("Papier");
}

#[test]
fn template_mode_render_takes_keys_the_edited_theme_omits_from_the_defaults() {
    // Pro ne définit pas `script-style` ; Papier (le thème par défaut) met
    // `display` à `serif`. Le brouillon de Pro se rend sur les défauts seuls.
    let mut s = session_with_theme(PRO_THEME);
    assert_that!(rendered_token(&s, "/script-style/display")).is_equal_to(Some(json!("sans")));
    s.edit_templates(DEFAULT_PACK).unwrap();
    assert_that!(rendered_token(&s, "/script-style/display")).is_equal_to(Some(json!("sans")));
    assert_that!(rendered_token(&s, "/color/brand-ink")).is_equal_to(Some(json!("#00154e")));
}

#[test]
fn saving_edited_builtin_creates_a_copy_used_by_the_deck() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.apply("x", vec![Op::SetToken { path: p(&["color", "accent"]), value: json!("#ff0000") }], None).unwrap();
    s.save_templates(false).unwrap();
    let r = s.user_deck().theme.clone().unwrap();
    assert_that!(r.id).is_not_equal_to(PAPIER_THEME);
    let m = s.theme_manifest(&r).unwrap();
    assert_that!(m.name.as_str()).is_equal_to("Papier (copie)");
    assert_that!(m.parent.is_some()).is_true();
    assert_that!(m.tokens["color"]["accent"]).is_equal_to(json!("#ff0000"));
}

#[test]
fn saving_edited_builtin_copy_is_current_and_listed() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    set_accent(&mut s, "#ff0000");
    s.save_templates(false).unwrap();
    let r = s.user_deck().theme.clone().unwrap();
    assert_that!(s.themes.current(r.id)).is_equal_to(Some(r.version.clone()));
    assert_that!(s.snapshot().themes.iter().any(|t| t.id == r.id)).is_true();
    // L'édition continue sur la copie, sans brouillon en attente.
    let view = s.snapshot().editing.unwrap().theme;
    assert_that!(view.base).is_equal_to(r);
    assert_that!(view.builtin).is_false();
    assert_that!(view.dirty).is_false();
}

#[test]
fn saving_renamed_builtin_keeps_the_given_name() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.rename_edited_theme("Papier rouge".into()).unwrap();
    s.save_templates(false).unwrap();
    let r = s.user_deck().theme.clone().unwrap();
    assert_that!(r.id).is_not_equal_to(PAPIER_THEME);
    assert_that!(s.theme_manifest(&r).unwrap().name.as_str()).is_equal_to("Papier rouge");
}

#[test]
fn saving_edited_user_theme_adds_a_current_version() {
    let data = fresh_dir("oku-slides-theme-edit-");
    let mut s = opened_on(data.path());
    let id = Uuid::new_v4();
    let old = add_user_theme(&s, id);
    s.set_theme(id).unwrap();
    s.edit_templates(DEFAULT_PACK).unwrap();
    set_accent(&mut s, "#ff0000");
    s.save_templates(false).unwrap();
    let r = s.user_deck().theme.clone().unwrap();
    assert_that!(r.id).is_equal_to(id);
    assert_that!(r.version).is_not_equal_to(old.version.clone());
    assert_that!(s.themes.current(id)).is_equal_to(Some(r.version.clone()));
    let m = s.theme_manifest(&r).unwrap();
    assert_that!(m.parent).is_equal_to(Some(old.version));
    assert_that!(m.name.as_str()).is_equal_to("Maison");
    assert_that!(m.description.as_str()).is_equal_to("Le thème de la maison");
    assert_that!(m.tokens["color"]["accent"]).is_equal_to(json!("#ff0000"));
}

#[test]
fn saving_without_theme_change_creates_nothing() {
    let mut s = session_with_theme(PAPIER_THEME);
    let before = s.user_deck().theme.clone();
    let themes = s.themes.themes();
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.save_templates(false).unwrap();
    assert_that!(s.user_deck().theme).is_equal_to(before);
    assert_that!(s.themes.themes()).is_equal_to(themes);
}

#[test]
fn saving_after_choosing_an_unchanged_theme_moves_the_deck_to_it() {
    let mut s = session_with_theme(PAPIER_THEME);
    let themes = s.themes.themes();
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.set_edited_theme(PRO_THEME).unwrap();
    s.save_templates(false).unwrap();
    let r = s.user_deck().theme.clone().unwrap();
    assert_that!(r.id).is_equal_to(PRO_THEME);
    assert_that!(s.theme_manifest(&r).map(|m| m.name)).is_equal_to(Some("Pro".to_owned()));
    assert_that!(s.themes.themes()).is_equal_to(themes);
}

#[test]
fn saving_theme_joins_the_pack_undo_step_of_the_stashed_deck() {
    let mut s = session_with_theme(PAPIER_THEME);
    let before = s.user_deck().theme.clone();
    let templates = s.user_deck().templates.clone();
    s.edit_templates(DEFAULT_PACK).unwrap();
    set_accent(&mut s, "#ff0000");
    s.save_templates(false).unwrap();
    s.close_templates().unwrap();
    assert_that!(s.deck().theme).is_not_equal_to(before.clone());
    s.undo().unwrap();
    assert_that!(s.deck().theme).is_equal_to(before);
    assert_that!(s.deck().templates).is_equal_to(templates);
}

#[test]
fn closing_without_saving_discards_theme_draft() {
    let mut s = session_with_theme(PAPIER_THEME);
    let before = s.user_deck().theme.clone();
    let themes = s.themes.themes();
    s.edit_templates(DEFAULT_PACK).unwrap();
    set_accent(&mut s, "#ff0000");
    s.close_templates().unwrap();
    assert_that!(s.deck().theme).is_equal_to(before);
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_none();
    assert_that!(s.themes.themes()).is_equal_to(themes);
}

#[test]
fn removing_a_referenced_color_is_refused() {
    let mut s = session_with_theme(PRO_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    let r = s.apply("x", vec![Op::UnsetToken { path: p(&["color", "brand-ink"]) }], None);
    assert_that!(r).is_err();
    assert_that!(r.unwrap_err().kind()).is_equal_to(SessionErrorKind::TokenInUse);
    assert_that!(s.deck().tokens.pointer("/color/brand-ink")).is_some();
}

#[test]
fn removing_a_default_color_from_the_draft_is_allowed() {
    // `accent` reste défini par les défauts : ceux qui y renvoient le trouvent.
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.apply("x", vec![Op::UnsetToken { path: p(&["color", "accent"]) }], None).unwrap();
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_none();
}

#[test]
fn template_mode_invalid_token_value_is_refused() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    let before = s.deck().tokens.clone();
    let r = s.apply("x", vec![Op::SetToken { path: p(&["size", "body"]), value: json!(-5) }], None);
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(SessionErrorKind::InvalidTokenValue);
    assert_that!(s.deck().tokens).is_equal_to(before);
}

#[test]
fn template_mode_token_may_refer_to_a_color_added_earlier_in_the_same_transaction() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
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
fn choosing_another_theme_replaces_draft_in_one_undo_step() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.set_edited_theme(PRO_THEME).unwrap();
    assert_that!(s.deck().tokens.pointer("/font/body/0")).is_equal_to(Some(&json!("Inter")));
    s.undo().unwrap();
    assert_that!(s.deck().tokens.pointer("/font/body/0")).is_equal_to(Some(&json!("Instrument Sans")));
}

#[test]
fn undoing_a_theme_choice_brings_back_the_previous_base() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    set_accent(&mut s, "#ff0000");
    s.set_edited_theme(PRO_THEME).unwrap();
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
fn set_edited_theme_outside_template_mode_is_refused() {
    let mut s = session_with_theme(PAPIER_THEME);
    let r = s.set_edited_theme(PRO_THEME);
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(SessionErrorKind::NotEditingTemplates);
}

#[test]
fn set_theme_in_template_mode_chooses_the_edited_theme() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.set_theme(PRO_THEME).unwrap();
    assert_that!(s.deck().theme).is_none();
    assert_that!(theme_edit(&s).base.id).is_equal_to(PRO_THEME);
}

#[test]
fn theme_edit_view_reports_dirty_after_a_token_edit_or_a_rename() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    let view = s.snapshot().editing.unwrap().theme;
    assert_that!(view.dirty).is_false();
    assert_that!(view.builtin).is_true();
    assert_that!(view.name.as_str()).is_equal_to("Papier");
    set_accent(&mut s, "#ff0000");
    assert_that!(s.snapshot().editing.unwrap().theme.dirty).is_true();
    s.undo().unwrap();
    assert_that!(s.snapshot().editing.unwrap().theme.dirty).is_false();
    s.rename_edited_theme("Autre".into()).unwrap();
    let view = s.snapshot().editing.unwrap().theme;
    assert_that!((view.dirty, view.name.as_str())).is_equal_to((true, "Autre"));
}

/// La révision du brouillon vue par le snapshot.
fn revision(s: &Session) -> String {
    s.snapshot().editing.unwrap().theme.revision
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
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.apply("x", vec![set_body_size(16)], None).unwrap();
    let (r18, r20) = (revisions_around(&mut s, set_body_size(18)), revisions_around(&mut s, set_body_size(20)));
    assert_that!(r18.0).is_not_equal_to(r18.1.clone());
    assert_that!(r20.0).is_not_equal_to(r20.1);
}

#[test]
fn theme_edit_revision_changes_on_unset_token() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    let (before, after) = revisions_around(&mut s, Op::UnsetToken { path: p(&["color", "accent"]) });
    assert_that!(before).is_not_equal_to(after);
}

#[test]
fn theme_edit_revision_changes_on_set_tokens() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    let (before, after) = revisions_around(&mut s, Op::SetTokens { tokens: json!({ "size": { "body": 30 } }) });
    assert_that!(before).is_not_equal_to(after);
}

#[test]
fn theme_edit_revision_changes_on_undo_and_redo_between_same_label_steps() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
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
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    assert_that!(revision(&s)).is_equal_to(revision(&s));
}

#[test]
fn rename_edited_theme_with_blank_name_keeps_the_base_name() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.rename_edited_theme("   ".into()).unwrap();
    assert_that!(theme_edit(&s).name.as_str()).is_equal_to("Papier");
}

#[test]
fn theme_requests_choose_and_rename_the_edited_theme() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    let json = s.handle(crate::Request::SetEditedTheme { id: PRO_THEME }).unwrap();
    let snap: Value = serde_json::from_str(&json).unwrap();
    assert_that!(snap.pointer("/editing/theme/base/id")).is_equal_to(Some(&json!(PRO_THEME.to_string())));
    let json = s.handle(crate::Request::RenameEditedTheme { name: "Pro à moi".into() }).unwrap();
    let snap: Value = serde_json::from_str(&json).unwrap();
    assert_that!(snap.pointer("/editing/theme/name")).is_equal_to(Some(&json!("Pro à moi")));
    assert_that!(snap.pointer("/editing/theme/dirty")).is_equal_to(Some(&json!(true)));
    assert_that!(snap.pointer("/editing/set")).is_equal_to(Some(&json!(DEFAULT_PACK.to_string())));
}

#[test]
fn switching_packs_in_template_mode_keeps_the_theme_draft() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    set_accent(&mut s, "#ff0000");
    s.rename_edited_theme("Rouge".into()).unwrap();
    s.edit_templates(crate::GRID_PACK).unwrap();
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_equal_to(Some(&json!("#ff0000")));
    assert_that!(theme_edit(&s).name.as_str()).is_equal_to("Rouge");
}

#[test]
fn theme_edit_survives_resume() {
    let data = fresh_dir("oku-slides-theme-edit-resume-");
    let dir = fresh_dir("oku-slides-theme-edit-resume-state-");
    let mut s = opened_on(data.path());
    assert_that!(s.resume(dir.path().to_owned())).is_true();
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.handle(crate::Request::SetEditedTheme { id: PRO_THEME }).unwrap();
    s.handle(crate::Request::RenameEditedTheme { name: "Pro à moi".into() }).unwrap();
    s.flush_resume();
    drop(s);
    let mut back = crate::test_support::session_on(data.path());
    assert_that!(back.resume(dir.path().to_owned())).is_true();
    let edit = theme_edit(&back);
    assert_that!(edit.base.id).is_equal_to(PRO_THEME);
    assert_that!(edit.name.as_str()).is_equal_to("Pro à moi");
    assert_that!(back.deck().theme).is_none();
    assert_that!(ThemeLibrary::is_builtin(edit.base.id)).is_true();
}

const SWATCH: &str = r#"#import "/oku-slides/lib.typ": widget, color-of, params as p
#let widget = widget("pastille", params: (body: p.plain()),
  (args, ctx) => rect(width: 40pt, height: 40pt, fill: color-of(ctx.tokens, "accent")))
"#;

#[test]
fn widget_preview_in_template_mode_follows_the_theme_draft() {
    let data = fresh_dir("oku-slides-theme-edit-preview-");
    let mut s = opened_on(data.path());
    set_accent(&mut s, "#abcdef");
    let set = s.deck().templates.clone().unwrap().id;
    s.edit_templates(set).unwrap();
    s.set_widget("pastille", SWATCH.into()).unwrap();
    set_accent(&mut s, "#12ab34");
    let preview = s.preview_widget("pastille", &serde_json::Map::new(), crate::PreviewFormat::Svg).unwrap();
    let svg = preview.svg.unwrap_or_default().to_lowercase();
    assert_that!(svg.contains("#12ab34")).is_true();
    assert_that!(svg.contains("#abcdef")).is_false();
}

#[test]
fn templates_view_in_template_mode_follows_the_theme_draft() {
    let mut s = session_with_theme(PAPIER_THEME);
    set_accent(&mut s, "#abcdef");
    s.edit_templates(DEFAULT_PACK).unwrap();
    let before = s.templates_view(None).unwrap();
    assert_that!(before.templates.iter().any(|t| t.svg.to_lowercase().contains("#abcdef"))).is_false();
    set_accent(&mut s, "#12ab34");
    let after = s.templates_view(None).unwrap();
    assert_that!(after.templates.iter().any(|t| t.svg.to_lowercase().contains("#12ab34"))).is_true();
}

#[test]
fn set_edited_theme_writes_the_new_base_to_the_resume_state() {
    let data = fresh_dir("oku-slides-theme-edit-resume-");
    let dir = fresh_dir("oku-slides-theme-edit-resume-state-");
    let mut s = opened_on(data.path());
    assert_that!(s.resume(dir.path().to_owned())).is_true();
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.handle(crate::Request::SetEditedTheme { id: PRO_THEME }).unwrap();
    s.flush_resume();
    drop(s);
    let mut back = crate::test_support::session_on(data.path());
    assert_that!(back.resume(dir.path().to_owned())).is_true();
    let edit = theme_edit(&back);
    assert_that!((edit.base.id, edit.name.as_str())).is_equal_to((PRO_THEME, "Pro"));
    assert_that!(back.deck().tokens.pointer("/font/body/0")).is_equal_to(Some(&json!("Inter")));
    assert_that!(back.snapshot().editing.unwrap().theme.dirty).is_false();
}

#[test]
fn undoing_a_theme_choice_after_saving_it_brings_back_the_previous_base() {
    let mut s = session_with_theme(PAPIER_THEME);
    let papier = s.user_deck().theme.clone();
    let themes = s.themes.themes();
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.set_edited_theme(PRO_THEME).unwrap();
    s.save_templates(false).unwrap();
    assert_that!(s.user_deck().theme.as_ref().map(|r| r.id)).is_equal_to(Some(PRO_THEME));
    s.undo().unwrap();
    assert_that!(theme_edit(&s).base.id).is_equal_to(PAPIER_THEME);
    assert_that!(s.snapshot().editing.unwrap().theme.dirty).is_false();
    s.save_templates(false).unwrap();
    assert_that!(s.user_deck().theme).is_equal_to(papier);
    assert_that!(s.themes.themes()).is_equal_to(themes);
}

#[test]
fn undoing_a_theme_choice_after_saving_a_copy_keeps_base_and_tokens_together() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.set_edited_theme(PRO_THEME).unwrap();
    set_accent(&mut s, "#ff0000");
    s.save_templates(false).unwrap();
    let copy = theme_edit(&s).base;
    assert_that!(copy.id).is_not_equal_to(PRO_THEME);
    // L'accent, puis le choix de Pro.
    s.undo().unwrap();
    assert_that!(theme_edit(&s).base).is_equal_to(copy.clone());
    s.undo().unwrap();
    assert_that!(theme_edit(&s).base.id).is_equal_to(PAPIER_THEME);
    assert_that!(s.snapshot().editing.unwrap().theme.dirty).is_false();
    s.redo().unwrap();
    assert_that!(theme_edit(&s).base).is_equal_to(copy);
}

#[test]
fn template_mode_set_tokens_with_an_invalid_leaf_is_refused() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
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
fn template_mode_valid_set_tokens_is_accepted() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
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

#[test]
fn set_edited_theme_embeds_the_base_version_in_the_working_sources() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.set_edited_theme(PRO_THEME).unwrap();
    let base = theme_edit(&s).base;
    assert_that!(s.doc.sources.themes.contains_key(&base.version)).is_true();
}

#[test]
fn entering_templates_without_deck_theme_reads_the_default_base_from_the_sources() {
    let mut s = session();
    s.apply("x", vec![Op::SetTheme { theme: None }], None).unwrap();
    s.edit_templates(DEFAULT_PACK).unwrap();
    let base = theme_edit(&s).base;
    // Embarquée par le deck de travail, ou déjà par le deck mis de côté : en mémoire.
    assert_that!(s.theme_manifest(&base).is_some()).is_true();
}

#[test]
fn base_tokens_of_a_library_theme_survive_the_library_going_away() {
    let data = fresh_dir("oku-slides-theme-edit-base-");
    let mut s = opened_on(data.path());
    let id = Uuid::new_v4();
    add_user_theme(&s, id);
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.set_edited_theme(id).unwrap();
    data.close().unwrap();
    assert_that!(s.base_tokens().get("color").is_some()).is_true();
}

#[test]
fn template_mode_set_theme_op_alone_chooses_the_edited_theme() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    let pro = ThemeRef { id: PRO_THEME, version: s.themes.current(PRO_THEME).unwrap() };
    s.apply("x", vec![Op::SetTheme { theme: Some(pro.clone()) }], None).unwrap();
    assert_that!(s.deck().theme).is_none();
    assert_that!(theme_edit(&s).base).is_equal_to(pro);
}

#[test]
fn template_mode_set_theme_op_with_other_ops_is_refused() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    let before = theme_edit(&s);
    let pro = ThemeRef { id: PRO_THEME, version: s.themes.current(PRO_THEME).unwrap() };
    let ops = vec![
        Op::SetTheme { theme: Some(pro) },
        Op::SetToken { path: p(&["color", "accent"]), value: json!("#ff0000") },
    ];
    let r = s.apply("x", ops, None);
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(SessionErrorKind::ThemeChoiceAlone);
    assert_that!(theme_edit(&s)).is_equal_to(before);
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_not_equal_to(Some(&json!("#ff0000")));
}

#[test]
fn saving_an_unchanged_base_missing_from_the_library_takes_the_embedded_copy() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.set_edited_theme(PRO_THEME).unwrap();
    // Une ancienne version de Pro, embarquée (mode modèles repris) mais que
    // la bibliothèque n'a plus (l'app a été mise à jour).
    let pro = Themes::builtins().iter().find(|b| b.manifest.id == PRO_THEME).unwrap();
    let old = ThemeManifest { description: "Pro d'avant".into(), ..pro.manifest.clone() };
    let files = old.to_files().unwrap();
    let base = ThemeRef { id: PRO_THEME, version: files.version() };
    assert_that!(s.themes.version(PRO_THEME, &base.version)).is_none();
    s.doc.sources.add_theme(files);
    s.set_theme_edit(ThemeEdit { base: base.clone(), name: old.name.clone() });
    s.save_templates(false).unwrap();
    assert_that!(s.user_deck().theme).is_equal_to(Some(base.clone()));
    s.close_templates().unwrap();
    assert_that!(s.doc.sources.themes.contains_key(&base.version)).is_true();
}

/// Les versions de thème que le deck ou son historique référencent.
fn referenced_themes(s: &Session) -> std::collections::BTreeSet<oku_slides_core::PackVersion> {
    let preview = s.doc.sources.history();
    let mut refs: std::collections::BTreeSet<_> = s.deck().theme.iter().map(|r| r.version.clone()).collect();
    for i in 0..preview.len() {
        let (deck, _) = preview.deck_at(i).unwrap();
        refs.extend(deck.theme.iter().map(|r| r.version.clone()));
    }
    refs
}

#[test]
fn saving_templates_without_theme_change_leaks_no_draft_base_into_the_saved_deck() {
    let data = fresh_dir("oku-slides-theme-edit-leak-");
    let mut s = opened_on(data.path());
    let before = s.doc.sources.themes.keys().cloned().collect::<Vec<_>>();
    let set = s.deck().templates.clone().unwrap().id;
    s.edit_templates(set).unwrap();
    // Un détour par Pro puis retour au thème du deck : le brouillon a
    // embarqué Pro dans le deck de travail, pas dans celui de l'utilisateur.
    let deck_theme = s.user_deck().theme.clone().unwrap();
    s.set_edited_theme(PRO_THEME).unwrap();
    s.set_edited_theme(deck_theme.id).unwrap();
    s.save_templates(false).unwrap();
    s.close_templates().unwrap();
    let file = data.path().join("fuite.oku-slides.pdf");
    s.save(Some(&file)).unwrap();
    let mut back = opened_on(data.path());
    back.open(&file).unwrap();
    let after = back.doc.sources.themes.keys().cloned().collect::<Vec<_>>();
    assert_that!(after).is_equal_to(before);
    let refs = referenced_themes(&back);
    assert_that!(after.iter().all(|v| refs.contains(v))).is_true();
}

#[test]
fn saving_an_edited_deck_only_theme_keeps_it_out_of_the_library() {
    let data = fresh_dir("oku-slides-theme-edit-deck-only-");
    let mut s = opened_on(data.path());
    let before = adopt_deck_only_theme(&mut s);
    let themes = s.themes.themes();
    s.edit_templates(DEFAULT_PACK).unwrap();
    set_accent(&mut s, "#ff0000");
    s.save_templates(false).unwrap();
    assert_that!(s.themes.themes()).is_equal_to(&themes);
    let r = s.deck_theme().cloned().unwrap();
    assert_that!(r.id).is_equal_to(before.id);
    let m = s.theme_manifest(&r).unwrap();
    assert_that!(m.scope).is_equal_to(oku_slides_core::PackScope::Deck);
    assert_that!(m.parent).is_equal_to(Some(before.version));
    assert_that!(m.tokens["color"]["accent"]).is_equal_to(json!("#ff0000"));
}

#[test]
fn set_edited_theme_of_a_deck_only_theme_starts_from_its_embedded_version() {
    let data = fresh_dir("oku-slides-theme-edit-deck-only-pick-");
    let mut s = opened_on(data.path());
    let r = adopt_deck_only_theme(&mut s);
    let tokens = s.theme_manifest(&r).unwrap().tokens;
    s.set_theme(PAPIER_THEME).unwrap();
    let (themes, files) = (s.themes.themes(), crate::test_support::theme_library_files(data.path()));
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.set_edited_theme(r.id).unwrap();
    assert_that!(theme_edit(&s)).is_equal_to(ThemeEdit { base: r.clone(), name: "Importé".into() });
    assert_that!(s.edited_tokens()).is_equal_to(tokens);
    s.save_templates(false).unwrap();
    s.close_templates().unwrap();
    assert_that!(s.deck().theme.clone()).is_equal_to(Some(r));
    assert_that!(s.themes.themes()).is_equal_to(themes);
    assert_that!(crate::test_support::theme_library_files(data.path())).is_equal_to(files);
}
