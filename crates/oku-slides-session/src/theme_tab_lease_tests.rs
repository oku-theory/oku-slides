use std::collections::BTreeSet;

use oku_slides_core::PackVersion;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::pack_lease::{LeaseScope, PackLease, pack_leased};
use crate::test_support::{fresh_dir, session_on};
use crate::test_support_packs::{assert_unleased, theme_scope_of};
use crate::theme::PAPIER_THEME;
use crate::theme_tab::theme_tab_tests::{theme_tab, with_maison};
use crate::{Localize, Request, SessionErrorKind, TokenOp};

/// The library's versions of theme `id` in `s`.
fn versions(s: &crate::Session, id: Uuid) -> BTreeSet<PackVersion> {
    s.themes.versions(id).into_iter().map(|v| v.version).collect()
}

#[test]
fn theme_tab_holds_its_theme_lease() {
    let data = fresh_dir("oku-slides-theme-lease-");
    let (mut s, id) = with_maison(data.path());
    s.open_theme_here(id).unwrap();
    let scope = theme_scope_of(&s);
    assert_that!(pack_leased(&scope, id)).is_equal_to(Some("maison".to_owned()));
    drop(s);
    assert_unleased(&scope, id);
}

#[test]
fn second_theme_tab_without_lease_is_read_only() {
    let data = fresh_dir("oku-slides-theme-lease-");
    let (mut first, id) = with_maison(data.path());
    first.open_theme_here(id).unwrap();
    let mut second = session_on(data.path());
    second.open_theme_here(id).unwrap();
    let view = second.theme_tab_view().unwrap();
    assert_that!((view.read_only, view.leased)).is_equal_to((true, false));
    let err = second.handle(Request::RenameEditedTheme { name: "Autre".into() }).unwrap_err();
    assert_that!((err.kind(), err.lease_holder())).is_equal_to((SessionErrorKind::ThemeOpenInTab, Some("maison")));
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(err.localize(&fr).as_str()).is_equal_to(
        "Le thème maison est en cours d'édition dans l'onglet « maison » : modifiez-le là, ou fermez d'abord cet onglet.",
    );
    let scope = theme_scope_of(&first);
    drop(first);
    second.handle(Request::RenameEditedTheme { name: "Autre".into() }).unwrap();
    let view = second.theme_tab_view().unwrap();
    assert_that!((view.read_only, view.leased)).is_equal_to((false, true));
    assert_that!(pack_leased(&scope, id)).is_some();
}

#[test]
fn edit_theme_refuses_a_theme_leased_by_a_tab() {
    let data = fresh_dir("oku-slides-theme-lease-");
    let (mut tab, id) = with_maison(data.path());
    tab.open_theme_here(id).unwrap();
    let mut deck = session_on(data.path());
    let before = versions(&deck, id);
    let ops = vec![TokenOp::SetToken { path: vec!["color".into(), "accent".into()], value: "#123456".into() }];
    let err = deck.edit_theme(id, None, ops).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ThemeOpenInTab);
    assert_that!(versions(&deck, id)).is_equal_to(before);
}

#[test]
fn edit_theme_goes_on_once_the_tab_is_gone() {
    let data = fresh_dir("oku-slides-theme-lease-");
    let (mut tab, id) = with_maison(data.path());
    tab.open_theme_here(id).unwrap();
    let scope = theme_scope_of(&tab);
    drop(tab);
    assert_unleased(&scope, id);
    let mut deck = session_on(data.path());
    let ops = vec![TokenOp::SetToken { path: vec!["color".into(), "accent".into()], value: "#123456".into() }];
    let edited = deck.edit_theme(id, None, ops).unwrap();
    assert_that!(edited.changed).is_true();
}

#[test]
fn pack_and_theme_leases_never_collide() {
    let dir = fresh_dir("oku-slides-theme-lease-");
    let packs = LeaseScope::new(&dir.path().join("widgets"), &dir.path().join("layouts"));
    let themes = LeaseScope::themes(&dir.path().join("themes"));
    let id = Uuid::new_v4();
    let pack = PackLease::acquire(&packs, id, "p", "Pack tab").unwrap();
    let theme = PackLease::acquire(&themes, id, "t", "Theme tab").unwrap();
    assert_that!(pack_leased(&packs, id)).is_equal_to(Some("Pack tab".to_owned()));
    assert_that!(pack_leased(&themes, id)).is_equal_to(Some("Theme tab".to_owned()));
    drop((pack, theme));
}

#[test]
fn theme_lease_refusal_names_the_theme() {
    let dir = fresh_dir("oku-slides-theme-lease-");
    let themes = LeaseScope::themes(&dir.path().join("themes"));
    let id = Uuid::new_v4();
    let _held = PackLease::acquire(&themes, id, "Maison", "Tab A").unwrap();
    let err = PackLease::acquire(&themes, id, "Maison", "Tab B").unwrap_err();
    assert_that!((err.kind(), err.lease_holder())).is_equal_to((SessionErrorKind::ThemeOpenInTab, Some("Tab A")));
}

#[test]
fn edit_theme_in_a_theme_tab_is_the_wrong_document() {
    let data = fresh_dir("oku-slides-theme-lease-");
    let mut s = theme_tab(data.path(), PAPIER_THEME);
    let err = s.edit_theme(PAPIER_THEME, Some("x".into()), Vec::new()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::WrongDocument);
}

#[test]
fn merge_file_in_a_theme_tab_is_the_wrong_document() {
    let data = fresh_dir("oku-slides-theme-lease-");
    let mut s = theme_tab(data.path(), PAPIER_THEME);
    let err = s.merge_file(&data.path().join("absent.oku-slides.pdf")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::WrongDocument);
}

#[test]
fn moving_the_pack_dirs_keeps_a_theme_tab_lease() {
    let data = fresh_dir("oku-slides-theme-lease-");
    let (mut s, id) = with_maison(data.path());
    s.open_theme_here(id).unwrap();
    let scope = theme_scope_of(&s);
    s.move_pack_dirs(data.path().join("w2"), data.path().join("l2")).unwrap();
    assert_that!(theme_scope_of(&s)).is_equal_to(scope.clone());
    assert_that!(s.theme_tab_view().unwrap().leased).is_true();
    assert_that!(pack_leased(&scope, id)).is_some();
}

#[test]
fn init_theme_tab_refused_leaves_a_pristine_deck_and_the_theme_unleased() {
    let data = fresh_dir("oku-slides-theme-lease-");
    let (mut s, maison) = with_maison(data.path());
    let scope = theme_scope_of(&s);
    let papier = PackLease::acquire(&scope, PAPIER_THEME, "Papier", "Papier").unwrap();
    let kind = s.init_theme_tab(Some(papier), maison).err().map(|e| e.kind());
    let found = (kind, s.tab_info().pristine, s.document_kind());
    assert_that!(found).is_equal_to((Some(SessionErrorKind::LeaseMissing), true, crate::DocumentKind::Deck));
    assert_unleased(&scope, maison);
    assert_unleased(&scope, PAPIER_THEME);
}
