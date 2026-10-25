use rstest::rstest;
use speculoos::prelude::*;

use super::{OCTET_STREAM, SVG, dimensions, sniff};

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
