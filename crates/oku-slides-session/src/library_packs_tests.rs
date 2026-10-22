use oku_slides_core::PackKind;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::SessionErrorKind;
use crate::pack::{BASE_PACK, ESSENTIEL_PACK};
use crate::test_support::{add_version_of, fresh_dir, manifest_ns, pack_library};

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
    let files = m.to_files(PackKind::Layouts, Default::default()).unwrap();
    let err = lib.add_version(m.id, &files).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackKindChanged);
}

#[test]
fn pack_kind_changed_message_is_localized_in_french() {
    let dir = fresh_dir("oku-slides-library-packs-");
    let lib = pack_library(dir.path());
    let m = manifest_ns(Uuid::new_v4(), "acme", &[]);
    add_version_of(&lib, PackKind::Layouts, &m, &[]);
    let err = lib.add_version(m.id, &m.to_files(PackKind::Widgets, Default::default()).unwrap()).unwrap_err();
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
