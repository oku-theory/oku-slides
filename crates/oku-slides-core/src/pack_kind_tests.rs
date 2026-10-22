use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use crate::{Category, PackKind};

#[rstest]
#[case(PackKind::Widgets, "widgets.json", "widgets", Category::Element)]
#[case(PackKind::Layouts, "layouts.json", "layouts", Category::Slide)]
fn pack_kind_names_its_files_and_category(
    #[case] kind: PackKind,
    #[case] manifest: &str,
    #[case] dir: &str,
    #[case] category: Category,
) {
    assert_that!((kind.manifest_file(), kind.files_dir(), kind.category())).is_equal_to((manifest, dir, category));
}

#[rstest]
#[case(json!("widgets"), PackKind::Widgets)]
#[case(json!("layouts"), PackKind::Layouts)]
fn pack_kind_from_json_reads_both(#[case] v: serde_json::Value, #[case] expected: PackKind) {
    assert_that!(serde_json::from_value::<PackKind>(v).unwrap()).is_equal_to(expected);
}

#[test]
fn pack_kind_from_json_other_value_is_refused() {
    assert_that!(serde_json::from_value::<PackKind>(json!("themes"))).is_err();
}

#[test]
fn pack_kind_all_lists_both_kinds_in_order() {
    assert_that!(PackKind::ALL.to_vec()).is_equal_to(vec![PackKind::Widgets, PackKind::Layouts]);
}
