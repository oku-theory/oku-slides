//! Word documents and presentations: their structure as Markdown, their images within limits,
//! and the hostile ones refused or cut.

#![expect(clippy::panic, reason = "test helpers fail loudly on a broken fixture")]

mod common;

use common::extract;
use common::{docx, docx_image, docx_styled, emf, limits, para, png, read, zip_dir_with};
use oku_slides_core::knowledge::Extracted;
use oku_slides_core::knowledge::{Cut, CutKind, DocumentType, ExtractLimits, ExtractRefusal};
use oku_slides_import::ReadLimits;
use speculoos::prelude::*;

const TOKEN: &str = "0123456789abcdef";

fn extracted(bytes: &[u8], kind: DocumentType, limits: &ExtractLimits) -> Extracted {
    extract(bytes, kind, limits, &read(), TOKEN).unwrap_or_else(|e| panic!("extract: {e:?}"))
}

fn image_cut(out: &Extracted) -> Option<u64> {
    out.cuts.iter().find(|c| c.what == CutKind::Images).map(|c| c.count)
}

/// The `docx-full` fixture with its picture generated.
fn docx_full() -> Vec<u8> {
    zip_dir_with("docx-full", &[("word/media/image1.png", &png(2, 2))])
}

/// The `pptx-notes-image` fixture with its picture generated.
fn pptx_notes_image() -> Vec<u8> {
    zip_dir_with("pptx-notes-image", &[("ppt/media/image1.png", &png(2, 2))])
}

#[test]
fn docx_full_fixture_markdown() {
    let out = extracted(&docx_full(), DocumentType::Docx, &limits());
    let md = &out.markdown;
    insta::assert_snapshot!(md);
    for mark in ["# ", "1. ", "- ", "| --- |", "**"] {
        assert_that!(md.as_str()).named(mark).contains(mark);
    }
    assert_that!(md.as_str()).contains("](https://example.com/share%5Cdocs)");
    assert_that!(md.as_str()).contains("javascript:alert(1)");
    assert_that!(md.as_str()).does_not_contain("](javascript:");
    for hidden in ["Hidden draft note", "Deleted sentence", "Secret comment text"] {
        assert_that!(md.as_str()).named(hidden).does_not_contain(hidden);
    }
    assert_that!(md.as_str()).contains(format!("![A red square](oku-image-{TOKEN}-0)").as_str());
    assert_that!(out.images.len()).is_equal_to(1);
    assert_that!(out.images[0].mime.as_str()).is_equal_to("image/png");
    assert_that!(out.cuts.is_empty()).is_true();
}

#[test]
fn docx_depth_past_cap_cut_stated() {
    // document, body, 123 pairs of sdt and sdtContent, then p, r and t: 251 levels.
    let deep = format!(
        "{}{}{}",
        "<w:sdt><w:sdtContent>".repeat(123),
        para("Buried text"),
        "</w:sdtContent></w:sdt>".repeat(123)
    );
    let bytes = docx(&format!("{}{deep}", para("Shallow text")), &[]);
    let run = std::thread::Builder::new()
        .stack_size(256 << 10)
        .spawn(move || extract(&bytes, DocumentType::Docx, &limits(), &read(), TOKEN))
        .unwrap_or_else(|e| panic!("spawn: {e}"));
    let out =
        run.join().unwrap_or_else(|_| panic!("the walk overflowed its stack")).unwrap_or_else(|e| panic!("{e:?}"));
    assert_that!(out.markdown.as_str()).contains("Shallow text");
    assert_that!(out.markdown.as_str()).does_not_contain("Buried text");
    assert_that!(out.markdown.as_str()).contains("nested elements not imported (limit 64)");
    assert_that!(out.cuts.iter().any(|c| c.what == CutKind::Depth && c.count > 0)).is_true();
}

#[test]
fn docx_zip_bomb_too_large() {
    let bytes = docx(&format!("{}{}", para("x"), " ".repeat(4 << 20)), &[]);
    assert_that!(bytes.len()).is_less_than(64 << 10);
    let tight = ReadLimits { max_part_bytes: 1 << 20, ..read() };
    let err = extract(&bytes, DocumentType::Docx, &limits(), &tight, TOKEN).err();
    assert_that!(err).is_equal_to(Some(ExtractRefusal::TooLarge));
}

/// t-1760: a slide inflating past the part limit is refused, never read whole.
#[test]
fn pptx_zip_bomb_too_large() {
    let slide = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">{}</p:sld>"#,
        " ".repeat(4 << 20)
    );
    let bytes = zip_dir_with(
        "pptx-notes-image",
        &[("ppt/media/image1.png", &png(2, 2)), ("ppt/slides/slide1.xml", slide.as_bytes())],
    );
    assert_that!(bytes.len()).is_less_than(64 << 10);
    let tight = ReadLimits { max_part_bytes: 1 << 20, ..read() };
    let err = extract(&bytes, DocumentType::Pptx, &limits(), &tight, TOKEN).err();
    assert_that!(err).is_equal_to(Some(ExtractRefusal::TooLarge));
}

#[test]
fn docx_document_with_oku_image_literal_not_substituted() {
    let body = format!("{}<w:p>{}</w:p>", para("See oku-image-ffffffffffffffff-0 here"), docx_image("rImg0", "chart"));
    let out = extracted(&docx(&body, &[("a.png", &png(2, 2))]), DocumentType::Docx, &limits());
    assert_that!(out.markdown.as_str()).contains("See oku-image-ffffffffffffffff-0 here");
    assert_that!(out.markdown.as_str()).does_not_contain("](oku-image-ffffffffffffffff");
    assert_that!(out.markdown.as_str()).contains(format!("![chart](oku-image-{TOKEN}-0)").as_str());
    assert_that!(out.images.len()).is_equal_to(1);
}

#[test]
fn pptx_slides_notes_table_image() {
    let out = extracted(&pptx_notes_image(), DocumentType::Pptx, &limits());
    let md = &out.markdown;
    insta::assert_snapshot!(md);
    assert_that!(md.as_str()).contains("## Slide 1 — Quarterly results");
    assert_that!(md.as_str()).contains("### Notes");
    assert_that!(md.as_str()).contains("Mention the South recovery.");
    assert_that!(md.as_str()).contains("| --- |");
    assert_that!(md.as_str()).contains(format!("![A red square](oku-image-{TOKEN}-0)").as_str());
    assert_that!(out.images.len()).is_equal_to(1);
}

#[test]
fn pptx_tiny_limits_too_large() {
    let tight = ReadLimits { max_xml_nodes: 20, ..read() };
    let err = extract(&pptx_notes_image(), DocumentType::Pptx, &limits(), &tight, TOKEN).err();
    assert_that!(err).is_equal_to(Some(ExtractRefusal::TooLarge));
}

#[test]
fn pptx_bytes_as_docx_type_mismatch() {
    let pptx = pptx_notes_image();
    let docx = docx_full();
    assert_that!(extract(&pptx, DocumentType::Docx, &limits(), &read(), TOKEN).err())
        .is_equal_to(Some(ExtractRefusal::TypeMismatch));
    assert_that!(extract(&docx, DocumentType::Pptx, &limits(), &read(), TOKEN).err())
        .is_equal_to(Some(ExtractRefusal::TypeMismatch));
}

#[test]
fn image_past_pixel_limit_listed_not_imported() {
    let body = format!("{}<w:p>{}</w:p>", para("Before"), docx_image("rImg0", "huge"));
    let out = extracted(&docx(&body, &[("huge.png", &png(20_000, 20_000))]), DocumentType::Docx, &limits());
    assert_that!(out.images.is_empty()).is_true();
    assert_that!(image_cut(&out)).is_equal_to(Some(1));
    assert_that!(out.markdown.as_str()).contains("… 1 images not imported (over 67108864 pixels)");
    assert_that!(out.markdown.as_str()).does_not_contain("oku-image-");
}

#[test]
fn images_past_max_images_listed() {
    let body = ["rImg0", "rImg1", "rImg2"].map(|rel| format!("<w:p>{}</w:p>", docx_image(rel, "p"))).concat();
    let media = [("a.png", png(2, 2)), ("b.png", png(3, 3)), ("c.png", png(4, 4))];
    let media: Vec<(&str, &[u8])> = media.iter().map(|(n, b)| (*n, b.as_slice())).collect();
    let two = ExtractLimits { max_images: 2, ..limits() };
    let out = extracted(&docx(&body, &media), DocumentType::Docx, &two);
    assert_that!(out.images.len()).is_equal_to(2);
    assert_that!(out.cuts.contains(&Cut { what: CutKind::Images, count: 1 })).is_true();
    assert_that!(out.markdown.as_str()).contains(format!("oku-image-{TOKEN}-1").as_str());
    assert_that!(out.markdown.as_str()).does_not_contain(format!("oku-image-{TOKEN}-2").as_str());
    assert_that!(out.markdown.as_str()).contains("… 1 images not imported (limit 2)");
}

#[test]
fn unsupported_emf_listed_not_imported() {
    let body = format!("<w:p>{}</w:p><w:p>{}</w:p>", docx_image("rImg0", "vector"), docx_image("rImg1", "raster"));
    let out = extracted(&docx(&body, &[("v.emf", &emf()), ("r.png", &png(2, 2))]), DocumentType::Docx, &limits());
    assert_that!(out.images.len()).is_equal_to(1);
    assert_that!(image_cut(&out)).is_equal_to(Some(1));
    assert_that!(out.markdown.as_str()).contains("… 1 images not imported (image/emf)");
    assert_that!(out.markdown.as_str()).contains(format!("![raster](oku-image-{TOKEN}-0)").as_str());
}

#[test]
fn docx_image_whose_placeholder_is_cut_not_kept() {
    let body = format!("{}<w:p>{}</w:p>{}", para(&"a".repeat(130)), docx_image("rImg0", "pic"), para(&"b".repeat(150)));
    let bytes = docx(&body, &[("p.png", &png(2, 2))]);
    let out = extracted(&bytes, DocumentType::Docx, &ExtractLimits { max_text_bytes: 200, ..limits() });
    assert_that!(out.images.is_empty()).is_true();
    assert_that!(out.markdown.as_str()).does_not_contain("oku-image-");
    assert_that!(out.markdown.as_str()).contains("(text budget spent)");
    assert_that!(image_cut(&out)).is_equal_to(Some(1));
}

/// t-1760: headers, footers, footnotes and endnotes are not read, and the document says so.
#[test]
fn docx_headers_and_notes_stated_as_not_imported() {
    let body = concat!(
        r#"<w:p><w:r><w:t>body</w:t></w:r><w:r><w:footnoteReference w:id="1"/></w:r><w:r><w:endnoteReference w:id="1"/></w:r></w:p>"#,
        r#"<w:sectPr><w:headerReference w:type="default" r:id="rH"/></w:sectPr>"#,
    );
    let out = extracted(&docx(body, &[]), DocumentType::Docx, &limits());
    assert_that!(out.markdown)
        .is_equal_to("body\n\n… not imported: headers and footers, footnotes, endnotes\n".to_owned());
}

/// t-1760: of alternate content, in a run or inside a drawing, only the chosen branch is read.
#[test]
fn docx_alternate_content_reads_only_the_choice() {
    let mc = r#"xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006""#;
    let blip = |rel: &str| format!(r#"<pic:pic><pic:blipFill><a:blip r:embed="{rel}"/></pic:blipFill></pic:pic>"#);
    let in_run = format!(
        r#"<w:r><mc:AlternateContent {mc}><mc:Choice Requires="wps"><w:drawing><wp:inline><wp:docPr id="1" name="c" descr="run"/>{}</wp:inline></w:drawing></mc:Choice><mc:Fallback><w:drawing>{}</w:drawing></mc:Fallback></mc:AlternateContent></w:r>"#,
        blip("rImg0"),
        blip("rImg1"),
    );
    let in_drawing = format!(
        r#"<w:r><w:drawing><wp:inline><wp:docPr id="2" name="d" descr="drawing"/><mc:AlternateContent {mc}><mc:Choice Requires="wps">{}</mc:Choice><mc:Fallback>{}</mc:Fallback></mc:AlternateContent></wp:inline></w:drawing></w:r>"#,
        blip("rImg2"),
        blip("rImg1"),
    );
    let body = format!("<w:p>{in_run}</w:p><w:p>{in_drawing}</w:p>");
    let bytes = docx(&body, &[("a.png", &png(2, 2)), ("b.png", &png(3, 3)), ("c.png", &png(4, 4))]);
    let out = extracted(&bytes, DocumentType::Docx, &limits());
    assert_that!(out.images.iter().map(|i| i.bytes.clone()).collect::<Vec<_>>())
        .is_equal_to(vec![png(2, 2), png(4, 4)]);
}

#[test]
fn docx_image_past_read_limits_stated_as_such() {
    let body = format!("<w:p>{}</w:p>", docx_image("rImg0", "big"));
    let mut big = png(2, 2);
    big.resize(4096, 0);
    let bytes = docx(&body, &[("big.png", &big)]);
    let read = ReadLimits { max_part_bytes: 2048, ..read() };
    let out = extract(&bytes, DocumentType::Docx, &limits(), &read, TOKEN).unwrap_or_else(|e| panic!("{e:?}"));
    assert_that!(out.markdown.as_str()).contains("… 1 images not imported (past the read limits)");
}

#[test]
fn docx_text_hidden_by_style_left_out_and_stated() {
    let styles = r#"<w:style w:type="character" w:styleId="Secret"><w:rPr><w:vanish/></w:rPr></w:style><w:style w:type="character" w:styleId="Child"><w:basedOn w:val="Secret"/></w:style><w:style w:type="paragraph" w:styleId="HiddenPara"><w:rPr><w:specVanish/></w:rPr></w:style>"#;
    let body = concat!(
        r#"<w:p><w:r><w:t>shown</w:t></w:r><w:r><w:rPr><w:rStyle w:val="Child"/></w:rPr><w:t>secret</w:t></w:r></w:p>"#,
        r#"<w:p><w:r><w:rPr><w:rStyle w:val="Secret"/><w:vanish w:val="0"/></w:rPr><w:t>unhidden</w:t></w:r></w:p>"#,
        r#"<w:p><w:pPr><w:pStyle w:val="HiddenPara"/></w:pPr><w:r><w:t>whole paragraph</w:t></w:r></w:p>"#,
        r#"<w:p><w:r><w:rPr><w:specVanish/></w:rPr><w:t>special</w:t></w:r></w:p>"#,
    );
    let out = extracted(&docx_styled(body, &[], Some(styles)), DocumentType::Docx, &limits());
    assert_that!(out.markdown).is_equal_to("shown\n\nunhidden\n\n… not imported: hidden text\n".to_owned());
}

#[test]
fn docx_text_box_read_as_paragraphs_after_its_anchor() {
    let body = concat!(
        r#"<w:p><w:r><w:t>Before</w:t></w:r><w:r><w:drawing><wp:anchor><wp:docPr id="1" name="box"/><a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><wps:wsp xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><wps:txbx><w:txbxContent><w:p><w:r><w:t>Boxed one</w:t></w:r></w:p><w:p><w:r><w:t>Boxed two</w:t></w:r></w:p></w:txbxContent></wps:txbx></wps:wsp></a:graphicData></a:graphic></wp:anchor></w:drawing></w:r></w:p>"#,
        r#"<w:p><w:r><w:t>After</w:t></w:r></w:p>"#,
    );
    let out = extracted(&docx(body, &[]), DocumentType::Docx, &limits());
    assert_that!(out.markdown).is_equal_to("Before\n\nBoxed one\n\nBoxed two\n\nAfter\n".to_owned());
}

#[test]
fn pptx_group_past_nesting_limit_cut_stated() {
    let open = r#"<p:grpSp><p:nvGrpSpPr><p:cNvPr id="9" name="g"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>"#;
    let text = r#"<p:sp><p:nvSpPr><p:cNvPr id="8" name="t"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:p><a:r><a:t>Deep inside</a:t></a:r></a:p></p:txBody></p:sp>"#;
    let slide = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{}{text}{}</p:spTree></p:cSld></p:sld>"#,
        open.repeat(40),
        "</p:grpSp>".repeat(40),
    );
    let bytes = zip_dir_with(
        "pptx-notes-image",
        &[("ppt/media/image1.png", &png(2, 2)), ("ppt/slides/slide1.xml", slide.as_bytes())],
    );
    let out = extracted(&bytes, DocumentType::Pptx, &limits());
    assert_that!(out.markdown.as_str()).does_not_contain("Deep inside");
    assert_that!(out.markdown.as_str()).contains("nested elements not imported (too deep)");
    assert_that!(out.markdown.as_str()).does_not_contain("media and embedded objects");
    assert_that!(out.cuts.iter().any(|c| c.what == CutKind::Depth && c.count >= 1)).is_true();
}

/// A one-slide deck whose 1001 shadows cut the import warnings, then holds `before` and a
/// picture citing `rId3` (`image1.png`); `extra` adds or replaces parts.
fn pptx_past_warning_limit(image: &[u8], before: &str, extra: &[(&str, &[u8])]) -> Vec<u8> {
    let shadow = r#"<p:sp><p:nvSpPr><p:cNvPr id="7" name="s"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:effectLst><a:outerShdw/></a:effectLst></p:spPr></p:sp>"#;
    let picture = r#"<p:pic><p:nvPicPr><p:cNvPr id="5" name="Picture 4" descr="huge"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId3"/></p:blipFill><p:spPr/></p:pic>"#;
    let slide = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{}{before}{picture}</p:spTree></p:cSld></p:sld>"#,
        shadow.repeat(1001),
    );
    let mut parts: Vec<(&str, &[u8])> =
        vec![("ppt/media/image1.png", image), ("ppt/slides/slide1.xml", slide.as_bytes())];
    parts.extend_from_slice(extra);
    zip_dir_with("pptx-notes-image", &parts)
}

#[test]
fn pptx_image_refused_past_warning_limit_still_listed() {
    let out = extracted(&pptx_past_warning_limit(&png(100_000, 100_000), "", &[]), DocumentType::Pptx, &limits());
    assert_that!(out.markdown.as_str()).contains("… 1 images not imported (over 67108864 pixels)");
    assert_that!(out.markdown.as_str()).contains("what was met past the import warning limit");
}

#[test]
fn pptx_image_imported_beyond_warning_limit_not_listed() {
    let out = extracted(&pptx_past_warning_limit(&png(2, 2), "", &[]), DocumentType::Pptx, &limits());
    assert_that!(out.images).has_length(1);
    assert_that!(out.markdown.as_str()).does_not_contain("images not imported");
    assert_that!(image_cut(&out)).is_none();
}

#[test]
fn pptx_unparsed_citation_past_read_limit_stops_scan_imported_kept() {
    let alternate = r#"<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main"><mc:Choice Requires="p14"><p:sp><p:nvSpPr><p:cNvPr id="6" name="c"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp></mc:Choice><mc:Fallback><p:pic><p:nvPicPr><p:cNvPr id="6" name="f"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId4"/></p:blipFill><p:spPr/></p:pic></mc:Fallback></mc:AlternateContent>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout2.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide" Target="../notesSlides/notesSlide1.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/><Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/big.png"/></Relationships>"#;
    let mut big = png(2, 2);
    big.resize(600_000, 0);
    let bytes = pptx_past_warning_limit(
        &png(2, 2),
        alternate,
        &[("ppt/media/big.png", &big), ("ppt/slides/_rels/slide1.xml.rels", rels.as_bytes())],
    );
    let read = ReadLimits { max_part_bytes: 512 * 1024, ..read() };
    let out = extract(&bytes, DocumentType::Pptx, &limits(), &read, TOKEN).unwrap_or_else(|e| panic!("{e:?}"));
    assert_that!(out.images).has_length(1);
    assert_that!(out.markdown.as_str()).does_not_contain("images not imported");
    assert_that!(out.markdown.as_str()).contains("pictures whose part could not be read again, not counted");
}

#[test]
fn pptx_unreadable_slide_relationships_past_warning_limit_stop_scan_stated() {
    // `read_pptx` keeps the slide without its layout; the scan cannot list its pictures.
    let bytes = pptx_past_warning_limit(&png(2, 2), "", &[("ppt/slides/_rels/slide1.xml.rels", b"not xml")]);
    let out = extracted(&bytes, DocumentType::Pptx, &limits());
    assert_that!(out.markdown.as_str()).contains("pictures whose part could not be read again, not counted");
}

#[test]
fn docx_alt_from_own_docpr_not_text_box_one() {
    let body = r#"<w:p><w:r><w:drawing><wp:anchor><a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><wps:wsp xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><wps:txbx><w:txbxContent><w:p><w:r><w:drawing><wp:inline><wp:docPr id="2" name="i" descr="inner"/></wp:inline></w:drawing></w:r></w:p></w:txbxContent></wps:txbx></wps:wsp></a:graphicData></a:graphic><wp:docPr id="1" name="o" descr="outer"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:blipFill><a:blip r:embed="rImg0"/></pic:blipFill></pic:pic></a:graphicData></a:graphic></wp:anchor></w:drawing></w:r></w:p>"#;
    let out = extracted(&docx(body, &[("a.png", &png(2, 2))]), DocumentType::Docx, &limits());
    assert_that!(out.markdown.as_str()).contains("![outer]");
    assert_that!(out.markdown.as_str()).does_not_contain("inner");
}

#[test]
fn pptx_unsupported_image_with_clipped_path_counted_once() {
    let name = format!("{}.emf", "a".repeat(220));
    let rels = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout2.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide" Target="../notesSlides/notesSlide1.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/{name}"/></Relationships>"#
    );
    let part = format!("ppt/media/{name}");
    let bytes = zip_dir_with(
        "pptx-notes-image",
        &[(part.as_str(), &emf()), ("ppt/slides/_rels/slide1.xml.rels", rels.as_bytes())],
    );
    let out = extracted(&bytes, DocumentType::Pptx, &limits());
    assert_that!(out.markdown.as_str()).contains("… 1 images not imported (image/emf)");
    assert_that!(out.markdown.as_str()).contains("pictures whose part could not be read again");
    assert_that!(image_cut(&out)).is_equal_to(Some(1));
}

/// A ragged table whose rows each pad to the widest: past the text budget it is left out
/// before its Markdown is built, never written out cell by cell (t-1760).
fn ragged_cells(wide: usize, tall: usize, cell: &str, wide_row: &str, row: &str) -> String {
    let first = cell.replace("{text}", "x") + &cell.replace("{text}", "").repeat(wide - 1);
    let rest = row.replace("{cells}", &cell.replace("{text}", "")).repeat(tall);
    wide_row.replace("{cells}", &first) + &rest
}

#[test]
fn docx_ragged_table_bomb_is_cut() {
    let cell = r"<w:tc><w:p><w:r><w:t>{text}</w:t></w:r></w:p></w:tc>";
    let rows = ragged_cells(50_000, 200_000, cell, "<w:tr>{cells}</w:tr>", "<w:tr>{cells}</w:tr>");
    let body = format!("{}<w:tbl>{rows}</w:tbl>{}", para("Intro"), para("After"));
    let out = extracted(&docx(&body, &[]), DocumentType::Docx, &limits());
    let md = out.markdown.as_str();
    assert_that!(md).contains("Intro");
    assert_that!(md).does_not_contain("| --- |");
    assert_that!(md).does_not_contain("After");
    assert_that!(md.len()).is_less_than_or_equal_to(limits().max_text_bytes);
    assert_that!(out.cuts.iter().any(|c| c.what == CutKind::Text && c.count > 0)).is_true();
}

#[test]
fn pptx_ragged_table_bomb_is_cut() {
    let cell = r"<a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>{text}</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>";
    let rows = ragged_cells(20_000, 20_000, cell, r#"<a:tr h="1">{cells}</a:tr>"#, r#"<a:tr h="1">{cells}</a:tr>"#);
    let table = format!(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="4" name="T"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblGrid><a:gridCol w="10"/></a:tblGrid>{rows}</a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#
    );
    let slide = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{table}</p:spTree></p:cSld></p:sld>"#
    );
    let bytes = zip_dir_with(
        "pptx-notes-image",
        &[("ppt/media/image1.png", &png(2, 2)), ("ppt/slides/slide1.xml", slide.as_bytes())],
    );
    let out = extracted(&bytes, DocumentType::Pptx, &limits());
    let md = out.markdown.as_str();
    assert_that!(md).contains("## Slide 1");
    assert_that!(md).does_not_contain("| --- |");
    assert_that!(md.len()).is_less_than_or_equal_to(limits().max_text_bytes);
    assert_that!(out.cuts.iter().any(|c| c.what == CutKind::Text && c.count > 0)).is_true();
}
