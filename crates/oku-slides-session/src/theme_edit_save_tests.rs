use oku_slides_core::{Op, ThemePin};
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::ThemeEdit;
use super::theme_edit_tests::{add_user_theme, builtin_ns, p, session_with_theme, set_accent, theme_edit, user_pin};
use crate::Session;
use crate::library_kind::LibraryKind;
use crate::pack::ESSENTIEL_PACK;
use crate::test_support::{fresh_dir, ns, opened_on};
use crate::theme::{PAPIER_THEME, PRO_THEME, ThemeManifest, Themes};

#[test]
fn saving_edited_builtin_creates_a_child_used_by_the_deck() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.apply("x", vec![Op::SetToken { path: p(&["color", "accent"]), value: json!("#ff0000") }], None).unwrap();
    s.save_pack(false).unwrap();
    let r = user_pin(&s);
    assert_that!(r.id).is_not_equal_to(PAPIER_THEME);
    assert_that!(r.extends.clone()).is_equal_to(vec![ns("papier")]);
    let m = s.theme_manifest(&r).unwrap();
    assert_that!(m.name.as_str()).is_equal_to("Papier (variante)");
    assert_that!(m.parent).is_none();
    assert_that!(m.tokens["color"]["accent"]).is_equal_to(json!("#ff0000"));
}

#[test]
fn saving_edited_builtin_copy_is_current_and_listed() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    set_accent(&mut s, "#ff0000");
    s.save_pack(false).unwrap();
    let r = user_pin(&s);
    assert_that!(s.themes.current(r.id)).is_equal_to(Some(r.version.clone()));
    assert_that!(s.snapshot().themes.iter().any(|t| t.id == r.id)).is_true();
    // L'édition continue sur la copie, sans brouillon en attente.
    let view = s.snapshot().editing.unwrap().theme;
    assert_that!(view.base).is_equal_to(r);
    assert_that!(view.child).is_false();
    assert_that!(view.dirty).is_false();
}

#[test]
fn saving_renamed_builtin_keeps_the_given_name() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.rename_edited_theme("Papier rouge".into()).unwrap();
    s.save_pack(false).unwrap();
    let r = user_pin(&s);
    assert_that!(r.id).is_not_equal_to(PAPIER_THEME);
    assert_that!(s.theme_manifest(&r).unwrap().name.as_str()).is_equal_to("Papier rouge");
}

#[test]
fn saving_edited_user_theme_adds_a_current_version() {
    let data = fresh_dir("oku-slides-theme-edit-");
    let mut s = opened_on(data.path());
    let id = Uuid::new_v4();
    let old = add_user_theme(&s, id);
    s.set_theme(&ns("maison")).unwrap();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    set_accent(&mut s, "#ff0000");
    s.save_pack(false).unwrap();
    let r = user_pin(&s);
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
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.save_pack(false).unwrap();
    assert_that!(s.user_deck().theme).is_equal_to(before);
    assert_that!(s.themes.themes()).is_equal_to(themes);
}

#[test]
fn saving_after_choosing_an_unchanged_theme_moves_the_deck_to_it() {
    let mut s = session_with_theme(PAPIER_THEME);
    let themes = s.themes.themes();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.set_edited_theme(&ns("pro")).unwrap();
    s.save_pack(false).unwrap();
    let r = user_pin(&s);
    assert_that!(r.id).is_equal_to(PRO_THEME);
    assert_that!(s.theme_manifest(&r).map(|m| m.name)).is_equal_to(Some("Pro".to_owned()));
    assert_that!(s.themes.themes()).is_equal_to(themes);
}

#[test]
fn saving_theme_joins_the_pack_undo_step_of_the_stashed_deck() {
    let mut s = session_with_theme(PAPIER_THEME);
    let before = s.user_deck().theme.clone();
    let templates = s.user_deck().widgets.clone();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    set_accent(&mut s, "#ff0000");
    s.save_pack(false).unwrap();
    s.close_pack().unwrap();
    assert_that!(s.deck().theme).is_not_equal_to(before.clone());
    s.undo().unwrap();
    assert_that!(s.deck().theme).is_equal_to(before);
    assert_that!(s.deck().widgets.clone()).is_equal_to(templates);
}

#[test]
fn closing_without_saving_discards_theme_draft() {
    let mut s = session_with_theme(PAPIER_THEME);
    let before = s.user_deck().theme.clone();
    let themes = s.themes.themes();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    set_accent(&mut s, "#ff0000");
    s.close_pack().unwrap();
    assert_that!(s.deck().theme).is_equal_to(before);
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_none();
    assert_that!(s.themes.themes()).is_equal_to(themes);
}

#[test]
fn set_edited_theme_writes_the_new_base_to_the_resume_state() {
    let data = fresh_dir("oku-slides-theme-edit-resume-");
    let dir = fresh_dir("oku-slides-theme-edit-resume-state-");
    let mut s = opened_on(data.path());
    assert_that!(s.resume(dir.path().to_owned())).is_true();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.handle(crate::Request::SetEditedTheme { id: PRO_THEME }).unwrap();
    s.flush_resume();
    drop(s);
    let mut back = crate::test_support::session_on(data.path());
    assert_that!(back.resume(dir.path().to_owned())).is_true();
    let edit = theme_edit(&back);
    assert_that!((edit.base.id, edit.name.as_str(), edit.child())).is_equal_to((PRO_THEME, "Pro", true));
    assert_that!(back.deck().tokens.clone()).is_equal_to(Value::Null);
    assert_that!(back.snapshot().editing.unwrap().theme.dirty).is_false();
}

#[test]
fn undoing_a_theme_choice_after_saving_it_brings_back_the_previous_base() {
    let data = fresh_dir("oku-slides-theme-edit-");
    let mut s = opened_on(data.path());
    let id = Uuid::new_v4();
    add_user_theme(&s, id);
    let papier = s.user_deck().theme.clone();
    let themes = s.themes.themes();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.set_edited_theme(&ns("maison")).unwrap();
    s.save_pack(false).unwrap();
    assert_that!(s.deck_theme_pin().map(|r| r.id)).is_equal_to(Some(id));
    s.undo().unwrap();
    assert_that!(theme_edit(&s).base.id).is_equal_to(PAPIER_THEME);
    assert_that!(s.snapshot().editing.unwrap().theme.dirty).is_false();
    s.save_pack(false).unwrap();
    assert_that!(s.user_deck().theme).is_equal_to(papier);
    assert_that!(s.themes.themes()).is_equal_to(themes);
}

#[test]
fn undoing_a_theme_choice_after_saving_a_child_keeps_base_and_tokens_together() {
    let data = fresh_dir("oku-slides-theme-edit-");
    let mut s = opened_on(data.path());
    add_user_theme(&s, Uuid::new_v4());
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    set_accent(&mut s, "#00ff00");
    s.set_edited_theme(&ns("maison")).unwrap();
    s.set_edited_theme(&ns("pro")).unwrap();
    set_accent(&mut s, "#ff0000");
    s.save_pack(false).unwrap();
    let child = theme_edit(&s).base;
    assert_that!(child.extends.clone()).is_equal_to(vec![ns("pro")]);
    // L'accent, puis le choix de Pro, puis celui de Maison.
    s.undo().unwrap();
    assert_that!(theme_edit(&s).base).is_equal_to(child.clone());
    s.undo().unwrap();
    assert_that!(theme_edit(&s).name.as_str()).is_equal_to("Maison");
    s.undo().unwrap();
    assert_that!(theme_edit(&s).base.id).is_equal_to(PAPIER_THEME);
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_equal_to(Some(&json!("#00ff00")));
    s.redo().unwrap();
    assert_that!(theme_edit(&s).name.as_str()).is_equal_to("Maison");
}

#[test]
fn base_tokens_of_a_library_theme_survive_the_library_going_away() {
    let data = fresh_dir("oku-slides-theme-edit-base-");
    let mut s = opened_on(data.path());
    let id = Uuid::new_v4();
    add_user_theme(&s, id);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.set_edited_theme(&ns("maison")).unwrap();
    data.close().unwrap();
    assert_that!(s.base_tokens().get("color").is_some()).is_true();
}

#[test]
fn saving_an_unchanged_base_missing_from_the_library_takes_the_embedded_copy() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.set_edited_theme(&ns("pro")).unwrap();
    // Une ancienne version de Pro, embarquée (mode pack repris) mais que
    // la bibliothèque n'a plus (l'app a été mise à jour).
    let pro = Themes::builtins().iter().find(|b| b.manifest.id == PRO_THEME).unwrap();
    let old = ThemeManifest { description: "Pro d'avant".into(), ..pro.manifest.clone() };
    let files = old.to_files().unwrap();
    let base = ThemePin { id: PRO_THEME, version: files.version(), extends: Vec::new() };
    assert_that!(s.themes.version(PRO_THEME, &base.version)).is_none();
    s.doc.sources.add_theme(files);
    s.set_theme_edit(ThemeEdit {
        base: base.clone(),
        namespace: ns("pro"),
        name: old.name.clone(),
        extends: vec![ns("pro")],
    });
    s.save_pack(false).unwrap();
    assert_that!(user_pin(&s)).is_equal_to(base.clone());
    s.close_pack().unwrap();
    assert_that!(s.doc.sources.themes.contains_key(&base.version)).is_true();
}

/// Les versions de thème que le deck ou son historique référencent.
fn referenced_themes(s: &Session) -> std::collections::BTreeSet<oku_slides_core::PackVersion> {
    let preview = s.doc.sources.history();
    let mut refs: std::collections::BTreeSet<_> = s.deck().themes.values().map(|r| r.version.clone()).collect();
    for i in 0..preview.len() {
        let (deck, _) = preview.deck_at(i).unwrap();
        refs.extend(deck.themes.values().map(|r| r.version.clone()));
    }
    refs
}

#[test]
fn saving_the_pack_without_theme_change_leaks_no_draft_base_into_the_saved_deck() {
    let data = fresh_dir("oku-slides-theme-edit-leak-");
    let mut s = opened_on(data.path());
    let before = s.doc.sources.themes.keys().cloned().collect::<Vec<_>>();
    let set = crate::ESSENTIEL_PACK;
    s.edit_pack(set).unwrap();
    // Un détour par Pro puis retour au thème du deck : le brouillon a
    // embarqué Pro dans le deck de travail, pas dans celui de l'utilisateur.
    let deck_theme = user_pin(&s);
    s.set_edited_theme(&ns("pro")).unwrap();
    s.set_edited_theme(&builtin_ns(deck_theme.id)).unwrap();
    s.save_pack(false).unwrap();
    s.close_pack().unwrap();
    let file = data.path().join("fuite.oku-slides.pdf");
    s.save(Some(&file)).unwrap();
    let mut back = opened_on(data.path());
    back.open(&file).unwrap();
    let after = back.doc.sources.themes.keys().cloned().collect::<Vec<_>>();
    assert_that!(after).is_equal_to(before);
    let refs = referenced_themes(&back);
    assert_that!(after.iter().all(|v| refs.contains(v))).is_true();
}
