use speculoos::prelude::*;

use super::*;
use crate::ImportErrorKind;
use crate::pptx::test_support::{CT, LIMITS, ROOT_RELS, TestPkg, zip_of};

const PRES_RELS: &str = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/a%20b.png"/>
<Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com" TargetMode="External"/>
</Relationships>"#;

fn minimal() -> TestPkg {
    TestPkg::with(&[("ppt/_rels/presentation.xml.rels", PRES_RELS.as_bytes())])
}

fn open_err(bytes: &[u8], limits: &ReadLimits) -> ImportErrorKind {
    match Package::open(bytes, limits) {
        Ok(_) => panic!("échec attendu"),
        Err(e) => e.kind(),
    }
}

#[test]
fn open_minimal_package_finds_main_part() {
    assert_that!(minimal().pkg().main_part().to_owned()).is_equal_to("ppt/presentation.xml".to_owned());
}

#[test]
fn rels_resolve_targets_relative_to_their_part() {
    let t = minimal();
    let pkg = t.pkg();
    let rels = pkg.rels("ppt/presentation.xml").unwrap();
    assert_that!(rels.target("rId2")).is_equal_to(Some("ppt/slides/slide1.xml"));
    assert_that!(rels.by_type("/slide")).is_equal_to(vec![("rId2", "ppt/slides/slide1.xml")]);
}

#[test]
fn rels_decode_percent_escapes_in_targets() {
    let t = minimal();
    let pkg = t.pkg();
    assert_that!(pkg.rels("ppt/presentation.xml").unwrap().target("rId3")).is_equal_to(Some("ppt/media/a b.png"));
}

#[test]
fn rels_keep_external_targets_apart() {
    let t = minimal();
    let pkg = t.pkg();
    let rels = pkg.rels("ppt/presentation.xml").unwrap();
    assert_that!((rels.target("rId9"), rels.external("rId9"))).is_equal_to((None, Some("https://example.com")));
}

#[test]
fn rels_are_read_once_then_cached() {
    let t = minimal();
    let pkg = t.pkg();
    let a = pkg.rels("ppt/presentation.xml").unwrap();
    let b = pkg.rels("ppt/presentation.xml").unwrap();
    assert_that!(std::rc::Rc::ptr_eq(&a, &b)).is_true();
}

#[test]
fn rels_of_part_without_rels_file_are_empty() {
    let t = minimal();
    let pkg = t.pkg();
    assert_that!(pkg.rels("ppt/slides/slide1.xml").unwrap().by_type("/slide")).is_empty();
}

#[test]
fn resolve_path_climbs_parent_directories() {
    assert_that!(resolve_path("ppt/slides/slide1.xml", "../media/image1.png"))
        .is_equal_to("ppt/media/image1.png".to_owned());
    assert_that!(resolve_path("ppt/slides/slide1.xml", "/ppt/media/x.png")).is_equal_to("ppt/media/x.png".to_owned());
}

#[test]
fn declared_entries_reads_the_end_of_central_directory() {
    let bytes = zip_of(&[("a", b"1"), ("b", b"2"), ("c", b"3")]);
    assert_that!(declared_entries(&bytes)).is_equal_to(Some(3));
}

#[test]
fn open_garbage_is_unreadable() {
    assert_that!(open_err(b"not a zip", &LIMITS)).is_equal_to(ImportErrorKind::Unreadable);
}

#[test]
fn open_compound_file_is_protected() {
    let mut cfb = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    cfb.extend_from_slice(&[0; 504]);
    assert_that!(open_err(&cfb, &LIMITS)).is_equal_to(ImportErrorKind::Protected);
}

#[test]
fn open_word_document_is_not_a_presentation() {
    let ct = CT
        .replace("presentationml.presentation.main", "wordprocessingml.document.main")
        .replace("/ppt/presentation.xml", "/word/document.xml");
    let rels = ROOT_RELS.replace("ppt/presentation.xml", "word/document.xml");
    let bytes = zip_of(&[
        ("[Content_Types].xml", ct.as_bytes()),
        ("_rels/.rels", rels.as_bytes()),
        ("word/document.xml", b"<w/>"),
    ]);
    assert_that!(open_err(&bytes, &LIMITS)).is_equal_to(ImportErrorKind::NotAPresentation);
}

#[test]
fn open_without_office_document_relationship_is_unreadable() {
    let bytes = zip_of(&[("[Content_Types].xml", CT.as_bytes()), ("_rels/.rels", b"<Relationships/>")]);
    assert_that!(open_err(&bytes, &LIMITS)).is_equal_to(ImportErrorKind::Unreadable);
}

#[test]
fn open_input_over_limit_is_too_large() {
    let t = minimal();
    assert_that!(open_err(t.bytes(), &ReadLimits { max_input_bytes: 10, ..LIMITS }))
        .is_equal_to(ImportErrorKind::TooLarge);
}

#[test]
fn open_with_too_many_declared_entries_is_too_large() {
    let t = minimal();
    assert_that!(open_err(t.bytes(), &ReadLimits { max_parts: 2, ..LIMITS })).is_equal_to(ImportErrorKind::TooLarge);
}

#[test]
fn bytes_exactly_at_part_limit_are_accepted_one_more_is_too_large() {
    let t = TestPkg::with(&[("ppt/media/a.bin", &[7u8; 1024]), ("ppt/media/b.bin", &[7u8; 1025])]);
    let pkg = Package::open(t.bytes(), &ReadLimits { max_part_bytes: 1024, ..LIMITS }).unwrap();
    assert_that!(pkg.bytes("ppt/media/a.bin").map(|b| b.len()).ok()).is_equal_to(Some(1024));
    assert_that!(pkg.bytes("ppt/media/b.bin").map_err(|e| e.kind()).err()).is_equal_to(Some(ImportErrorKind::TooLarge));
}

#[test]
fn total_over_limit_is_too_large() {
    let t = TestPkg::with(&[("ppt/media/a.bin", &[7u8; 600]), ("ppt/media/b.bin", &[7u8; 600])]);
    // L'ouverture lit [Content_Types].xml et _rels/.rels ; il reste de quoi lire une image, pas deux.
    let budget = (CT.len() + ROOT_RELS.len() + 900) as u64;
    let pkg = Package::open(t.bytes(), &ReadLimits { max_total_bytes: budget, ..LIMITS }).unwrap();
    assert_that!(pkg.bytes("ppt/media/a.bin").is_ok()).is_true();
    assert_that!(pkg.bytes("ppt/media/b.bin").map_err(|e| e.kind()).err()).is_equal_to(Some(ImportErrorKind::TooLarge));
}

#[test]
fn bytes_after_a_too_large_read_nothing_and_fail() {
    let t = TestPkg::with(&[("ppt/media/big.bin", &[7u8; 2048]), ("ppt/media/small.bin", &[7u8; 8])]);
    let pkg = Package::open(t.bytes(), &ReadLimits { max_part_bytes: 1024, ..LIMITS }).unwrap();
    assert_that!(pkg.bytes("ppt/media/big.bin").map_err(|e| e.kind()).err())
        .is_equal_to(Some(ImportErrorKind::TooLarge));
    let before = pkg.read_total.get();
    assert_that!(pkg.bytes("ppt/media/small.bin").map_err(|e| e.kind()).err())
        .is_equal_to(Some(ImportErrorKind::TooLarge));
    assert_that!(pkg.read_total.get()).is_equal_to(before);
}

#[test]
fn bytes_over_the_part_limit_count_what_was_decompressed() {
    let t = TestPkg::with(&[("ppt/media/big.bin", &[7u8; 2048])]);
    let pkg = Package::open(t.bytes(), &ReadLimits { max_part_bytes: 1024, ..LIMITS }).unwrap();
    let before = pkg.read_total.get();
    assert_that!(pkg.bytes("ppt/media/big.bin").is_err()).is_true();
    assert_that!(pkg.read_total.get()).is_equal_to(before + 1025);
}

#[test]
fn bytes_decompress_at_most_the_remaining_budget() {
    let t = TestPkg::with(&[("ppt/media/a.bin", &[7u8; 900])]);
    let opened = (CT.len() + ROOT_RELS.len()) as u64;
    let pkg = Package::open(t.bytes(), &ReadLimits { max_total_bytes: opened + 100, ..LIMITS }).unwrap();
    assert_that!(pkg.bytes("ppt/media/a.bin").map_err(|e| e.kind()).err()).is_equal_to(Some(ImportErrorKind::TooLarge));
    assert_that!(pkg.read_total.get()).is_equal_to(opened + 101);
}

#[test]
fn rels_unreadable_fail_once_then_read_as_empty_without_rereading() {
    let t = TestPkg::with(&[("ppt/slides/_rels/slide1.xml.rels", b"<Relationships")]);
    let pkg = t.pkg();
    assert_that!(pkg.rels("ppt/slides/slide1.xml").map_err(|e| e.kind()).err())
        .is_equal_to(Some(ImportErrorKind::Unreadable));
    let before = pkg.read_total.get();
    assert_that!(pkg.rels("ppt/slides/slide1.xml").map(|r| r.by_type("").len()).ok()).is_equal_to(Some(0));
    assert_that!(pkg.read_total.get()).is_equal_to(before);
}

#[test]
fn text_and_rels_strip_a_utf8_bom() {
    let rels = format!("\u{feff}{PRES_RELS}");
    let t = TestPkg::with(&[
        ("ppt/_rels/presentation.xml.rels", rels.as_bytes()),
        ("ppt/a.xml", "\u{feff}<a/>".as_bytes()),
    ]);
    let pkg = t.pkg();
    assert_that!(pkg.text("ppt/a.xml").ok()).is_equal_to(Some("<a/>".to_owned()));
    assert_that!(pkg.rels("ppt/presentation.xml").unwrap().target("rId2")).is_equal_to(Some("ppt/slides/slide1.xml"));
}

/// Fin d'archive Zip64 seule : enregistrement Zip64 annonçant `entries`, localisateur, puis EOCD saturé.
fn zip64_tail(entries: u64, locator: bool) -> Vec<u8> {
    let mut v = vec![0u8; 8]; // octets quelconques avant l'enregistrement
    let record = v.len() as u64;
    v.extend_from_slice(&[0x50, 0x4b, 0x06, 0x06]);
    v.extend_from_slice(&44u64.to_le_bytes()); // taille du reste de l'enregistrement
    v.extend_from_slice(&[45, 0, 45, 0, 0, 0, 0, 0, 0, 0, 0, 0]); // versions, disques
    v.extend_from_slice(&entries.to_le_bytes()); // entrées sur ce disque
    v.extend_from_slice(&entries.to_le_bytes()); // entrées en tout
    v.extend_from_slice(&[0; 16]); // taille et décalage du répertoire central
    if locator {
        v.extend_from_slice(&ZIP64_LOCATOR);
        v.extend_from_slice(&[0; 4]);
        v.extend_from_slice(&record.to_le_bytes());
        v.extend_from_slice(&1u32.to_le_bytes());
    }
    v.extend_from_slice(&EOCD);
    v.extend_from_slice(&[0; 6]);
    v.extend_from_slice(&0xFFFFu16.to_le_bytes());
    v.extend_from_slice(&[0; 10]);
    v
}

#[test]
fn declared_entries_follows_the_zip64_locator() {
    assert_that!(declared_entries(&zip64_tail(70_000, true))).is_equal_to(Some(70_000));
}

#[test]
fn declared_entries_saturated_without_zip64_locator_is_unknown() {
    assert_that!(declared_entries(&zip64_tail(70_000, false))).is_none();
}

#[test]
fn open_with_too_many_zip64_entries_is_too_large_before_indexing() {
    assert_that!(open_err(&zip64_tail(70_000, true), &LIMITS)).is_equal_to(ImportErrorKind::TooLarge);
}
