use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::Storage;
use crate::test_support::{fresh_dir, library_with_raw_theme, ns};

fn temp_theme_library() -> (ThemeLibrary, tempfile::TempDir) {
    let dir = fresh_dir("oku-slides-themes-");
    (ThemeLibrary::new(Some(dir.path().to_owned()), Storage::local().unwrap()), dir)
}

fn theme(name: &str, namespace: &str, created: &str) -> ThemeManifest {
    ThemeManifest {
        id: Uuid::new_v4(),
        name: name.into(),
        namespace: ns(namespace),
        extends: Vec::new(),
        description: String::new(),
        parent: None,
        created: created.into(),
        tokens: json!({"color": {"accent": "#ff0000"}}).as_object().cloned().unwrap(),
    }
}

#[test]
fn builtin_themes_have_constant_ids_in_order() {
    let ids: Vec<Uuid> = Themes::builtins().iter().map(|b| b.manifest.id).collect();
    assert_that!(ids).is_equal_to(vec![PAPIER_THEME, PRO_THEME, NEUTRE_THEME]);
}

#[test]
fn builtin_themes_are_papier_pro_neutre_with_namespaces() {
    let ns: Vec<_> =
        ThemeLibrary::builtin_only().themes().into_iter().map(|t| t.namespace.as_str().to_owned()).collect();
    assert_that!(ns).is_equal_to(vec!["papier".to_owned(), "pro".to_owned(), "neutre".to_owned()]);
}

#[rstest::rstest]
#[case::papier(PAPIER_NAMESPACE, PAPIER_THEME)]
#[case::pro(PRO_NAMESPACE, PRO_THEME)]
#[case::neutre(NEUTRE_NAMESPACE, NEUTRE_THEME)]
fn builtin_namespace_constants_name_their_builtin(#[case] namespace: &str, #[case] id: Uuid) {
    let found = ThemeLibrary::builtin_only().by_namespace(&ns(namespace)).map(|s| s.manifest.id);
    assert_that!(found).is_equal_to(Some(id));
}

#[test]
fn builtin_themes_extend_nothing() {
    let extends: Vec<usize> = Themes::builtins().iter().map(|b| b.manifest.extends.len()).collect();
    assert_that!(extends).is_equal_to(vec![0, 0, 0]);
}

#[test]
fn builtin_theme_is_current_and_listed_first() {
    let (lib, _dir) = temp_theme_library();
    let names: Vec<String> = lib.themes().into_iter().map(|t| t.name).collect();
    assert_that!(names).is_equal_to(vec!["Papier".to_owned(), "Pro".to_owned(), "Neutre".to_owned()]);
}

#[test]
fn themes_flags_builtin_and_default() {
    let (lib, _dir) = temp_theme_library();
    let flags: Vec<(bool, bool)> = lib.themes().iter().map(|t| (t.builtin, t.default)).collect();
    assert_that!(flags).is_equal_to(vec![(true, true), (true, false), (true, false)]);
}

#[test]
fn user_theme_version_is_stored_under_themes_dir() {
    let (lib, dir) = temp_theme_library();
    let m = theme("Mien", "mien", "2026-10-05T10:00:00Z");
    lib.add_version(m.id, &m.to_files().unwrap()).unwrap();
    assert_that!(dir.path().join(m.id.to_string()).join("versions").exists()).is_true();
    assert_that!(lib.current(m.id).is_some()).is_true();
}

#[test]
fn themes_lists_user_themes_after_builtins_by_lowercase_name() {
    let (lib, _dir) = temp_theme_library();
    for (name, namespace) in [("zèbre", "zebre"), ("Abeille", "abeille")] {
        let m = theme(name, namespace, "2026-10-05T10:00:00Z");
        lib.add_version(m.id, &m.to_files().unwrap()).unwrap();
    }
    let infos = lib.themes();
    let names: Vec<&str> = infos.iter().map(|t| t.name.as_str()).collect();
    assert_that!(names).is_equal_to(vec!["Papier", "Pro", "Neutre", "Abeille", "zèbre"]);
    assert_that!(infos[3].builtin).is_false();
    assert_that!(infos[3].namespace.as_str()).is_equal_to("abeille");
}

#[test]
fn theme_info_carries_the_extends_of_its_version() {
    let (lib, _dir) = temp_theme_library();
    let m = ThemeManifest { extends: vec![ns("papier")], ..theme("Marque", "marque", "2026-10-05T10:00:00Z") };
    lib.add_version(m.id, &m.to_files().unwrap()).unwrap();
    let info = lib.themes().into_iter().find(|t| t.id == m.id).unwrap();
    assert_that!(info.extends).is_equal_to(vec![ns("papier")]);
}

#[test]
fn theme_manifest_round_trips_through_files() {
    let m = theme("Mien", "mien", "2026-10-05T10:00:00Z");
    let files = m.to_files().unwrap();
    assert_that!(ThemeManifest::from_files(&files).unwrap()).is_equal_to(m);
    assert_that!(files.widgets().is_empty()).is_true();
}

#[rstest::rstest]
#[case::builtin_theme("pro")]
#[case::builtin_pack("base")]
#[case::reserved("deck")]
fn user_theme_under_a_taken_namespace_is_refused_and_writes_nothing(#[case] namespace: &str) {
    let (lib, dir) = temp_theme_library();
    let m = theme("Autre", namespace, "2026-10-05T10:00:00Z");
    assert_that!(lib.add_version(m.id, &m.to_files().unwrap())).is_err();
    assert_that!(dir.path().join(m.id.to_string()).exists()).is_false();
}

#[test]
fn user_theme_without_namespace_is_skipped_and_counted() {
    let (themes, _dir) = library_with_raw_theme(crate::test_support::OLD_THEME_JSON);
    assert_that!(themes.themes().iter().any(|t| t.name == "Old")).is_false();
    assert_that!(themes.skipped()).is_equal_to(1);
}

#[test]
fn library_without_user_themes_skips_nothing() {
    let (lib, _dir) = temp_theme_library();
    let m = theme("Mien", "mien", "2026-10-05T10:00:00Z");
    lib.add_version(m.id, &m.to_files().unwrap()).unwrap();
    assert_that!(lib.skipped()).is_equal_to(0);
}
