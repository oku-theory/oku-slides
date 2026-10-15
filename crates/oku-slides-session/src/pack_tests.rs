use speculoos::prelude::*;

use super::*;

use crate::test_support::manifest;

#[test]
fn pack_info_of_marks_the_default_pack() {
    let m = manifest(DEFAULT_PACK, "Essentiel", None, "2026-10-01T00:00:00Z");
    let v = m.to_files(std::collections::BTreeMap::new()).unwrap().version();
    let info = PackInfo::of(&m, v.clone(), true, false);
    assert_that!((info.id, info.version, info.default, info.builtin)).is_equal_to((DEFAULT_PACK, v, true, true));
}

#[test]
fn pack_info_of_counts_the_templates_of_a_user_pack() {
    let mut m = manifest(uuid::Uuid::new_v4(), "Mien", None, "2026-10-01T00:00:00Z");
    m.templates.push(Template {
        id: "texte".into(),
        name: "Texte".into(),
        description: String::new(),
        slide: oku_slides_core::Slide::new("s-texte", "content"),
    });
    let v = m.to_files(std::collections::BTreeMap::new()).unwrap().version();
    let info = PackInfo::of(&m, v, false, true);
    assert_that!((info.default, info.builtin, info.received, info.count)).is_equal_to((false, false, true, 1));
}
