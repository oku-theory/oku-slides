use rstest::rstest;
use speculoos::prelude::*;

use super::{OCTET_STREAM, SVG, SniffRefusal, dimensions, sniff, sniff_document};
use crate::knowledge::DocumentType;

/// En-tête EMF : `EMR_HEADER` (type 1) et la signature ` EMF` à l'octet 40.
fn emf() -> Vec<u8> {
    let mut b = vec![0u8; 48];
    b[0] = 1;
    b[40..44].copy_from_slice(b" EMF");
    b
}

#[rstest]
#[case(b"\x89PNG\r\n\x1a\nrest".to_vec(), "image/png")]
#[case(b"\xFF\xD8\xFF\xE0rest".to_vec(), "image/jpeg")]
#[case(b"GIF89a....".to_vec(), "image/gif")]
#[case(b"GIF87a....".to_vec(), "image/gif")]
#[case(b"RIFF\x10\0\0\0WEBPVP8 ".to_vec(), "image/webp")]
#[case(b"BM\x36\0\0\0".to_vec(), "image/bmp")]
#[case(b"II*\0rest".to_vec(), "image/tiff")]
#[case(b"MM\0*rest".to_vec(), "image/tiff")]
#[case(b"II\xBC\x01rest".to_vec(), "image/vnd.ms-photo")]
#[case(emf(), "image/emf")]
#[case(b"\xD7\xCD\xC6\x9Arest".to_vec(), "image/wmf")]
#[case(b"\x01\0\x09\0\0\x03rest".to_vec(), "image/wmf")]
fn sniff_magic_bytes_give_the_real_type(#[case] bytes: Vec<u8>, #[case] mime: &str) {
    assert_that!(sniff(&bytes)).is_equal_to(mime);
}

#[rstest]
#[case("<svg xmlns=\"http://www.w3.org/2000/svg\"/>")]
#[case("\u{feff}<svg>")]
#[case("<?xml version=\"1.0\"?>\n<!-- Generator: X -->\n<svg\n viewBox=\"0 0 1 1\">")]
#[case("<?xml version=\"1.0\"?><!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"svg11.dtd\"><svg>")]
#[case("<!DOCTYPE svg [<!ENTITY a \"b\">]><svg>")]
#[case("<x:svg xmlns:x=\"http://www.w3.org/2000/svg\"/>")]
#[case("<?xml version=\"1.0\"?>\n<svg_1.a-b:svg>")]
fn sniff_svg_root_after_prolog_is_svg(#[case] text: &str) {
    assert_that!(sniff(text.as_bytes())).is_equal_to(SVG);
}

#[rstest]
#[case(b"".as_slice())]
#[case(b"emf".as_slice())]
#[case(b"<html><svg></svg></html>".as_slice())]
#[case(b"<svgx/>".as_slice())]
#[case(b"<x:svgx/>".as_slice())]
#[case(b"<:svg/>".as_slice())]
#[case(b"<x:y:svg/>".as_slice())]
#[case(b"<!-- unterminated <svg>".as_slice())]
#[case(b"\x89PN".as_slice())]
#[case(b"RIFF\x10\0\0\0WAVE".as_slice())]
fn sniff_unknown_content_is_octet_stream(#[case] bytes: &[u8]) {
    assert_that!(sniff(bytes)).is_equal_to(OCTET_STREAM);
}

/// Un DOCTYPE après l'autre, sans `[` : chacun ne se lit que jusqu'à son `>`, la lecture reste linéaire.
#[test]
fn sniff_many_doctypes_without_root_is_octet_stream() {
    let text = "<!DOCTYPE a>".repeat(100_000);
    assert_that!(sniff(text.as_bytes())).is_equal_to(OCTET_STREAM);
}

/// En-tête WebP `RIFF` suivi du segment `chunk` et de `rest`.
fn webp(chunk: [u8; 4], rest: &[u8]) -> Vec<u8> {
    let mut b = b"RIFF\0\0\0\0WEBP".to_vec();
    b.extend_from_slice(&chunk);
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(rest);
    b
}

#[rstest]
#[case::png(b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR\0\x01\x86\xA0\0\0\0\x02".to_vec(), "image/png", (100_000, 2))]
#[case::gif(b"GIF89a\x10\x27\x03\0".to_vec(), "image/gif", (10_000, 3))]
#[case::webp_lossy(webp(*b"VP8 ", b"\0\0\0\x9d\x01\x2a\x40\x01\xF0\0"), "image/webp", (320, 240))]
#[case::webp_lossless(webp(*b"VP8L", b"\x2F\x3F\xC0\x3B\0"), "image/webp", (64, 240))]
#[case::webp_extended(webp(*b"VP8X", b"\0\0\0\0\x9F\x86\x01\x01\0\0"), "image/webp", (100_000, 2))]
#[case::jpeg_after_app0_and_fill(
    b"\xFF\xD8\xFF\xE0\0\x04JF\xFF\xFF\xC2\0\x11\x08\xFF\xFF\x27\x10\x03".to_vec(),
    "image/jpeg",
    (10_000, 65_535)
)]
fn dimensions_header_gives_width_and_height(#[case] bytes: Vec<u8>, #[case] mime: &str, #[case] size: (u32, u32)) {
    assert_that!(dimensions(&bytes, mime)).is_equal_to(Some(size));
}

#[rstest]
#[case::png_truncated(b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR\0\0".to_vec(), "image/png")]
#[case::png_without_ihdr(b"\x89PNG\r\n\x1a\n-fake-header-here".to_vec(), "image/png")]
#[case::gif_truncated(b"GIF89a\x10".to_vec(), "image/gif")]
#[case::webp_unknown_chunk(webp(*b"ALPH", b"\0\0\0\0\0\0\0\0"), "image/webp")]
#[case::jpeg_scan_before_frame(b"\xFF\xD8\xFF\xDA\0\x02".to_vec(), "image/jpeg")]
#[case::jpeg_segment_past_end(b"\xFF\xD8\xFF\xE0\xFF\xFF".to_vec(), "image/jpeg")]
#[case::jpeg_garbage(b"\xFF\xD8garbage".to_vec(), "image/jpeg")]
#[case::not_a_raster(b"<svg/>".to_vec(), "image/svg+xml")]
fn dimensions_unreadable_header_is_none(#[case] bytes: Vec<u8>, #[case] mime: &str) {
    assert_that!(dimensions(&bytes, mime)).is_none();
}

const OLE2: &[u8] = b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1rest";
const ZIP: &[u8] = b"PK\x03\x04rest";

#[rstest]
#[case::pdf(b"%PDF-1.7\n".as_slice(), "report.pdf", DocumentType::Pdf)]
#[case::pdf_uppercase(b"%PDF-1.4\n".as_slice(), "REPORT.PDF", DocumentType::Pdf)]
#[case::docx(ZIP, "notes.docx", DocumentType::Docx)]
#[case::pptx(ZIP, "deck.pptx", DocumentType::Pptx)]
#[case::xlsx(ZIP, "sales.xlsx", DocumentType::Xlsx)]
#[case::ods(ZIP, "sales.ods", DocumentType::Ods)]
#[case::markdown(b"# Title\n".as_slice(), "readme.md", DocumentType::Markdown)]
#[case::markdown_long(b"# Title\n".as_slice(), "readme.markdown", DocumentType::Markdown)]
#[case::text("caf\u{e9}\n".as_bytes(), "notes.txt", DocumentType::Text)]
#[case::text_without_extension(b"plain".as_slice(), "notes", DocumentType::Text)]
#[case::path_keeps_last_segment(ZIP, "C:\\docs.pdf\\deck.pptx", DocumentType::Pptx)]
fn sniff_document_known_magic_gives_its_kind(#[case] bytes: &[u8], #[case] name: &str, #[case] kind: DocumentType) {
    assert_that!(sniff_document(bytes, name)).is_equal_to(Ok(kind));
}

#[rstest]
#[case::xls(OLE2, "old.xls", SniffRefusal::Legacy)]
#[case::ppt(OLE2, "old.ppt", SniffRefusal::Legacy)]
#[case::encrypted_docx(OLE2, "locked.docx", SniffRefusal::Legacy)]
#[case::pdf_named_zip(ZIP, "report.pdf", SniffRefusal::TypeMismatch)]
#[case::pdf_named_docx(b"%PDF-1.7\n".as_slice(), "notes.docx", SniffRefusal::TypeMismatch)]
#[case::pdf_named_md(b"%PDF-1.7\n".as_slice(), "notes.md", SniffRefusal::TypeMismatch)]
#[case::pdf_named_txt(b"%PDF-1.7\n".as_slice(), "notes.txt", SniffRefusal::TypeMismatch)]
#[case::text_named_docx(b"plain".as_slice(), "notes.docx", SniffRefusal::TypeMismatch)]
#[case::binary_named_pdf(b"\xFF\xFEgarbage".as_slice(), "report.pdf", SniffRefusal::TypeMismatch)]
#[case::text_named_png(b"bonjour".as_slice(), "x.png", SniffRefusal::TypeMismatch)]
#[case::text_named_svg(b"plain".as_slice(), "logo.SVG", SniffRefusal::TypeMismatch)]
#[case::svg_named_svg(b"<svg/>".as_slice(), "logo.svg", SniffRefusal::Unsupported)]
#[case::bmp_named_bmp(b"BM\x36\0\0\0".as_slice(), "a.bmp", SniffRefusal::Unsupported)]
#[case::png_named_txt(b"\x89PNG\r\n\x1a\n\0".as_slice(), "a.txt", SniffRefusal::Unsupported)]
#[case::text_named_heic(b"plain".as_slice(), "photo.heic", SniffRefusal::TypeMismatch)]
#[case::pdf_named_jpg(b"%PDF-1.7\n".as_slice(), "scan.jpg", SniffRefusal::TypeMismatch)]
#[case::zip_named_webp(ZIP, "x.webp", SniffRefusal::TypeMismatch)]
#[case::binary_named_gif(b"\xFF\xFEgarbage".as_slice(), "x.gif", SniffRefusal::TypeMismatch)]
#[case::zip(ZIP, "archive.zip", SniffRefusal::Unsupported)]
#[case::zip_named_text(ZIP, "notes.txt", SniffRefusal::TypeMismatch)]
#[case::text_with_nul(b"abc\0def".as_slice(), "notes.txt", SniffRefusal::Unsupported)]
#[case::not_utf8(b"\xFF\xFEgarbage".as_slice(), "notes.txt", SniffRefusal::Unsupported)]
#[case::empty(b"".as_slice(), "notes.txt", SniffRefusal::Unsupported)]
#[case::empty_pdf(b"".as_slice(), "report.pdf", SniffRefusal::Unsupported)]
fn sniff_document_refused_bytes_say_why(#[case] bytes: &[u8], #[case] name: &str, #[case] refusal: SniffRefusal) {
    assert_that!(sniff_document(bytes, name)).is_equal_to(Err(refusal));
}

/// t-1760: a PDF may open with a preamble (a mail header, a BOM line) before `%PDF-`, as readers
/// accept; the marker counts at a line start in the first KiB, under a `.pdf` name or a binary
/// file without a known extension.
#[rstest]
#[case::preamble_named_pdf(b"junk line\n%PDF-1.7\n\xE2\xE3".as_slice(), "report.pdf")]
#[case::cr_preamble_named_pdf(b"junk\r%PDF-1.4\n".as_slice(), "report.pdf")]
#[case::binary_preamble_no_extension(b"\xFF\xFE\n%PDF-1.7\n\0".as_slice(), "report")]
fn sniff_pdf_with_preamble(#[case] bytes: &[u8], #[case] name: &str) {
    assert_that!(sniff_document(bytes, name)).is_equal_to(Ok(DocumentType::Pdf));
}

/// t-1760: the preamble rule never makes a text a PDF: a marker inside a line, past the first
/// KiB, or in a Markdown or text file stays text.
#[rstest]
#[case::mid_line_named_pdf(b"x %PDF-1.7\n\xFF".as_slice(), "report.pdf", Err(SniffRefusal::TypeMismatch))]
#[case::markdown_mentions_it(b"# PDF\n%PDF-1.7 is a header\n".as_slice(), "notes.md", Ok(DocumentType::Markdown))]
#[case::text_mentions_it(b"line\n%PDF-1.7\n".as_slice(), "notes", Ok(DocumentType::Text))]
fn sniff_pdf_marker_in_text_stays_text(
    #[case] bytes: &[u8],
    #[case] name: &str,
    #[case] expected: Result<DocumentType, SniffRefusal>,
) {
    assert_that!(sniff_document(bytes, name)).is_equal_to(expected);
}

#[test]
fn sniff_pdf_marker_past_the_first_kib_is_not_a_pdf() {
    let mut bytes = vec![0xFF; 1100];
    bytes.extend_from_slice(b"\n%PDF-1.7\n");
    assert_that!(sniff_document(&bytes, "report.pdf")).is_equal_to(Err(SniffRefusal::TypeMismatch));
}
