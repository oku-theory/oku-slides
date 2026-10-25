use std::fmt::Write as _;

use rstest::rstest;
use speculoos::prelude::*;

use super::{Clean, sanitize};
use crate::ImportErrorKind;
use crate::error::ImportError;

const NS: &str = r#"xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink""#;
const NODES: u32 = 10_000;
const BYTES: usize = 1 << 20;

fn svg(body: &str) -> String {
    format!(r#"<svg {NS} viewBox="0 0 10 10">{body}</svg>"#)
}

fn clean(text: &str) -> Clean {
    sanitize(text.as_bytes(), "ppt/media/image1.svg", NODES, BYTES).unwrap_or_else(|e| panic!("SVG refusé : {e:?}"))
}

fn rejected(text: &[u8]) -> ImportError {
    match sanitize(text, "ppt/media/image1.svg", NODES, BYTES) {
        Ok(c) => panic!("SVG accepté : {}", String::from_utf8_lossy(&c.bytes)),
        Err(e) => e,
    }
}

fn out(c: &Clean) -> String {
    String::from_utf8(c.bytes.clone()).unwrap()
}

#[test]
fn sanitize_harmless_svg_keeps_its_drawing_unaltered() {
    let c = clean(&svg(
        r##"<defs><linearGradient id="g"><stop offset="0" stop-color="red"/></linearGradient></defs><rect width="5" height="5" fill="url(#g)" style="stroke:url(#g)"/><use xlink:href="#g"/>"##,
    ));
    assert_that!(c.altered).is_false();
    assert_that!(out(&c)).is_equal_to(format!(
        r##"<svg {NS} viewBox="0 0 10 10"><defs><linearGradient id="g"><stop offset="0" stop-color="red"/></linearGradient></defs><rect width="5" height="5" fill="url(#g)" style="stroke:url(#g)"/><use xlink:href="#g"/></svg>"##
    ));
}

#[rstest]
#[case::script(svg("<script>alert(1)</script>"), "alert")]
#[case::script_cdata(svg("<script><![CDATA[alert(1)]]></script>"), "alert")]
#[case::foreign_object(
    svg(r#"<foreignObject><div xmlns="http://www.w3.org/1999/xhtml">evil</div></foreignObject>"#),
    "evil"
)]
#[case::iframe(svg(r#"<iframe src="https://evil.example/"/>"#), "evil")]
#[case::embed(svg(r#"<embed src="https://evil.example/"/>"#), "evil")]
#[case::animate_href(svg(r#"<a><animate attributeName="href" to="javascript:alert(1)"/></a>"#), "javascript")]
#[case::set_event(svg(r#"<set attributeName="onmouseover" to="evil()"/>"#), "evil")]
#[case::style_import(svg("<style>@import 'https://evil.example/a.css';</style>"), "evil")]
#[case::style_element_url(svg("<style>rect { fill: url(https://evil.example/p) }</style>"), "evil")]
#[case::style_after_comment(svg("<style><!---->@import url(https://evil.example/a.css)</style>"), "evil")]
#[case::style_after_pi(svg("<style>a{}<?x?>@import url(https://evil.example/a.css)</style>"), "evil")]
#[case::style_after_element(svg("<style>a{}<desc/>@import url(https://evil.example/a.css)</style>"), "evil")]
fn sanitize_hostile_svg_drops_the_threat_and_reports_it(#[case] text: String, #[case] needle: &str) {
    let c = clean(&text);
    assert_that!(c.altered).is_true();
    assert_that!(out(&c).contains(needle)).is_false();
    assert_that!(out(&c).starts_with("<svg ")).is_true();
}

/// Attribut risqué : seul lui disparaît, l'élément qui le porte et ses enfants restent.
#[rstest]
#[case::onload(format!(r#"<svg {NS} onload="alert(1)"/>"#), format!("<svg {NS}/>"))]
#[case::onclick_upper(svg(r#"<rect ONCLICK="alert(1)"/>"#), svg("<rect/>"))]
#[case::image_external(svg(r#"<image href="https://evil.example/x.png" width="2"/>"#), svg(r#"<image width="2"/>"#))]
#[case::xlink_javascript(svg(r#"<a xlink:href="javascript:alert(1)"><rect/></a>"#), svg("<a><rect/></a>"))]
#[case::use_external(svg(r#"<use xlink:href="https://evil.example/s.svg#a" x="1"/>"#), svg(r#"<use x="1"/>"#))]
#[case::data_svg(
    svg(r#"<image href="data:image/svg+xml,&lt;svg onload='evil()'/&gt;" width="2"/>"#),
    svg(r#"<image width="2"/>"#)
)]
#[case::style_url(svg(r#"<rect style="fill:url(http://evil.example/p)" x="1"/>"#), svg(r#"<rect x="1"/>"#))]
#[case::style_quoted_url(svg(r#"<rect style="fill:url( 'http://evil.example/p')" x="1"/>"#), svg(r#"<rect x="1"/>"#))]
#[case::fill_url(svg(r#"<rect fill="url(https://evil.example/p#g)" x="1"/>"#), svg(r#"<rect x="1"/>"#))]
#[case::style_escape(svg(r#"<rect style="fill:u\72l(http://evil.example/p)" x="1"/>"#), svg(r#"<rect x="1"/>"#))]
#[case::style_image_set(
    svg(r#"<rect style="fill:image-set('https://evil.example/p' 1x)" x="1"/>"#),
    svg(r#"<rect x="1"/>"#)
)]
#[case::style_image(svg(r#"<rect style="fill:image('https://evil.example/p')" x="1"/>"#), svg(r#"<rect x="1"/>"#))]
#[case::style_cross_fade(
    svg(r#"<rect style="fill:Cross-Fade('https://evil.example/a', red 50%)" x="1"/>"#),
    svg(r#"<rect x="1"/>"#)
)]
#[case::style_webkit_image_set(
    svg(r#"<rect style="fill:-webkit-image-set('https://evil.example/p' 1x)" x="1"/>"#),
    svg(r#"<rect x="1"/>"#)
)]
#[case::style_element(svg(r#"<rect style="fill:-moz-element(#a)" x="1"/>"#), svg(r#"<rect x="1"/>"#))]
#[case::style_src(svg(r#"<rect style="fill:src('https://evil.example/p')" x="1"/>"#), svg(r#"<rect x="1"/>"#))]
#[case::style_attr(svg(r#"<rect style="fill:attr(data-u type(&lt;url&gt;))" x="1"/>"#), svg(r#"<rect x="1"/>"#))]
fn sanitize_hostile_attribute_is_dropped_alone(#[case] text: String, #[case] expected: String) {
    let c = clean(&text);
    assert_that!(c.altered).is_true();
    assert_that!(out(&c)).is_equal_to(expected);
}

#[rstest]
#[case::use_of_use(svg(r##"<defs><rect id="a"/><g id="b"><use href="#a"/></g></defs><use href="#b"/>"##).into_bytes())]
#[case::use_of_itself(svg(r##"<g id="a"><use xlink:href="#a"/></g>"##).into_bytes())]
#[case::fan_out(svg(&format!(
    r#"<defs><g id="a">{}</g></defs>{}"#,
    "<rect/>".repeat(1_000),
    r##"<use href="#a"/>"##.repeat(11)
)).into_bytes())]
#[case::entities(
    br#"<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY a "aaaaaaaaaa"><!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;"><!ENTITY c "&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;">]><svg xmlns="http://www.w3.org/2000/svg"><text>&c;</text></svg>"#.to_vec()
)]
#[case::external_entity(
    br#"<!DOCTYPE svg [<!ENTITY x SYSTEM "file:///etc/passwd">]><svg xmlns="http://www.w3.org/2000/svg"><text>&x;</text></svg>"#.to_vec()
)]
#[case::parameter_entity(
    br#"<!DOCTYPE svg [<!ENTITY % p "x">]><svg xmlns="http://www.w3.org/2000/svg"/>"#.to_vec()
)]
#[case::markup_entity(
    br#"<!DOCTYPE svg [<!ENTITY a "<script/>">]><svg xmlns="http://www.w3.org/2000/svg"><g>&a;</g></svg>"#.to_vec()
)]
#[case::entity_outside_doctype(
    br#"<!DOCTYPE svg [<!ENTITY a "x">]><svg xmlns="http://www.w3.org/2000/svg"><!-- <!ENTITY b "y"> --></svg>"#.to_vec()
)]
#[case::too_many_entities(format!(
    r#"<!DOCTYPE svg [{}]><svg xmlns="http://www.w3.org/2000/svg"/>"#,
    (0..65).fold(String::new(), |mut out, i| {
        let _ = write!(out, r#"<!ENTITY e{i} "x">"#);
        out
    })
).into_bytes())]
#[case::linear_expansion_over_the_output(format!(
    r#"<!DOCTYPE svg [<!ENTITY a "{}">]><svg xmlns="http://www.w3.org/2000/svg"><text>{}</text></svg>"#,
    "a".repeat(1_000),
    "&a;".repeat(1_100)
).into_bytes())]
#[case::attlist(
    br#"<!DOCTYPE svg [<!ATTLIST svg onload CDATA "evil()">]><svg xmlns="http://www.w3.org/2000/svg"/>"#.to_vec()
)]
#[case::deep(format!(r#"<svg xmlns="http://www.w3.org/2000/svg">{}{}</svg>"#, "<g>".repeat(300), "</g>".repeat(300)).into_bytes())]
#[case::malformed(br#"<svg xmlns="http://www.w3.org/2000/svg"><rect"#.to_vec())]
#[case::no_namespace(b"<svg><rect/></svg>".to_vec())]
#[case::not_utf8(b"<svg xmlns=\"http://www.w3.org/2000/svg\">\xff\xfe</svg>".to_vec())]
fn sanitize_unsafe_or_unreadable_svg_is_rejected_as_unreadable(#[case] text: Vec<u8>) {
    assert_that!(rejected(&text).kind()).is_equal_to(ImportErrorKind::Unreadable);
}

#[test]
fn sanitize_use_within_the_node_limit_is_kept() {
    let text =
        svg(&format!(r#"<defs><g id="a">{}</g></defs>{}"#, "<rect/>".repeat(1_000), r##"<use href="#a"/>"##.repeat(9)));
    assert_that!(clean(&text).altered).is_false();
}

/// Sortie de `text` assainie sous la borne d'octets `bytes`.
fn bounded(text: &str, bytes: usize) -> Result<Clean, ImportError> {
    sanitize(text.as_bytes(), "ppt/media/image1.svg", NODES, bytes)
}

#[test]
fn sanitize_output_at_the_part_limit_is_kept() {
    let text = svg(&format!("<text>{}</text>", ">".repeat(1_000)));
    let size = out(&clean(&text)).len();
    assert_that!(bounded(&text, size).map(|c| c.bytes.len()).ok()).is_equal_to(Some(size));
}

#[test]
fn sanitize_output_one_byte_over_the_part_limit_is_too_large() {
    // Chaque `>` du texte devient `&gt;` : la sortie quadruple ce que l'entrée en contient.
    let text = svg(&format!("<text>{}</text>", ">".repeat(1_000)));
    let size = out(&clean(&text)).len();
    assert_that!(bounded(&text, size - 1).map(|_| ()).map_err(|e| e.kind()))
        .is_equal_to(Err(ImportErrorKind::TooLarge));
}

#[test]
fn sanitize_svg_over_the_node_limit_is_too_large() {
    let text = svg(&"<rect/>".repeat(100));
    let e = sanitize(text.as_bytes(), "ppt/media/image1.svg", 50, BYTES).err().unwrap();
    assert_that!(e.kind()).is_equal_to(ImportErrorKind::TooLarge);
}

#[test]
fn sanitize_doctype_without_declarations_is_dropped_silently() {
    let text = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd"><!-- c --><svg {NS}><rect/></svg>"#
    );
    let c = clean(&text);
    assert_that!(c.altered).is_false();
    assert_that!(out(&c)).is_equal_to(format!("<svg {NS}><rect/></svg>"));
}

#[test]
fn sanitize_raster_data_uri_and_fragment_are_kept() {
    let c = clean(&svg(
        r##"<image href="data:image/png;base64,iVBORw0KGgo="/><image xlink:href=" DATA:image/webp,AAAA"/><a href="#x"><rect/></a>"##,
    ));
    assert_that!(c.altered).is_false();
    assert_that!(out(&c).contains("data:image/png;base64,iVBORw0KGgo=")).is_true();
    assert_that!(out(&c).contains("DATA:image/webp,AAAA")).is_true();
    assert_that!(out(&c).contains(r##"href="#x""##)).is_true();
}

#[test]
fn sanitize_foreign_metadata_is_dropped_silently() {
    let c = clean(&format!(
        r#"<svg {NS} xmlns:inkscape="urn:inkscape" inkscape:version="1"><metadata><x xmlns="urn:rdf">m</x></metadata><inkscape:grid/><g inkscape:label="l"><rect/></g></svg>"#
    ));
    assert_that!(c.altered).is_false();
    assert_that!(out(&c)).is_equal_to(format!("<svg {NS}><g><rect/></g></svg>"));
}

#[test]
fn sanitize_text_and_attributes_are_escaped_and_round_trip() {
    let c = clean(&svg(
        r#"<text xml:space="preserve" font-family="A &quot;B&quot; &amp; &lt;C&gt;">x &lt; y &amp;&amp; "z" <![CDATA[<b>]]></text>"#,
    ));
    let text = out(&c);
    let doc = roxmltree::Document::parse(&text).unwrap();
    let t = doc.descendants().find(|n| n.has_tag_name(("http://www.w3.org/2000/svg", "text"))).unwrap();
    assert_that!(t.attribute("font-family")).is_equal_to(Some(r#"A "B" & <C>"#));
    assert_that!(t.attribute(("http://www.w3.org/XML/1998/namespace", "space"))).is_equal_to(Some("preserve"));
    assert_that!(t.text()).is_equal_to(Some(r#"x < y && "z" <b>"#));
}

#[test]
fn sanitize_output_is_stable_when_sanitized_again() {
    let first = clean(&svg(r#"<script>x</script><g onclick="y"><text>a &amp; b</text></g>"#));
    let second = clean(&out(&first));
    assert_that!(second.altered).is_false();
    assert_that!(second.bytes).is_equal_to(first.bytes);
}

#[test]
fn sanitize_illustrator_literal_entities_are_expanded_and_kept() {
    let text = r#"<?xml version="1.0"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd" [
	<!ENTITY ns_svg "http://www.w3.org/2000/svg">
	<!-- styles -->
	<!ENTITY st0 'fill:#ff0000;'>
]>
<svg xmlns="&ns_svg;"><rect style="&st0;" width="1"/></svg>"#;
    let c = clean(text);
    assert_that!(c.altered).is_false();
    assert_that!(out(&c)).is_equal_to(svg_root(r#"<rect style="fill:#ff0000;" width="1"/>"#));
}

/// Sortie attendue d'un `<svg>` sans attribut autour de `body`.
fn svg_root(body: &str) -> String {
    format!("<svg {NS}>{body}</svg>")
}
