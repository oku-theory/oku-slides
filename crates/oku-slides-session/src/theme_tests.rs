use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::Storage;
use crate::test_support::fresh_dir;

fn temp_theme_library() -> (ThemeLibrary, tempfile::TempDir) {
    let dir = fresh_dir("oku-slides-themes-");
    (ThemeLibrary::new(Some(dir.path().to_owned()), Storage::local().unwrap()), dir)
}

fn theme(name: &str, created: &str) -> ThemeManifest {
    ThemeManifest {
        id: Uuid::new_v4(),
        name: name.into(),
        description: String::new(),
        parent: None,
        created: created.into(),
        scope: oku_slides_core::PackScope::Library,
        tokens: json!({"color": {"accent": "#ff0000"}}).as_object().cloned().unwrap(),
    }
}

#[test]
fn builtin_themes_have_constant_ids_in_order() {
    let ids: Vec<Uuid> = Themes::builtins().iter().map(|b| b.manifest.id).collect();
    assert_that!(ids).is_equal_to(vec![PAPIER_THEME, PRO_THEME]);
}

#[test]
fn builtin_theme_is_current_and_listed_first() {
    let (lib, _dir) = temp_theme_library();
    let names: Vec<String> = lib.themes().into_iter().map(|t| t.name).collect();
    assert_that!(names).is_equal_to(vec!["Papier".to_owned(), "Pro".to_owned()]);
}

#[test]
fn themes_flags_builtin_and_default() {
    let (lib, _dir) = temp_theme_library();
    let flags: Vec<(bool, bool)> = lib.themes().iter().map(|t| (t.builtin, t.default)).collect();
    assert_that!(flags).is_equal_to(vec![(true, true), (true, false)]);
}

#[test]
fn user_theme_version_is_stored_under_themes_dir() {
    let (lib, dir) = temp_theme_library();
    let m = theme("Mien", "2026-10-05T10:00:00Z");
    lib.add_version(m.id, &m.to_files().unwrap()).unwrap();
    assert_that!(dir.path().join(m.id.to_string()).join("versions").exists()).is_true();
    assert_that!(lib.current(m.id).is_some()).is_true();
}

#[test]
fn themes_lists_user_themes_after_builtins_by_lowercase_name() {
    let (lib, _dir) = temp_theme_library();
    for name in ["zèbre", "Abeille"] {
        let m = theme(name, "2026-10-05T10:00:00Z");
        lib.add_version(m.id, &m.to_files().unwrap()).unwrap();
    }
    let infos = lib.themes();
    let names: Vec<&str> = infos.iter().map(|t| t.name.as_str()).collect();
    assert_that!(names).is_equal_to(vec!["Papier", "Pro", "Abeille", "zèbre"]);
    assert_that!(infos[2].builtin).is_false();
}

#[test]
fn theme_manifest_round_trips_through_files() {
    let m = theme("Mien", "2026-10-05T10:00:00Z");
    let files = m.to_files().unwrap();
    assert_that!(ThemeManifest::from_files(&files).unwrap()).is_equal_to(m);
    assert_that!(files.widgets().is_empty()).is_true();
}

#[rstest::rstest]
#[case::add(false)]
#[case::receive(true)]
fn theme_version_of_a_deck_only_theme_is_refused_and_writes_nothing(#[case] received: bool) {
    let (lib, dir) = temp_theme_library();
    let m = ThemeManifest { scope: oku_slides_core::PackScope::Deck, ..theme("Importé", "2026-10-01T10:00:00Z") };
    let files = m.to_files().unwrap();
    let result = if received { lib.receive_version(m.id, &files) } else { lib.add_version(m.id, &files) };
    assert_that!(result.map_err(|e| e.kind())).is_equal_to(Err(crate::SessionErrorKind::DeckOnly));
    assert_that!(dir.path().join(m.id.to_string()).exists()).is_false();
    assert_that!(lib.themes().iter().any(|t| t.id == m.id)).is_false();
}

#[test]
fn open_of_a_deck_with_a_deck_only_theme_adds_nothing_to_the_theme_library() {
    use crate::Request;
    use crate::test_support::{adopt_deck_only_theme, session_on};
    let (me, them) = (fresh_dir("oku-slides-theme-scope-me-"), fresh_dir("oku-slides-theme-scope-them-"));
    let mut other = session_on(them.path());
    let r = adopt_deck_only_theme(&mut other);
    let path = them.path().join("importe.oku-slides.pdf");
    other.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap();
    let mut s = session_on(me.path());
    let themes = s.themes.themes();
    s.open(&path).unwrap();
    assert_that!(s.deck().theme.clone()).is_equal_to(Some(r.clone()));
    assert_that!(s.themes.versions(r.id).is_empty()).is_true();
    assert_that!(s.themes.themes()).is_equal_to(&themes);
}
