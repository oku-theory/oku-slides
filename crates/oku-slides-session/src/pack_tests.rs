use oku_slides_core::PackKind;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

use crate::test_support::{manifest, widget_files};

#[rstest]
#[case(BASE_PACK)]
#[case(ESSENTIEL_PACK)]
fn pack_info_of_marks_both_builtins_as_default(#[case] id: uuid::Uuid) {
    let m = manifest(id, "Intégré", None, "2026-10-01T00:00:00Z");
    let files = m.to_files(PackKind::Widgets, std::collections::BTreeMap::new()).unwrap();
    let info = PackInfo::of(&files, &m, true, false);
    assert_that!((info.id, info.version, info.default, info.builtin)).is_equal_to((id, files.version(), true, true));
}

#[test]
fn pack_info_of_counts_the_layouts_and_profiles_of_a_user_layout_pack() {
    let m = manifest(uuid::Uuid::new_v4(), "Mien", None, "2026-10-01T00:00:00Z");
    let files = m.to_files(PackKind::Layouts, widget_files(&[("_commun", "C"), ("a", "A"), ("b", "B")])).unwrap();
    let info = PackInfo::of(&files, &m, false, true);
    assert_that!((info.kind, info.default, info.builtin, info.received, info.widgets, info.profiles)).is_equal_to((
        PackKind::Layouts,
        false,
        false,
        true,
        2,
        0,
    ));
}
