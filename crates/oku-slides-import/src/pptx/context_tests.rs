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
</Relationships>"#;

fn pkg() -> TestPkg {
    TestPkg::with(&[
        ("ppt/slides/_rels/slide1.xml.rels", SRELS.as_bytes()),
        ("ppt/media/image1.png", PNG),
        ("ppt/media/pic.emf", b"emf"),
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
fn emf_asset_is_kept_and_reported_unsupported() {
    let t = pkg();
    let (a, w, _) = with_ctx(&t.pkg(), "ppt/slides/slide1.xml", |cx| cx.asset("rId4"));
    assert_that!(a.is_some()).is_true();
    assert_that!(w.iter().any(|w| matches!(w.kind, WarningKind::UnsupportedImage(_)))).is_true();
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
