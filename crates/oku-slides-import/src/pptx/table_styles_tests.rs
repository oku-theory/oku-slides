use std::collections::BTreeSet;

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::pptx::xml::{attr_str, elements, parse};

#[test]
fn builtin_guids_are_distinct() {
    let ids: BTreeSet<String> = BUILTIN.iter().map(|(g, ..)| g.to_ascii_uppercase()).collect();
    assert_that!(ids.len()).is_equal_to(BUILTIN.len());
}

#[test]
fn every_builtin_style_is_a_well_formed_table_style() {
    for (guid, ..) in BUILTIN {
        let xml = builtin(guid).unwrap();
        let doc = parse(&xml, "t", 10_000).unwrap_or_else(|e| panic!("{guid} : {e:?}"));
        let root = doc.root_element();
        let parts: Vec<&str> = elements(root).map(|p| p.tag_name().name()).collect();
        assert_that!((attr_str(root, "styleId"), parts.contains(&"wholeTbl"), parts.contains(&"firstRow")))
            .is_equal_to((Some(guid), true, true));
    }
}

#[test]
fn unknown_guid_has_no_builtin_style() {
    assert_that!(builtin("{00000000-0000-0000-0000-000000000000}")).is_none();
}

#[rstest]
#[case("{073A0DAA-6AF3-43AB-8588-CEC1D06C72B9}", r#"<a:schemeClr val="dk1"/>"#)]
#[case("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}", r#"<a:schemeClr val="accent1"/>"#)]
#[case("{0660B408-B3CF-4A94-85FC-2B1E0A45F4A2}", r#"<a:schemeClr val="accent2"/>"#)]
fn header_fill_follows_the_accent_or_the_base_color(#[case] guid: &str, #[case] header: &str) {
    let xml = builtin(guid).unwrap();
    let first_row = &xml[xml.find("<a:firstRow>").unwrap()..];
    assert_that!(first_row).contains(&*format!("<a:solidFill>{header}</a:solidFill>"));
}
