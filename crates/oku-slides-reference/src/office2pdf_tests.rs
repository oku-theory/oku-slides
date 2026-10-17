use std::io::{Cursor, Read, Write};
use std::path::Path;

use rstest::rstest;
use speculoos::prelude::*;
use zip::write::SimpleFileOptions;

use super::*;
use crate::{ReferenceErrorKind, SlideSpan};

fn fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/import").join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Les polices de l'app employées par les fixtures (Inter, Carlito, Liberation Sans).
fn fonts() -> Vec<Vec<u8>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts");
    ["inter-400.ttf", "inter-700.ttf", "carlito-400.ttf", "liberation-sans-400.ttf"]
        .iter()
        .map(|f| std::fs::read(dir.join(f)).unwrap_or_else(|e| panic!("{f}: {e}")))
        .collect()
}

/// Les noms des parts de `pptx`.
fn names(pptx: &[u8]) -> Vec<String> {
    let zip = zip::ZipArchive::new(Cursor::new(pptx)).unwrap();
    zip.file_names().map(|n| n.unwrap().into_owned()).collect()
}

/// `pptx` avec une part de plus, `name`.
fn with_part(pptx: &[u8], name: &str, bytes: &[u8]) -> Vec<u8> {
    let mut src = zip::ZipArchive::new(Cursor::new(pptx)).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..src.len() {
        out.raw_copy_file(src.by_index_raw(i).unwrap()).unwrap();
    }
    out.start_file(name, SimpleFileOptions::default()).unwrap();
    out.write_all(bytes).unwrap();
    out.finish().unwrap().into_inner()
}

/// `pptx` avec la slide `part` masquée (`show="0"`).
pub(crate) fn hide(pptx: &[u8], part: &str) -> Vec<u8> {
    let mut src = zip::ZipArchive::new(Cursor::new(pptx)).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..src.len() {
        let mut f = src.by_index(i).unwrap();
        let name = f.name().unwrap().into_owned();
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes).unwrap();
        if name == part {
            let xml = String::from_utf8(bytes).unwrap().replacen("<p:sld ", "<p:sld show=\"0\" ", 1);
            bytes = xml.into_bytes();
        }
        out.start_file(name, SimpleFileOptions::default()).unwrap();
        out.write_all(&bytes).unwrap();
    }
    out.finish().unwrap().into_inner()
}

#[test]
fn render_libreoffice_fixture_gives_one_page_per_slide() {
    let pdf = Office2Pdf.render(&fixture("libreoffice.pptx"), fonts(), None).unwrap();
    assert_that!(pdf.page_count()).is_equal_to(3);
}

#[test]
fn render_hidden_slide_keeps_its_page() {
    let pptx = hide(&fixture("libreoffice.pptx"), "ppt/slides/slide2.xml");
    let pdf = Office2Pdf.render(&pptx, fonts(), None).unwrap();
    assert_that!(pdf.page_count()).is_equal_to(3);
}

#[test]
fn render_garbage_is_a_render_error() {
    let err = Office2Pdf.render(b"not a zip", vec![], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Render);
}

#[test]
fn witness_names_the_version_pinned_in_the_workspace() {
    let manifest = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml")).unwrap();
    let pin = manifest.lines().find(|l| l.starts_with("office2pdf = ")).unwrap_or_default().to_owned();
    let version = WITNESS.trim_start_matches("office2pdf ");
    assert_that!(pin).contains(format!(r#"version = "={version}""#));
}

#[test]
fn render_of_a_slide_span_gives_its_pages_only() {
    let span = SlideSpan { first: 2, last: 3 };
    let pdf = Office2Pdf.render(&fixture("libreoffice.pptx"), fonts(), Some(span)).unwrap();
    assert_that!(pdf.page_count()).is_equal_to(2);
}

#[test]
fn without_fonts_drops_the_embedded_font_parts() {
    let pptx = with_part(&fixture("libreoffice.pptx"), "ppt/fonts/font1.fntdata", b"police");
    let stripped = without_fonts(&pptx).unwrap();
    assert_that!(names(&stripped).iter().any(|n| n.starts_with("ppt/fonts/"))).is_false();
}

#[test]
fn without_fonts_keeps_every_other_part() {
    let pptx = fixture("libreoffice.pptx");
    let with = with_part(&pptx, "ppt/fonts/font1.fntdata", b"police");
    assert_that!(names(&without_fonts(&with).unwrap())).is_equal_to(names(&pptx));
}

#[test]
fn render_of_a_presentation_with_embedded_fonts_still_renders() {
    let pptx = with_part(&fixture("libreoffice.pptx"), "ppt/fonts/font1.fntdata", b"police");
    assert_that!(Office2Pdf.render(&pptx, fonts(), None).map(|p| p.page_count()).ok()).is_equal_to(Some(3));
}

#[rstest]
#[case::fonts_dir("ppt/fonts/font1.fntdata", true)]
#[case::media_dir("ppt/media/{01234567-89AB-CDEF-0123-456789ABCDEF}.fntdata", true)]
#[case::upper_case("ppt/media/X.FNTDATA", true)]
#[case::anything_under_fonts_dir("ppt/fonts/readme.txt", true)]
#[case::image("ppt/media/image1.png", false)]
#[case::extension_alone_in_a_name("ppt/media/fntdata", false)]
#[case::bare_extension(".fntdata", true)]
#[case::shorter_than_extension("a", false)]
#[case::empty("", false)]
fn is_font_of_a_part_name_is_true_for_embedded_fonts(#[case] name: &str, #[case] font: bool) {
    assert_that!(is_font(name)).is_equal_to(font);
}

#[test]
fn without_fonts_drops_a_font_part_outside_fonts_dir() {
    let pptx =
        with_part(&fixture("libreoffice.pptx"), "ppt/media/{01234567-89AB-CDEF-0123-456789ABCDEF}.fntdata", b"police");
    let stripped = without_fonts(&pptx).unwrap();
    assert_that!(names(&stripped).iter().any(|n| n.ends_with(".fntdata"))).is_false();
}

#[test]
fn office2pdf_view_of_a_font_part_is_refused() {
    let pptx = with_part(&fixture("libreoffice.pptx"), "ppt/media/X.fntdata", b"police");
    assert_that!(ensure_no_font_for_office2pdf(&pptx)).is_err();
}

#[test]
fn office2pdf_view_of_a_stripped_deck_is_accepted() {
    let pptx = with_part(&fixture("libreoffice.pptx"), "ppt/media/X.fntdata", b"police");
    assert_that!(ensure_no_font_for_office2pdf(&without_fonts(&pptx).unwrap())).is_ok();
}
