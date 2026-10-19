//! Espaces de noms des packs de la bibliothèque : uniques, ceux des packs intégrés.

use std::path::Path;

use oku_slides_core::kinds;
use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::pack::{BASE_PACK, DEFAULT_PACK};
use crate::test_support::{add_version, files_of, fresh_dir, manifest_ns};

fn library(dir: &Path) -> PackLibrary {
    PackLibrary::new(Some(dir.to_owned()), Storage::local().unwrap())
}

#[test]
fn library_add_version_namespace_of_another_pack_is_refused() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    add_version(&lib, &manifest_ns(Uuid::new_v4(), "acme", &[]), &[]);
    let other = manifest_ns(Uuid::new_v4(), "acme", &[]);
    let err = lib.add_version(other.id, &files_of(&other)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::NamespaceTaken);
    assert_that!(dir.path().join(other.id.to_string()).exists()).is_false();
}

#[test]
fn library_add_version_namespace_of_a_builtin_is_refused() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let other = manifest_ns(Uuid::new_v4(), "essentiel", &[]);
    let err = lib.add_version(other.id, &files_of(&other)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::NamespaceTaken);
    let shown = oku_slides_i18n::Localize::localize(&err, &oku_slides_i18n::Localizer::new(Some("fr")));
    assert_that!(shown.as_str()).contains(DEFAULT_PACK.to_string().as_str());
}

#[test]
fn library_add_version_new_version_of_the_same_pack_keeps_its_namespace() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let first = manifest_ns(Uuid::new_v4(), "acme", &[]);
    let r = add_version(&lib, &first, &[]);
    let next = crate::pack::PackManifest { parent: Some(r.version), created: "2026-10-02T10:00:00Z".into(), ..first };
    assert_that!(lib.add_version(next.id, &files_of(&next)).unwrap()).is_true();
}

#[test]
fn library_builtins_have_their_namespaces() {
    let names: Vec<_> = builtin_versions().iter().map(|b| b.manifest.namespace.to_string()).collect();
    assert_that!(names).is_equal_to(vec!["essentiel".to_owned(), "grilles".to_owned(), "base".to_owned()]);
}

#[test]
fn library_builtin_base_has_its_id_and_seven_widget_files() {
    let base = builtin_versions().iter().find(|b| b.manifest.namespace == kinds::base::namespace()).unwrap();
    let files = base.files.widgets().keys().filter(|f| !f.is_common()).count();
    assert_that!((base.manifest.id, files)).is_equal_to((BASE_PACK, 7));
}

#[rstest]
#[case("base")]
#[case("essentiel")]
#[case("grilles")]
fn library_add_another_pack_claiming_a_builtin_namespace_is_refused(#[case] namespace: &str) {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let other = manifest_ns(Uuid::new_v4(), namespace, &[]);
    let err = lib.add_version(other.id, &files_of(&other)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::NamespaceTaken);
}

#[test]
fn library_namespace_of_a_pack_named_base_skips_the_builtin() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let slug = crate::namespace_slug::library_namespace(&lib, "Base").unwrap();
    assert_that!(slug.as_str()).is_not_equal_to("base");
}

#[test]
fn library_template_sets_leave_out_packs_without_templates() {
    let dir = fresh_dir("oku-slides-library-");
    let ids: Vec<Uuid> = library(dir.path()).template_sets().iter().map(|p| p.id).collect();
    assert_that!((ids.contains(&DEFAULT_PACK), ids.contains(&BASE_PACK))).is_equal_to((true, false));
}

#[test]
fn library_by_namespace_finds_the_owner_current_version() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let m = manifest_ns(Uuid::new_v4(), "acme", &[]);
    let r = add_version(&lib, &m, &[]);
    let found = lib.by_namespace(&m.namespace).map(|s| (s.manifest.id, s.version));
    assert_that!(found).is_equal_to(Some((m.id, r.version)));
    assert_that!(lib.by_namespace(&crate::test_support::ns("absent"))).is_none();
}

#[test]
fn library_add_version_library_scope_deck_namespace_is_refused() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let m = manifest_ns(Uuid::new_v4(), "deck", &[]);
    let err = lib.add_version(m.id, &files_of(&m)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::NamespaceReserved);
    assert_that!(dir.path().join(m.id.to_string()).exists()).is_false();
    let de = oku_slides_i18n::Localize::localize(&err, &oku_slides_i18n::Localizer::new(Some("de")));
    let en = oku_slides_i18n::Localize::localize(&err, &oku_slides_i18n::Localizer::new(Some("en")));
    assert_that!(de).is_not_equal_to(en);
}

#[test]
fn library_receive_version_library_scope_deck_namespace_is_refused() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let m = manifest_ns(Uuid::new_v4(), "deck", &[]);
    let err = lib.receive_version(m.id, &files_of(&m)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::NamespaceReserved);
    assert_that!(dir.path().join(m.id.to_string()).exists()).is_false();
}

/// Une nouvelle version de `first` (sa fille) sous l'espace de noms `namespace`.
fn renamed(first: &crate::pack::PackManifest, parent: PackVersion, namespace: &str) -> crate::pack::PackManifest {
    crate::pack::PackManifest {
        namespace: crate::test_support::ns(namespace),
        parent: Some(parent),
        created: "2026-10-02T10:00:00Z".into(),
        ..first.clone()
    }
}

#[test]
fn library_add_version_new_version_under_another_namespace_is_refused() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let first = manifest_ns(Uuid::new_v4(), "acme", &[]);
    let r = add_version(&lib, &first, &[]);
    let next = renamed(&first, r.version, "autre");
    let err = lib.add_version(next.id, &files_of(&next)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::NamespaceChanged);
    assert_that!(lib.versions(first.id)).has_length(1);
    let de = oku_slides_i18n::Localize::localize(&err, &oku_slides_i18n::Localizer::new(Some("de")));
    assert_that!(de.as_str()).contains("acme");
    assert_that!(de.as_str()).contains("autre");
}

#[test]
fn library_receive_version_of_a_builtin_under_another_namespace_is_refused() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let builtin = builtin_versions().iter().find(|b| b.manifest.id == DEFAULT_PACK).unwrap();
    let next = renamed(&builtin.manifest, builtin.version.clone(), "autre");
    let err = lib.receive_version(next.id, &files_of(&next)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::NamespaceChanged);
    assert_that!(dir.path().join(DEFAULT_PACK.to_string()).exists()).is_false();
}
