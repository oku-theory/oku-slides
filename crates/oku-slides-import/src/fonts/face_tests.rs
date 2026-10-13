use rstest::rstest;
use speculoos::prelude::*;

use super::*;

const CARLITO: &[u8] = include_bytes!("../../../../assets/fonts/carlito-400.ttf");
const CARLITO_BOLD_ITALIC: &[u8] = include_bytes!("../../../../assets/fonts/carlito-700-italic.ttf");
const LIBERATION_SANS_BOLD: &[u8] = include_bytes!("../../../../assets/fonts/liberation-sans-700.ttf");
const CALADEA_ITALIC: &[u8] = include_bytes!("../../../../assets/fonts/caladea-400-italic.ttf");
const INTER_LIGHT: &[u8] = include_bytes!("../../../../assets/fonts/inter-300.ttf");

#[rstest]
#[case(CARLITO, "Carlito", FaceStyle::Regular, 400)]
#[case(CARLITO_BOLD_ITALIC, "Carlito", FaceStyle::BoldItalic, 700)]
#[case(LIBERATION_SANS_BOLD, "Liberation Sans", FaceStyle::Bold, 700)]
#[case(CALADEA_ITALIC, "Caladea", FaceStyle::Italic, 400)]
#[case(INTER_LIGHT, "Inter", FaceStyle::Regular, 300)]
fn read_real_face_gives_family_style_and_weight(
    #[case] data: &[u8],
    #[case] family: &str,
    #[case] style: FaceStyle,
    #[case] weight: u16,
) {
    let info = read(data, 0);
    assert_that!(info.as_ref().map(|i| (i.family.as_str(), i.style, i.weight)))
        .is_equal_to(Some((family, style, weight)));
    assert_that!(info.map(|i| i.aliases.contains(&family.to_lowercase()))).is_equal_to(Some(true));
}

#[test]
fn read_real_face_gives_installable_rights() {
    assert_that!(read(CARLITO, 0).map(|i| restricted(i.fs_type))).is_equal_to(Some(false));
}

#[rstest]
#[case(&[])]
#[case(b"not a font at all")]
#[case(b"\x00\x01\x00\x00\x00\xff")]
#[case(b"ttcf\x00\x01\x00\x00\xff\xff\xff\xff")]
fn read_garbage_gives_none(#[case] data: &[u8]) {
    assert_that!(read(data, 0)).is_none();
}

#[rstest]
#[case(CARLITO, 1)]
#[case(b"ttcf\x00\x01\x00\x00\x00\x00\x00\x02\x00\x00\x00\x14\x00\x00\x00\x14", 2)]
#[case(b"ttcf", 0)]
#[case(b"ttcf\x00\x01\x00\x00\xff\xff\xff\xff", 0)]
#[case(b"ttcf\x00\x01\x00\x00\x00\x00\x00\x03\x00\x00\x00\x14", 0)]
fn count_gives_faces_of_a_file(#[case] data: &[u8], #[case] n: u32) {
    assert_that!(count(data)).is_equal_to(n);
}

#[rstest]
#[case(0x0000, false)]
#[case(0x0002, true)]
#[case(0x0102, true)]
#[case(0x0004, false)]
#[case(0x0006, false)]
#[case(0x000A, false)]
fn restricted_follows_the_least_restrictive_bit(#[case] fs_type: u16, #[case] expected: bool) {
    assert_that!(restricted(fs_type)).is_equal_to(expected);
}

#[rstest]
#[case(b"OTTO\x00", "otf")]
#[case(b"ttcf\x00", "ttc")]
#[case(b"\x00\x01\x00\x00", "ttf")]
#[case(b"", "ttf")]
fn extension_follows_the_magic(#[case] data: &[u8], #[case] ext: &str) {
    assert_that!(extension(data)).is_equal_to(ext);
}

#[rstest]
#[case("Calibri Light", "Calibri")]
#[case("Liberation Sans", "Liberation Sans")]
#[case("Noto Sans Display ExtraBold", "Noto Sans Display")]
#[case("Arial-BoldMT", "Arial-BoldMT")]
#[case("Source Sans Pro Semi Bold", "Source Sans Pro")]
#[case(" .SF NS Text Regular ", "SF NS Text")]
#[case("Inter", "Inter")]
fn typographic_family_trims_style_suffixes_like_typst(#[case] name: &str, #[case] family: &str) {
    assert_that!(typographic_family(name)).is_equal_to(family);
}

#[test]
fn read_real_face_gives_its_line_metrics_in_em() {
    let m = read(CARLITO, 0).and_then(|i| i.metrics()).unwrap();
    assert_that!(m.cap_height).is_close_to(0.642, 0.001);
    // `hhea` de Carlito : 1950 et −550 sur 2048.
    assert_that!(m.ascender).is_close_to(0.952, 0.001);
    assert_that!(m.descender).is_close_to(0.269, 0.001);
}
