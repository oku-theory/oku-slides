use speculoos::prelude::*;

use super::*;
use crate::ImportErrorKind;

#[test]
fn parse_over_node_limit_is_too_large() {
    let doc = format!("<r>{}</r>", "<a/>".repeat(50));
    assert_that!(parse(&doc, "p.xml", 10).map(|_| ()).map_err(|e| e.kind()).err())
        .is_equal_to(Some(ImportErrorKind::TooLarge));
}

#[test]
fn parse_ten_thousand_levels_is_refused_without_recursing() {
    let doc = format!("{}{}", "<a x='>'>".repeat(10_000), "</a>".repeat(10_000));
    assert_that!(parse(&doc, "p.xml", 1_000_000).map(|_| ()).map_err(|e| e.kind()).err())
        .is_equal_to(Some(ImportErrorKind::Unreadable));
}

#[test]
fn parse_at_the_depth_limit_with_comment_cdata_and_pi_is_accepted() {
    let inner = "<!-- <a> --><![CDATA[<a>]]><?pi <a>?><b/>";
    let doc = format!("{}{inner}{}", "<a>".repeat(MAX_XML_DEPTH - 1), "</a>".repeat(MAX_XML_DEPTH - 1));
    assert_that!(parse(&doc, "p.xml", 1_000_000).is_ok()).is_true();
}

#[test]
fn depth_counts_open_elements_not_self_closing_or_quoted_angle_brackets() {
    // a (1) > c (2) ; b et d sont auto-fermants ; le « /> » entre guillemets ne ferme rien.
    assert_that!(max_depth("<a q=\"/>\"><b/><c><d/></c></a>")).is_equal_to(2);
}

const NS_R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn doc(xml: &str) -> roxmltree::Document<'_> {
    parse(xml, "t", 10_000).unwrap()
}

#[test]
fn child_finds_by_local_name_whatever_the_prefix() {
    let d = doc(r#"<p:sp xmlns:p="urn:p" xmlns:a="urn:a"><p:spPr><a:xfrm><a:off x="12700"/></a:xfrm></p:spPr></p:sp>"#);
    assert_that!(child(d.root_element(), "spPr").map(|n| n.tag_name().name())).is_equal_to(Some("spPr"));
    assert_that!(descendant(d.root_element(), "off").and_then(|n| attr_i64(n, "x"))).is_equal_to(Some(12700));
}

#[test]
fn attr_r_skips_the_plain_id_written_first() {
    let xml = format!(r#"<p:sldId xmlns:p="urn:p" xmlns:r="{NS_R}" id="256" r:id="rId2"/>"#);
    let d = doc(&xml);
    assert_that!((attr_r(d.root_element(), "id"), attr_str(d.root_element(), "id")))
        .is_equal_to((Some("rId2"), Some("256")));
}

#[test]
fn attr_r_accepts_the_strict_relationships_namespace() {
    let d = doc(r#"<b xmlns:r="http://purl.oclc.org/ooxml/officeDocument/relationships" r:embed="rId3"/>"#);
    assert_that!(attr_r(d.root_element(), "embed")).is_equal_to(Some("rId3"));
}

#[test]
fn attr_r_ignores_other_namespaces() {
    let d = doc(r#"<b xmlns:x="urn:x" x:embed="rId3"/>"#);
    assert_that!(attr_r(d.root_element(), "embed")).is_none();
}

#[test]
fn attr_bool_reads_ooxml_booleans() {
    let d = doc(r#"<x a="1" b="false" c="on" d="maybe"/>"#);
    let n = d.root_element();
    assert_that!((attr_bool(n, "a"), attr_bool(n, "b"), attr_bool(n, "c"), attr_bool(n, "d"), attr_bool(n, "e")))
        .is_equal_to((Some(true), Some(false), Some(true), None, None));
}

#[test]
fn attr_f64_rejects_non_finite_values() {
    let d = doc(r#"<x a="NaN" b="inf" c="1.5"/>"#);
    let n = d.root_element();
    assert_that!((attr_f64(n, "a"), attr_f64(n, "b"), attr_f64(n, "c"))).is_equal_to((None, None, Some(1.5)));
}

#[test]
fn attr_u32_rejects_negative_and_too_large_values() {
    let d = doc(r#"<x a="-1" b="4294967296" c="7"/>"#);
    let n = d.root_element();
    assert_that!((attr_u32(n, "a"), attr_u32(n, "b"), attr_u32(n, "c"))).is_equal_to((None, None, Some(7)));
}

#[test]
fn units_convert_emu_angles_percents_and_hundredths() {
    use crate::pptx::units::{angle, emu, hundredths, pct};
    assert_that!((emu(12700), angle(5_400_000), pct(50_000), hundredths(1800))).is_equal_to((1.0, 90.0, 0.5, 18.0));
}
