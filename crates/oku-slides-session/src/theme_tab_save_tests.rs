use std::collections::BTreeSet;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use oku_slides_core::{Op, PackVersion, ThemePin};
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::library::draft_lock;
use crate::pack_lease::PackLease;
use crate::test_support::{fresh_dir, ns};
use crate::test_support_packs::theme_scope_of;
use crate::theme::{PAPIER_THEME, ThemeManifest};
use crate::theme_edit::theme_edit_tests::p;
use crate::theme_tab::ThemeTab;
use crate::theme_tab::theme_tab_tests::{theme_tab, with_maison};
use crate::{Mode, Request, Session, SessionErrorKind, TabDocument};

/// Sets the draft's token `path` to `value`, one undo step.
fn set(s: &mut Session, path: &[&str], value: &str) {
    s.apply("x", vec![Op::SetToken { path: p(path), value: json!(value) }], None).unwrap();
}

/// Sets the draft's `color.accent` to `value`, one undo step.
fn accent(s: &mut Session, value: &str) {
    set(s, &["color", "accent"], value);
}

/// A theme tab on a new user theme `maison` in `data`: the tab, the theme's id and the version it opened.
fn maison_tab(data: &Path) -> (Session, Uuid, PackVersion) {
    let (mut s, id) = with_maison(data);
    let base = s.themes.current(id).unwrap();
    s.open_theme_here(id).unwrap();
    (s, id, base)
}

/// Saves the tab's draft, which must have changed.
fn saved(s: &mut Session) -> ThemePin {
    s.save_theme(false).unwrap().unwrap()
}

/// The library's versions of theme `id`.
fn versions(s: &Session, id: Uuid) -> BTreeSet<PackVersion> {
    s.themes.versions(id).into_iter().map(|v| v.version).collect()
}

/// The manifest of version `v` of theme `id`, read from the library.
fn manifest(s: &Session, id: Uuid, v: &PackVersion) -> ThemeManifest {
    ThemeManifest::from_files(&s.themes.version(id, v).unwrap()).unwrap()
}

/// What the theme tab edits.
fn tab_of(s: &Session) -> ThemeTab {
    match &s.mode {
        Mode::Theme { edit, .. } => edit.clone(),
        _ => panic!("theme tab expected"),
    }
}

/// A deck adopts version `v` of theme `id`, which freezes it.
fn adopt(s: &Session, id: Uuid, v: &PackVersion) {
    let files = s.themes.version(id, v).unwrap();
    s.themes.adopt(&draft_lock(), id, &files).unwrap();
}

#[test]
fn save_draft_unadopted_replaces_in_place() {
    let data = fresh_dir("oku-slides-theme-save-");
    let (mut s, id, base) = maison_tab(data.path());
    accent(&mut s, "#111111");
    let first = saved(&mut s);
    accent(&mut s, "#222222");
    let second = saved(&mut s);
    assert_that!(second.version).is_not_equal_to(first.version);
    assert_that!(versions(&s, id)).is_equal_to(BTreeSet::from([base.clone(), second.version.clone()]));
    assert_that!(s.themes.current(id)).is_equal_to(Some(second.version.clone()));
    assert_that!(manifest(&s, id, &second.version).parent).is_equal_to(Some(base));
}

#[test]
fn save_draft_adopted_opens_child_version() {
    let data = fresh_dir("oku-slides-theme-save-");
    let (mut s, id, base) = maison_tab(data.path());
    accent(&mut s, "#111111");
    let first = saved(&mut s);
    let first_files = s.themes.version(id, &first.version).unwrap();
    adopt(&s, id, &first.version);
    accent(&mut s, "#222222");
    let second = saved(&mut s);
    assert_that!(manifest(&s, id, &second.version).parent).is_equal_to(Some(first.version.clone()));
    assert_that!(s.themes.version(id, &first.version)).is_equal_to(Some(first_files));
    assert_that!(versions(&s, id)).is_equal_to(BTreeSet::from([base, first.version, second.version]));
}

#[test]
fn adopt_then_save_concurrent_never_rewrites_adopted() {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let data = fresh_dir("oku-slides-theme-save-");
        let (mut s, id, _) = maison_tab(data.path());
        accent(&mut s, "#111111");
        let first = saved(&mut s);
        let adopted = s.themes.version(id, &first.version).unwrap();
        accent(&mut s, "#222222");
        let (themes, files) = (s.themes.clone(), adopted.clone());
        let adopt_thread = std::thread::spawn(move || themes.adopt(&draft_lock(), id, &files).is_ok());
        let save_thread = std::thread::spawn(move || {
            let ok = s.save_theme(false).is_ok();
            (s, ok)
        });
        let (s, save_ok) = save_thread.join().unwrap();
        let adopt_ok = adopt_thread.join().unwrap();
        let kept = s.themes.version(id, &first.version);
        tx.send((adopt_ok, save_ok, kept == Some(adopted))).unwrap();
    });
    let outcome = rx.recv_timeout(Duration::from_secs(120)).expect("adopt and save finish");
    assert_that!(outcome).is_equal_to((true, true, true));
}

#[test]
fn save_theme_unchanged_writes_nothing() {
    let data = fresh_dir("oku-slides-theme-save-");
    let (mut s, id, base) = maison_tab(data.path());
    assert_that!(s.save_theme(false).unwrap()).is_none();
    assert_that!(versions(&s, id)).is_equal_to(BTreeSet::from([base.clone()]));
    assert_that!(s.themes.current(id)).is_equal_to(Some(base));
}

#[test]
fn save_theme_request_answers_null_when_unchanged() {
    let data = fresh_dir("oku-slides-theme-save-");
    let (mut s, _, _) = maison_tab(data.path());
    let out = s.handle(Request::SaveTheme { as_new: false }).unwrap();
    assert_that!(serde_json::from_str::<Value>(&out).unwrap()).is_equal_to(Value::Null);
}

#[test]
fn save_theme_request_answers_the_saved_pin() {
    let data = fresh_dir("oku-slides-theme-save-");
    let (mut s, id, _) = maison_tab(data.path());
    accent(&mut s, "#111111");
    let out = s.handle(Request::SaveTheme { as_new: false }).unwrap();
    let pin: ThemePin = serde_json::from_str(&out).unwrap();
    assert_that!(pin.id).is_equal_to(id);
    assert_that!(s.themes.current(id)).is_equal_to(Some(pin.version));
}

#[test]
fn save_theme_of_a_builtin_creates_a_child_and_edits_it() {
    let data = fresh_dir("oku-slides-theme-save-");
    let mut s = theme_tab(data.path(), PAPIER_THEME);
    accent(&mut s, "#111111");
    let child = saved(&mut s);
    assert_that!(child.id).is_not_equal_to(PAPIER_THEME);
    let m = manifest(&s, child.id, &child.version);
    assert_that!(m.name.as_str()).is_equal_to("Papier (variante)");
    assert_that!(m.extends.clone()).is_equal_to(vec![ns("papier")]);
    assert_that!(m.parent).is_none();
    assert_that!(s.themes.current(child.id)).is_equal_to(Some(child.version.clone()));
    assert_that!(s.tab_info().document).is_equal_to(TabDocument::Theme { theme: child.id });
    assert_that!(tab_of(&s).parent).is_none();
    accent(&mut s, "#222222");
    let again = saved(&mut s);
    assert_that!(again.id).is_equal_to(child.id);
    assert_that!(versions(&s, child.id)).is_equal_to(BTreeSet::from([again.version]));
}

#[test]
fn save_theme_of_a_builtin_moves_the_lease_to_the_child() {
    let data = fresh_dir("oku-slides-theme-save-");
    let mut s = theme_tab(data.path(), PAPIER_THEME);
    let scope = theme_scope_of(&s);
    accent(&mut s, "#111111");
    let child = saved(&mut s);
    assert_that!(crate::pack_lease::pack_leased(&scope, child.id)).is_some();
    crate::test_support_packs::assert_unleased(&scope, PAPIER_THEME);
}

#[test]
fn save_theme_as_new_copies_under_a_new_id() {
    let data = fresh_dir("oku-slides-theme-save-");
    let (mut s, id, base) = maison_tab(data.path());
    accent(&mut s, "#111111");
    let copy = s.save_theme(true).unwrap().unwrap();
    assert_that!(copy.id).is_not_equal_to(id);
    let m = manifest(&s, copy.id, &copy.version);
    let name = s.localizer.tr_with("theme-copy-of", &[("name", "maison".into())]);
    assert_that!(m.name).is_equal_to(name);
    assert_that!(m.parent).is_none();
    assert_that!(m.tokens.get("color").and_then(|c| c.get("accent"))).is_equal_to(Some(&json!("#111111")));
    assert_that!(versions(&s, id)).is_equal_to(BTreeSet::from([base.clone()]));
    assert_that!(s.themes.current(id)).is_equal_to(Some(base));
    assert_that!(s.tab_info().document).is_equal_to(TabDocument::Theme { theme: copy.id });
}

#[test]
fn opening_a_theme_tab_freezes_its_base() {
    let data = fresh_dir("oku-slides-theme-save-");
    let (mut s, id) = with_maison(data.path());
    let base = s.themes.current(id).unwrap();
    assert_that!(s.themes.is_frozen(id, &base)).is_false();
    s.open_theme_here(id).unwrap();
    assert_that!(s.themes.is_frozen(id, &base)).is_true();
}

#[test]
fn save_theme_checks_the_closure_before_writing() {
    let data = fresh_dir("oku-slides-theme-save-");
    let (mut s, id, base) = maison_tab(data.path());
    accent(&mut s, "#111111");
    // The draft extends a theme the library lacks (as if it had been removed).
    if let Mode::Theme { edit, .. } = &mut s.mode {
        edit.draft.extends = vec![ns("absent")];
    }
    let err = s.save_theme(false).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownTheme);
    assert_that!(versions(&s, id)).is_equal_to(BTreeSet::from([base.clone()]));
    assert_that!(s.themes.current(id)).is_equal_to(Some(base));
}

#[test]
fn save_theme_undo_then_save_replaces_the_draft() {
    let data = fresh_dir("oku-slides-theme-save-");
    let (mut s, id, base) = maison_tab(data.path());
    let ink = manifest(&s, id, &base).tokens["color"]["ink"].clone();
    accent(&mut s, "#111111");
    set(&mut s, &["color", "ink"], "#222222");
    let first = saved(&mut s);
    s.undo().unwrap();
    assert_that!(s.themes.current(id)).is_equal_to(Some(first.version.clone()));
    let second = saved(&mut s);
    assert_that!(versions(&s, id)).is_equal_to(BTreeSet::from([base, second.version.clone()]));
    let tokens = manifest(&s, id, &second.version).tokens;
    assert_that!(tokens["color"]["ink"].clone()).is_equal_to(ink);
    assert_that!(tokens["color"]["accent"].clone()).is_equal_to(json!("#111111"));
}

#[test]
fn save_theme_after_adoption_rebases_on_the_frozen_version() {
    let data = fresh_dir("oku-slides-theme-save-");
    let (mut s, id, _) = maison_tab(data.path());
    accent(&mut s, "#111111");
    let first = saved(&mut s);
    adopt(&s, id, &first.version);
    accent(&mut s, "#222222");
    let second = saved(&mut s);
    assert_that!(manifest(&s, id, &second.version).parent).is_equal_to(Some(first.version.clone()));
    assert_that!(tab_of(&s).parent).is_equal_to(Some(first.version.clone()));
    accent(&mut s, "#333333");
    let third = saved(&mut s);
    assert_that!(manifest(&s, id, &third.version).parent).is_equal_to(Some(first.version));
    assert_that!(s.themes.lists(id, &second.version)).is_false();
}

#[test]
fn save_theme_of_a_builtin_takes_the_new_lease_before_writing() {
    let data = fresh_dir("oku-slides-theme-save-");
    let mut s = theme_tab(data.path(), PAPIER_THEME);
    accent(&mut s, "#111111");
    let child = s.theme_to_save(false).unwrap().unwrap();
    // Another tab holds the new theme's id.
    let held = PackLease::acquire(&theme_scope_of(&s), child.id, "Autre", "Autre onglet").unwrap();
    let err = s.write_theme(child.clone()).unwrap_err();
    assert_that!((err.kind(), err.lease_holder()))
        .is_equal_to((SessionErrorKind::ThemeOpenInTab, Some("Autre onglet")));
    assert_that!(s.themes.current(child.id)).is_none();
    assert_that!(tab_of(&s).draft.base.id).is_equal_to(PAPIER_THEME);
    drop(held);
}

#[test]
fn theme_tab_is_not_dirty_after_a_save() {
    let data = fresh_dir("oku-slides-theme-save-");
    let (mut s, _, _) = maison_tab(data.path());
    accent(&mut s, "#111111");
    assert_that!(s.tab_info().dirty).is_true();
    saved(&mut s);
    assert_that!(s.tab_info().dirty).is_false();
    assert_that!(s.theme_tab_view().unwrap().theme.dirty).is_false();
}

#[test]
fn save_theme_keeps_the_session_version() {
    let data = fresh_dir("oku-slides-theme-save-");
    let (mut s, _, _) = maison_tab(data.path());
    accent(&mut s, "#111111");
    let pin = saved(&mut s);
    assert_that!(tab_of(&s).session_version).is_equal_to(Some(pin.version.clone()));
    assert_that!(s.theme_tab_view().unwrap().session_version).is_equal_to(Some(pin.version));
}

#[test]
fn save_theme_outside_a_theme_tab_is_refused() {
    let data = fresh_dir("oku-slides-theme-save-");
    let mut s = crate::test_support::session_on(data.path());
    let err = s.handle(Request::SaveTheme { as_new: false }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::WrongDocument);
}
