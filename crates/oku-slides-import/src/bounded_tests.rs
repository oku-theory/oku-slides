use speculoos::prelude::*;

use super::*;
use crate::ImportErrorKind;
use crate::pptx::test_support::{LIMITS, zip_of, zip64_tail};

fn open_err(bytes: &[u8], limits: &ReadLimits) -> Option<ImportErrorKind> {
    BoundedZip::open(bytes, limits).err().map(|e| e.kind())
}

#[test]
fn declared_entries_reads_the_end_of_central_directory() {
    let bytes = zip_of(&[("a", b"1"), ("b", b"2"), ("c", b"3")]);
    assert_that!(declared_entries(&bytes)).is_equal_to(Some(3));
}

#[test]
fn declared_entries_follows_the_zip64_locator() {
    assert_that!(declared_entries(&zip64_tail(70_000, true))).is_equal_to(Some(70_000));
}

#[test]
fn declared_entries_saturated_without_zip64_locator_is_unknown() {
    assert_that!(declared_entries(&zip64_tail(70_000, false))).is_none();
}

/// A real archive whose end of central directory claims 60 000 entries: indexing it would fail
/// as unreadable, so `TooLarge` proves the announced count is checked first.
#[test]
fn bounded_zip_entry_count_from_eocd_checked_first() {
    let mut bytes = zip_of(&[("a", b"1"), ("b", b"2")]);
    let at = bytes.windows(4).rposition(|w| w == EOCD).unwrap_or_else(|| panic!("no EOCD"));
    let forged = 60_000u16.to_le_bytes();
    bytes[at + 8..at + 10].copy_from_slice(&forged);
    bytes[at + 10..at + 12].copy_from_slice(&forged);
    assert_that!(open_err(&bytes, &ReadLimits { max_parts: 1000, ..LIMITS }))
        .is_equal_to(Some(ImportErrorKind::TooLarge));
    assert_that!(open_err(&bytes, &ReadLimits { max_parts: 60_000, ..LIMITS }))
        .is_equal_to(Some(ImportErrorKind::Unreadable));
}

#[test]
fn bounded_zip_compound_file_is_protected() {
    let mut cfb = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    cfb.extend_from_slice(&[0; 504]);
    assert_that!(open_err(&cfb, &LIMITS)).is_equal_to(Some(ImportErrorKind::Protected));
}

#[test]
fn bounded_zip_read_past_part_limit_is_too_large_then_exhausted() {
    let bytes = zip_of(&[("big", &[7u8; 2048]), ("small", b"x")]);
    let zip = BoundedZip::open(&bytes, &ReadLimits { max_part_bytes: 1024, ..LIMITS }).unwrap();
    assert_that!(zip.read("big").map_err(|e| e.kind()).err()).is_equal_to(Some(ImportErrorKind::TooLarge));
    assert_that!(zip.decompressed()).is_equal_to(1025);
    assert_that!(zip.read("small").map_err(|e| e.kind()).err()).is_equal_to(Some(ImportErrorKind::TooLarge));
    assert_that!(zip.decompressed()).is_equal_to(1025);
}

#[test]
fn bounded_zip_text_strips_bom_and_missing_part_is_unreadable() {
    let bytes = zip_of(&[("a.xml", "\u{feff}<a/>".as_bytes())]);
    let zip = BoundedZip::open(&bytes, &LIMITS).unwrap();
    assert_that!(zip.text("a.xml").ok()).is_equal_to(Some("<a/>".to_owned()));
    assert_that!(zip.exists("b.xml")).is_false();
    assert_that!(zip.read("b.xml").map_err(|e| e.kind()).err()).is_equal_to(Some(ImportErrorKind::Unreadable));
}

#[test]
fn parse_xml_past_nodes_is_too_large() {
    let err = parse_xml("<a><b/><b/><b/></a>", "x.xml", 2).err().map(|e| e.kind());
    assert_that!(err).is_equal_to(Some(ImportErrorKind::TooLarge));
}

#[test]
fn parse_xml_with_dtd_is_unreadable() {
    let err = parse_xml("<!DOCTYPE a [<!ENTITY e \"x\">]><a>&e;</a>", "x.xml", 100).err().map(|e| e.kind());
    assert_that!(err).is_equal_to(Some(ImportErrorKind::Unreadable));
}

#[test]
fn resolve_path_climbs_parent_directories_and_decodes() {
    assert_that!(resolve_path("xl/workbook.xml", "worksheets/sheet%201.xml"))
        .is_equal_to("xl/worksheets/sheet 1.xml".to_owned());
    assert_that!(resolve_path("xl/workbook.xml", "/xl/sharedStrings.xml"))
        .is_equal_to("xl/sharedStrings.xml".to_owned());
}
