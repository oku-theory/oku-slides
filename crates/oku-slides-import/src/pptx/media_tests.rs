use rstest::rstest;
use speculoos::prelude::*;

use super::{OCTET_STREAM, SVG, sniff};

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
