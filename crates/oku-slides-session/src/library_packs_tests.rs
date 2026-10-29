use oku_slides_core::PackKind;
use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::DraftKind;
use crate::SessionErrorKind;
use crate::pack::{BASE_PACK, ESSENTIEL_PACK, PackManifest};
use crate::pack_lease::PackLease;
use crate::test_support::{add_user_theme, add_version_of, fresh_dir, manifest_ns, pack_library};
use crate::test_support_packs::assert_unleased;
use crate::theme::PAPIER_NAMESPACE;

#[test]
fn kind_of_builtins_is_their_kind() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    assert_that!((lib.kind_of(BASE_PACK), lib.kind_of(ESSENTIEL_PACK)))
        .is_equal_to((PackKind::Widgets, PackKind::Layouts));
}

#[test]
fn kind_of_a_user_layout_pack_is_layouts() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let m = manifest_ns(Uuid::new_v4(), "acme", &[]);
    add_version_of(&lib, PackKind::Layouts, &m, &[]);
    assert_that!((lib.kind_of(m.id), lib.current_version(m.id).map(|s| s.files.kind())))
        .is_equal_to((PackKind::Layouts, Some(PackKind::Layouts)));
}

#[test]
fn add_version_of_another_kind_than_earlier_versions_is_refused() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let m = manifest_ns(Uuid::new_v4(), "acme", &[]);
    add_version_of(&lib, PackKind::Widgets, &m, &[]);
    let files = m.to_files(PackKind::Layouts, std::collections::BTreeMap::default()).unwrap();
    let err = lib.add_version(m.id, &files).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackKindChanged);
}

#[test]
fn pack_kind_changed_message_is_localized_in_french() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let m = manifest_ns(Uuid::new_v4(), "acme", &[]);
    add_version_of(&lib, PackKind::Layouts, &m, &[]);
    let err = lib
        .add_version(m.id, &m.to_files(PackKind::Widgets, std::collections::BTreeMap::default()).unwrap())
        .unwrap_err();
    let fr = oku_slides_i18n::Localize::localize(&err, &oku_slides_i18n::Localizer::new(Some("fr")));
    assert_that!(fr).is_not_equal_to(oku_slides_i18n::Localize::localize(&err, oku_slides_i18n::english()));
}

#[test]
fn packs_of_each_kind_list_only_that_kind() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    add_version_of(&lib, PackKind::Layouts, &manifest_ns(Uuid::new_v4(), "acme", &[]), &[]);
    let kinds: Vec<Vec<PackKind>> =
        PackKind::ALL.iter().map(|k| lib.packs(*k).iter().map(|p| p.kind).collect()).collect();
    assert_that!(kinds).is_equal_to(vec![vec![PackKind::Widgets], vec![PackKind::Layouts, PackKind::Layouts]]);
}

/// The default name of a new pack, or theme, in `locale`.
fn default_name(key: &str, locale: &str) -> String {
    oku_slides_i18n::Localizer::new(Some(locale)).tr(key)
}

/// `lib` creates an empty pack of `kind` named `name`, which must work: its id.
fn created(lib: &super::PackLibrary, kind: PackKind, name: &str) -> Uuid {
    lib.create_empty_pack(kind, name).unwrap_or_else(|e| panic!("new pack: {}", e.one_line()))
}

/// `lib` creates an empty theme named `name`, which must work: its id.
fn created_theme(lib: &super::PackLibrary, name: &str) -> Uuid {
    lib.create_empty_theme(name).unwrap_or_else(|e| panic!("new theme: {}", e.one_line()))
}

/// The namespace of pack `id`'s current version.
fn namespace_of(lib: &super::PackLibrary, id: Uuid) -> String {
    lib.current_version(id).map(|s| s.manifest.namespace.to_string()).unwrap_or_default()
}

/// The namespace of theme `id`'s current version.
fn theme_namespace_of(lib: &super::PackLibrary, id: Uuid) -> String {
    lib.themes().current_version(id).map(|s| s.manifest.namespace.to_string()).unwrap_or_default()
}

#[rstest]
#[case::widgets(PackKind::Widgets)]
#[case::layouts(PackKind::Layouts)]
fn create_empty_pack_writes_current_unadopted_version(#[case] kind: PackKind) {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let id = created(&lib, kind, "My pack");
    let current = lib.current_version(id).unwrap_or_else(|| panic!("a current version"));
    let facts = (current.files.kind(), current.files.widgets().len(), lib.versions(id).len());
    assert_that!(facts).is_equal_to((kind, 0, 1));
    assert_that!(lib.is_frozen(id, &current.version)).is_false();
}

#[test]
fn create_empty_pack_twice_gets_suffixed_namespaces() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let (a, b) = (created(&lib, PackKind::Widgets, "My pack"), created(&lib, PackKind::Layouts, "My pack"));
    assert_that!((namespace_of(&lib, a), namespace_of(&lib, b)))
        .is_equal_to(("my-pack".to_owned(), "my-pack-2".to_owned()));
}

#[rstest]
#[case::fr("fr", "mon-pack")]
#[case::ja("ja", "pack")]
#[case::ar("ar", "pack")]
#[case::ru("ru", "pack")]
fn create_empty_pack_namespace_parses_in_every_script(#[case] locale: &str, #[case] expected: &str) {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let id = created(&lib, PackKind::Widgets, &default_name("pack-default-name", locale));
    assert_that!(namespace_of(&lib, id)).is_equal_to(expected.to_owned());
}

#[test]
fn create_empty_theme_namespace_falls_back_to_theme() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let id = created_theme(&lib, &default_name("theme-default-name", "ja"));
    assert_that!(theme_namespace_of(&lib, id)).is_equal_to("theme".to_owned());
}

#[test]
fn create_empty_pack_namespace_avoids_theme_namespaces() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    add_user_theme(lib.themes(), Uuid::new_v4(), "my-pack", &[PAPIER_NAMESPACE], serde_json::Map::new(), None);
    let id = created(&lib, PackKind::Widgets, "My pack");
    assert_that!(namespace_of(&lib, id)).is_equal_to("my-pack-2".to_owned());
}

#[test]
fn create_empty_theme_namespace_avoids_pack_namespaces() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    created(&lib, PackKind::Layouts, "My theme");
    let id = created_theme(&lib, "My theme");
    assert_that!(theme_namespace_of(&lib, id)).is_equal_to("my-theme-2".to_owned());
}

#[test]
fn create_empty_pack_leaves_no_lease() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let id = created(&lib, PackKind::Widgets, "My pack");
    assert_unleased(lib.lease_scope().unwrap_or_else(|| panic!("a lease scope")), id);
}

#[test]
fn create_empty_theme_leaves_no_lease() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let id = created_theme(&lib, "My theme");
    assert_unleased(lib.theme_scope().unwrap_or_else(|| panic!("a theme lease scope")), id);
}

#[test]
fn create_empty_theme_extends_default_theme() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let id = created_theme(&lib, "My theme");
    let manifest = lib.themes().current_version(id).map(|s| s.manifest);
    let shape = manifest.map(|m| (m.extends.iter().map(ToString::to_string).collect::<Vec<_>>(), m.tokens.len()));
    assert_that!(shape).is_equal_to(Some((vec![PAPIER_NAMESPACE.to_owned()], 0)));
}

#[test]
fn create_empty_theme_passes_check_theme() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let id = created_theme(&lib, "My theme");
    let ns = lib.themes().current_version(id).map_or_else(|| panic!("theme"), |s| s.manifest.namespace);
    assert_that!(lib.check_theme(id, &ns).map_err(|e| e.kind())).is_equal_to(Ok(()));
}

#[test]
fn create_empty_pack_concurrent_calls_get_distinct_namespaces() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let threads: Vec<_> = (0..2)
        .map(|_| {
            let lib = pack_library(dir.path());
            std::thread::spawn(move || created(&lib, PackKind::Widgets, "My pack"))
        })
        .collect();
    let ids: Vec<Uuid> = threads.into_iter().map(|t| t.join().unwrap_or_else(|_| panic!("thread"))).collect();
    let mut namespaces: Vec<String> = ids.iter().map(|id| namespace_of(&lib, *id)).collect();
    namespaces.sort();
    assert_that!(namespaces).is_equal_to(vec!["my-pack".to_owned(), "my-pack-2".to_owned()]);
}

#[test]
fn discard_untouched_deletes_a_single_version_draft() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let id = created(&lib, PackKind::Layouts, "My pack");
    let deleted = lib.discard_untouched(DraftKind::Pack, id).map_err(|e| e.kind());
    assert_that!((deleted, lib.current(id))).is_equal_to((Ok(true), None));
}

#[test]
fn discard_untouched_deletes_a_single_version_theme() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let id = created_theme(&lib, "My theme");
    let deleted = lib.discard_untouched(DraftKind::Theme, id).map_err(|e| e.kind());
    assert_that!((deleted, lib.themes().current(id))).is_equal_to((Ok(true), None));
}

#[test]
fn discard_untouched_keeps_a_saved_draft() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let id = created(&lib, PackKind::Widgets, "My pack");
    let first = lib.current_version(id).unwrap_or_else(|| panic!("a version"));
    let saved = PackManifest { name: "Saved".into(), parent: Some(first.version), ..first.manifest };
    lib.add_version(id, &saved.to_files(PackKind::Widgets, std::collections::BTreeMap::new()).unwrap()).unwrap();
    let kept = lib.discard_untouched(DraftKind::Pack, id).map_err(|e| e.kind());
    assert_that!((kept, lib.versions(id).len())).is_equal_to((Ok(false), 2));
}

#[test]
fn discard_untouched_keeps_a_leased_draft() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let id = created(&lib, PackKind::Widgets, "My pack");
    let scope = lib.lease_scope().cloned().unwrap_or_else(|| panic!("a lease scope"));
    let lease = PackLease::acquire(&scope, id, "My pack", "t-9").unwrap_or_else(|e| panic!("{}", e.one_line()));
    let kept = lib.discard_untouched(DraftKind::Pack, id).map_err(|e| e.kind());
    drop(lease);
    assert_that!((kept, lib.current(id).is_some())).is_equal_to((Ok(false), true));
}

#[test]
fn discard_untouched_of_a_missing_id_is_ok() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    assert_that!(lib.discard_untouched(DraftKind::Pack, Uuid::new_v4()).map_err(|e| e.kind())).is_equal_to(Ok(false));
}

#[test]
fn discard_untouched_never_deletes_a_builtin() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let kept = lib.discard_untouched(DraftKind::Theme, crate::theme::PAPIER_THEME).map_err(|e| e.kind());
    assert_that!((kept, lib.themes().current(crate::theme::PAPIER_THEME).is_some())).is_equal_to((Ok(false), true));
}
