use oku_slides_core::{PackScope, ThemeRef};
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::deck_theme_edit::TokenOp;
use crate::test_support::{adopt_deck_only_theme, fresh_dir, opened_on, theme_library_files};
use crate::theme::{PAPIER_THEME, ThemeInfo};
use crate::{Session, SessionErrorKind};

fn accent(value: &str) -> TokenOp {
    TokenOp::SetToken { path: vec!["color".into(), "accent".into()], value: json!(value) }
}

/// L'entrée de liste attendue pour la version `r` du thème du deck seul « Importé ».
fn imported(r: &ThemeRef) -> ThemeInfo {
    ThemeInfo {
        id: r.id,
        version: r.version.clone(),
        name: "Importé".into(),
        builtin: false,
        default: false,
        deck_only: true,
    }
}

/// Embarque une version enfant de `parent` (créée à `created`) dans les sources de `s`, sans l'adopter.
fn embed_child(s: &mut Session, parent: &ThemeRef, created: &str) -> ThemeRef {
    let mut manifest = s.theme_manifest(parent).unwrap();
    manifest.parent = Some(parent.version.clone());
    manifest.created = created.into();
    manifest.description = created.into();
    let files = manifest.to_files().unwrap();
    let version = s.doc.sources.add_theme(files);
    ThemeRef { id: parent.id, version }
}

#[test]
fn deck_only_themes_lists_the_import_theme_after_another_is_chosen() {
    let data = fresh_dir("oku-slides-deck-themes-list-");
    let mut s = opened_on(data.path());
    let r = adopt_deck_only_theme(&mut s);
    s.set_theme(PAPIER_THEME).unwrap();
    assert_that!(s.deck_only_themes()).is_equal_to(vec![imported(&r)]);
    assert_that!(s.snapshot().deck_themes).is_equal_to(vec![imported(&r)]);
    assert_that!(s.deck_only_theme_ref(r.id)).is_equal_to(Some(r));
}

#[test]
fn deck_only_themes_gives_one_entry_per_id_its_leaf_version() {
    let data = fresh_dir("oku-slides-deck-themes-leaf-");
    let mut s = opened_on(data.path());
    let r = adopt_deck_only_theme(&mut s);
    s.edit_theme(r.id, None, vec![accent("#c0392b")]).unwrap();
    let leaf = s.edit_theme(r.id, None, vec![accent("#2980b9")]).unwrap();
    s.set_theme(PAPIER_THEME).unwrap();
    let leaf = ThemeRef { id: r.id, version: leaf.version };
    assert_that!(s.deck_only_themes()).is_equal_to(vec![imported(&leaf)]);
}

#[test]
fn deck_only_themes_gives_the_version_the_deck_uses_over_a_newer_leaf() {
    let data = fresh_dir("oku-slides-deck-themes-used-");
    let mut s = opened_on(data.path());
    let r = adopt_deck_only_theme(&mut s);
    embed_child(&mut s, &r, "2099-01-01T00:00:00Z");
    assert_that!(s.deck_only_themes()).is_equal_to(vec![imported(&r)]);
}

#[test]
fn deck_only_themes_breaks_a_tie_between_leaves_by_newest_created() {
    let data = fresh_dir("oku-slides-deck-themes-tie-");
    let mut s = opened_on(data.path());
    let r = adopt_deck_only_theme(&mut s);
    embed_child(&mut s, &r, "2026-01-01T00:00:00Z");
    let newest = embed_child(&mut s, &r, "2026-02-01T00:00:00Z");
    s.set_theme(PAPIER_THEME).unwrap();
    assert_that!(s.deck_only_theme_ref(r.id)).is_equal_to(Some(newest));
}

#[test]
fn deck_only_themes_leaves_out_library_themes() {
    let data = fresh_dir("oku-slides-deck-themes-library-");
    let s = opened_on(data.path());
    assert_that!(s.deck_only_themes()).is_empty();
}

#[test]
fn set_theme_of_an_embedded_deck_only_theme_readopts_it_and_leaves_the_library_unchanged() {
    let data = fresh_dir("oku-slides-deck-themes-repick-");
    let mut s = opened_on(data.path());
    let r = adopt_deck_only_theme(&mut s);
    s.set_theme(PAPIER_THEME).unwrap();
    let (themes, files) = (s.themes.themes(), theme_library_files(data.path()));
    s.set_theme(r.id).unwrap();
    assert_that!(s.deck().theme.clone()).is_equal_to(Some(r.clone()));
    assert_that!(s.theme_manifest(&r).map(|m| m.scope)).is_equal_to(Some(PackScope::Deck));
    assert_that!(s.themes.themes()).is_equal_to(themes);
    assert_that!(s.themes.versions(r.id).is_empty()).is_true();
    assert_that!(theme_library_files(data.path())).is_equal_to(files);
}

#[test]
fn set_theme_of_an_unknown_id_is_refused_as_unknown_theme() {
    let data = fresh_dir("oku-slides-deck-themes-unknown-");
    let mut s = opened_on(data.path());
    let before = s.deck().theme.clone();
    let err = s.set_theme(Uuid::new_v4()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownTheme);
    assert_that!(s.deck().theme.clone()).is_equal_to(before);
}

#[test]
fn deck_only_themes_falls_back_to_the_newest_created_when_a_parent_cycle_leaves_no_leaf() {
    let data = fresh_dir("oku-slides-deck-themes-cycle-");
    let mut s = opened_on(data.path());
    let r = adopt_deck_only_theme(&mut s);
    let child = embed_child(&mut s, &r, "2099-01-01T00:00:00Z");
    s.set_theme(PAPIER_THEME).unwrap();
    // Des sources incohérentes : la racine se déclare enfant de son enfant.
    let mut root = s.theme_manifest(&r).unwrap();
    root.parent = Some(child.version.clone());
    s.doc.sources.themes.insert(r.version.clone(), root.to_files().unwrap());
    assert_that!(s.deck_only_theme_ref(r.id)).is_equal_to(Some(child));
}
