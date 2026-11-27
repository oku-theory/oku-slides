use speculoos::prelude::*;

use super::*;

#[test]
fn level_of_title_and_headings_by_id_or_name() {
    assert_that!(level_of("Title")).is_equal_to(Some(1));
    assert_that!(level_of("Heading3")).is_equal_to(Some(3));
    assert_that!(level_of("heading 6")).is_equal_to(Some(6));
}

#[test]
fn level_of_other_styles_none() {
    for name in ["Normal", "Heading7", "Heading0", "Titre1", "headingX"] {
        assert_that!(level_of(name)).named(name).is_none();
    }
}

#[test]
fn on_property_toggled_by_its_value() {
    let xml = r#"<rPr><b/><i val="0"/><vanish val="false"/><caps val="true"/></rPr>"#;
    let doc = roxmltree::Document::parse(xml).unwrap();
    let props = Some(doc.root_element());
    assert_that!(on(props, "b")).is_true();
    assert_that!(on(props, "i")).is_false();
    assert_that!(on(props, "vanish")).is_false();
    assert_that!(on(props, "caps")).is_true();
    assert_that!(on(props, "strike")).is_false();
    assert_that!(on(None, "b")).is_false();
}

#[test]
fn toggle_unset_property_none() {
    let xml = r#"<rPr><vanish val="off"/><specVanish/></rPr>"#;
    let doc = roxmltree::Document::parse(xml).unwrap();
    let props = Some(doc.root_element());
    assert_that!(toggle(props, "vanish")).is_equal_to(Some(false));
    assert_that!(toggle(props, "specVanish")).is_equal_to(Some(true));
    assert_that!(toggle(props, "b")).is_none();
}

#[test]
fn inherits_hidden_follows_base_until_set_and_stops_on_cycle() {
    let own: BTreeMap<&str, (Option<bool>, Option<&str>)> = BTreeMap::from([
        ("Base", (Some(true), None)),
        ("Child", (None, Some("Base"))),
        ("Shown", (Some(false), Some("Base"))),
        ("LoopA", (None, Some("LoopB"))),
        ("LoopB", (None, Some("LoopA"))),
    ]);
    assert_that!(inherits_hidden(&own, "Child")).is_true();
    assert_that!(inherits_hidden(&own, "Shown")).is_false();
    assert_that!(inherits_hidden(&own, "LoopA")).is_false();
    assert_that!(inherits_hidden(&own, "Missing")).is_false();
}
