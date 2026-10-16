use rstest::rstest;
use speculoos::prelude::*;

use crate::ImportErrorKind;
use crate::pptx::test_support::{PNG, TestPkg, run_ctx, with_ctx};
use crate::source::WarningKind;

const SRELS: &str = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/missing.png"/>
<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/pic.emf"/>
<Relationship Id="rId8" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="javascript:alert(1)" TargetMode="External"/>
<Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://oku.example/doc" TargetMode="External"/>
<Relationship Id="rId10" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://oku.example/a&#10;b" TargetMode="External"/>
<Relationship Id="rId11" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/photo.png"/>
<Relationship Id="rId12" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/icon.svg"/>
<Relationship Id="rId13" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/bomb.svg"/>
<Relationship Id="rId14" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/hidden.png"/>
<Relationship Id="rId15" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/huge.png"/>
<Relationship Id="rId16" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/broken.png"/>
</Relationships>"#;

const HOSTILE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" onload="alert(1)"><script>alert(2)</script><rect width="1" height="1"/></svg>"#;
const BOMB_SVG: &str = r#"<!DOCTYPE svg [<!ENTITY a "aaaa"><!ENTITY b "&a;&a;&a;&a;">]><svg xmlns="http://www.w3.org/2000/svg"><text>&b;</text></svg>"#;

/// JPEG 1 × 1 réduit à son segment `SOF0`.
const JPEG: &[u8] = b"\xFF\xD8\xFF\xC0\0\x11\x08\0\x01\0\x01\x03";
/// PNG dont l'en-tête annonce 100 000 × 100 000 pixels, sans données.
const HUGE_PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR\0\x01\x86\xA0\0\x01\x86\xA0\x08\x06\0\0\0";

fn pkg() -> TestPkg {
    TestPkg::with(&[
        ("ppt/slides/_rels/slide1.xml.rels", SRELS.as_bytes()),
        ("ppt/media/image1.png", PNG),
        ("ppt/media/pic.emf", b"emf"),
        ("ppt/media/photo.png", JPEG),
        ("ppt/media/icon.svg", HOSTILE_SVG.as_bytes()),
        ("ppt/media/bomb.svg", BOMB_SVG.as_bytes()),
        ("ppt/media/hidden.png", HOSTILE_SVG.as_bytes()),
        ("ppt/media/huge.png", HUGE_PNG),
        ("ppt/media/broken.png", b"\x89PNG\r\n\x1a\n-no-header"),
    ])
}

#[test]
fn asset_read_twice_is_one_asset_with_mime() {
    let t = pkg();
    let ((a, b), _, assets) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| (cx.asset("rId2"), cx.asset("rId2")));
    assert_that!(a.clone()).is_equal_to(b);
    assert_that!(assets.get(&a.unwrap()).map(|x| x.mime.as_str())).is_equal_to(Some("image/png"));
}

#[test]
fn missing_asset_warns_missing_part() {
    let t = pkg();
    let (a, w, _) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| cx.asset("rId3"));
    assert_that!(a).is_none();
    assert_that!(w.iter().any(|w| matches!(&w.kind, WarningKind::MissingPart(p) if p == "ppt/media/missing.png")))
        .is_true();
}

#[test]
fn asset_mime_comes_from_content_not_extension() {
    let t = pkg();
    let (a, w, assets) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| cx.asset("rId11"));
    assert_that!(assets.get(&a.unwrap()).map(|x| x.mime.as_str())).is_equal_to(Some("image/jpeg"));
    assert_that!(w).is_empty();
}

#[test]
fn asset_unknown_content_is_octet_stream_reported_unsupported() {
    let t = pkg();
    let (a, w, assets) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| cx.asset("rId4"));
    assert_that!(assets.get(&a.unwrap()).map(|x| x.mime.as_str())).is_equal_to(Some("application/octet-stream"));
    assert_that!(w.iter().any(|w| matches!(&w.kind, WarningKind::UnsupportedImage(p) if p == "ppt/media/pic.emf")))
        .is_true();
}

#[rstest]
#[case::svg_extension("rId12", "ppt/media/icon.svg")]
#[case::png_extension("rId14", "ppt/media/hidden.png")]
fn asset_hostile_svg_is_sanitized_and_identified_by_its_clean_bytes(#[case] rid: &str, #[case] path: &str) {
    let t = pkg();
    let ((a, again), w, assets) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| (cx.asset(rid), cx.asset(rid)));
    let id = a.unwrap();
    let asset = &assets[&id];
    let text = String::from_utf8(asset.bytes.clone()).unwrap();
    assert_that!(asset.mime.as_str()).is_equal_to("image/svg+xml");
    assert_that!(text.contains("alert")).is_false();
    assert_that!(text.contains("<rect")).is_true();
    assert_that!(id.clone()).is_equal_to(oku_slides_core::asset_id(&asset.bytes));
    assert_that!(again).is_equal_to(Some(id));
    let unsafe_svg: Vec<_> = w.iter().filter(|w| matches!(&w.kind, WarningKind::UnsafeSvg(p) if p == path)).collect();
    assert_that!(unsafe_svg).has_length(1);
}

#[test]
fn asset_svg_declaring_entities_is_rejected_once() {
    let t = pkg();
    let ((a, again), w, assets) =
        with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| (cx.asset("rId13"), cx.asset("rId13")));
    assert_that!((a, again)).is_equal_to((None, None));
    assert_that!(assets.len()).is_equal_to(0);
    let unsafe_svg: Vec<_> = w.iter().filter(|w| matches!(w.kind, WarningKind::UnsafeSvg(_))).collect();
    assert_that!(unsafe_svg).has_length(1);
}

#[test]
fn https_link_is_kept_and_unsafe_links_are_refused() {
    let t = pkg();
    let ((good, script, spaced), w, _) =
        with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| (cx.link("rId9"), cx.link("rId8"), cx.link("rId10")));
    assert_that!((good.as_deref(), script, spaced)).is_equal_to((Some("https://oku.example/doc"), None, None));
    assert_that!(w.iter().filter(|w| matches!(w.kind, WarningKind::UnsafeLink(_))).count()).is_equal_to(2);
}

#[test]
fn item_ids_are_integers_made_unique_and_unforgeable() {
    let t = pkg();
    let (ids, _, _) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| {
        [cx.item_id("4"), cx.item_id("4"), cx.item_id("4~2"), cx.item_id("../s2/1")]
    });
    assert_that!(ids).is_equal_to(["s1/4".to_owned(), "s1/4~2".to_owned(), "s1/x".to_owned(), "s1/x~2".to_owned()]);
}

#[test]
fn item_ids_many_duplicates_get_consecutive_suffixes() {
    // Fichier hostile : des milliers de `cNvPr id="4"` ; le suffixe suivant est mémorisé, pas recherché depuis ~2.
    let t = pkg();
    let (ids, _, _) =
        with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| (0..5000).map(|_| cx.item_id("4")).collect::<Vec<_>>());
    assert_that!((ids[2].as_str(), ids[4999].as_str())).is_equal_to(("s1/4~3", "s1/4~5000"));
}

#[test]
fn raw_item_ids_keep_only_safe_characters() {
    let t = pkg();
    let (id, _, _) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| cx.item_id_raw("{A1/../B2}"));
    assert_that!(id).is_equal_to("s1/{A1B2}".to_owned());
}

#[test]
fn asset_over_part_limit_becomes_a_fatal_too_large() {
    let big = vec![0u8; 2 << 20];
    let t = TestPkg::with(&[
        ("ppt/slides/_rels/slide1.xml.rels", SRELS.as_bytes()),
        ("ppt/media/image1.png", big.as_slice()),
    ]);
    let p = t.pkg();
    let err = run_ctx(&p, "ppt/slides/slide1.xml", None, |cx| cx.asset("rId2")).map(|_| ()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::TooLarge);
}

#[test]
fn missing_asset_referenced_twice_warns_once() {
    let t = pkg();
    let ((a, b), w, _) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| (cx.asset("rId3"), cx.asset("rId3")));
    assert_that!((a, b)).is_equal_to((None, None));
    assert_that!(w.iter().filter(|w| matches!(w.kind, WarningKind::MissingPart(_))).count()).is_equal_to(1);
}

#[test]
fn linked_doc_over_the_node_limit_is_fatal() {
    let t = pkg();
    let p = t.pkg();
    let xml = format!("<r>{}</r>", "<a/>".repeat(2_000_000));
    let err = run_ctx(&p, "ppt/slides/slide1.xml", None, |cx| {
        cx.linked_doc(&xml, "ppt/notesSlides/n.xml", None, WarningKind::UnreadablePart("n".into())).is_some()
    })
    .map(|_| ())
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::TooLarge);
}

#[test]
fn linked_doc_malformed_warns_with_the_given_kind() {
    let t = pkg();
    let (doc, w, _) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| {
        cx.linked_doc("<r>", "ppt/charts/c.xml", Some("s1/4"), WarningKind::UnreadableItem("c".into())).is_some()
    });
    assert_that!(doc).is_false();
    assert_that!(w.iter().map(|w| (w.item.as_deref(), w.kind.clone())).collect::<Vec<_>>())
        .is_equal_to(vec![(Some("s1/4"), WarningKind::UnreadableItem("c".into()))]);
}

#[test]
fn asset_raster_beyond_max_image_pixels_is_refused_unread() {
    let t = pkg();
    let ((a, again), w, assets) =
        with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| (cx.asset("rId15"), cx.asset("rId15")));
    assert_that!((a, again)).is_equal_to((None, None));
    assert_that!(assets.len()).is_equal_to(0);
    let oversized: Vec<_> =
        w.iter().filter(|w| matches!(&w.kind, WarningKind::OversizedImage(p) if p == "ppt/media/huge.png")).collect();
    assert_that!(oversized).has_length(1);
}

#[test]
fn asset_raster_with_unreadable_header_is_refused_as_unsupported() {
    let t = pkg();
    let (a, w, _) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| cx.asset("rId16"));
    assert_that!(a).is_none();
    assert_that!(w.iter().any(|w| matches!(&w.kind, WarningKind::UnsupportedImage(p) if p == "ppt/media/broken.png")))
        .is_true();
}

/// Bombe zip (t-692) : `refs` lectures de chacune des `parts` images d'un octet de trop
/// (`max_part_bytes + 1` octets nuls) décompressent une seule fois `max_part_bytes + 1` octets.
#[rstest]
#[case::one_part(1, 1000)]
#[case::two_parts(2, 500)]
fn oversized_parts_read_many_times_are_decompressed_once(#[case] parts: usize, #[case] refs: usize) {
    use crate::pptx::package::Package;
    use crate::pptx::test_support::LIMITS;
    let rel = |i: usize| {
        format!(
            r#"<Relationship Id="rId{i}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/bomb{i}.png"/>"#
        )
    };
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{}</Relationships>"#,
        (0..parts).map(rel).collect::<String>()
    );
    let cap = usize::try_from(LIMITS.max_part_bytes).unwrap();
    let bomb = vec![0u8; cap + 1];
    let names: Vec<String> = (0..parts).map(|i| format!("ppt/media/bomb{i}.png")).collect();
    let mut entries: Vec<(&str, &[u8])> = vec![("ppt/slides/_rels/slide1.xml.rels", rels.as_bytes())];
    entries.extend(names.iter().map(|n| (n.as_str(), bomb.as_slice())));
    let t = TestPkg::with(&entries);
    let pkg = Package::open(t.bytes(), &LIMITS).unwrap();
    let opened = pkg.decompressed();
    let r = run_ctx(&pkg, "ppt/slides/slide1.xml", None, |cx| {
        for _ in 0..refs {
            for i in 0..parts {
                assert_that!(cx.asset(&format!("rId{i}"))).is_none();
            }
        }
    });
    assert_that!(r.map(|_| ()).map_err(|e| e.kind()).err()).is_equal_to(Some(ImportErrorKind::TooLarge));
    // Les relations de la slide, puis une seule décompression bornée de la première image.
    let rels_bytes = u64::try_from(rels.len()).unwrap();
    assert_that!(pkg.decompressed() - opened).is_equal_to(rels_bytes + LIMITS.max_part_bytes + 1);
}
