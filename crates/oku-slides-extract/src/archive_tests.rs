use std::io::{Cursor, Write};

use speculoos::prelude::*;
use zip::write::SimpleFileOptions;

use super::*;

fn zip_of(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        w.start_file(*name, SimpleFileOptions::default()).unwrap();
        w.write_all(data.as_bytes()).unwrap();
    }
    w.finish().unwrap().into_inner()
}

fn compound(with: &[u8]) -> Vec<u8> {
    let mut cfb = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    cfb.extend_from_slice(&[0; 504]);
    cfb.extend_from_slice(with);
    cfb
}

#[test]
fn open_compound_file_encrypted_or_legacy() {
    let read = ReadLimits::default();
    assert_that!(open(&compound(ENCRYPTION_INFO), &read).err()).is_equal_to(Some(ExtractRefusal::Encrypted));
    assert_that!(open(&compound(b"Workbook"), &read).err()).is_equal_to(Some(ExtractRefusal::Legacy));
}

#[test]
fn open_not_a_zip_type_mismatch() {
    assert_that!(open(b"%PDF-1.7 hello", &ReadLimits::default()).err()).is_equal_to(Some(ExtractRefusal::TypeMismatch));
}

#[test]
fn main_part_follows_the_office_document_relationship() {
    let package = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
        <Relationship Id="x" Type="http://e.org/hyperlink" Target="https://e.org" TargetMode="External"/>
        <Relationship Id="r" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="/xl/workbook.xml"/>
    </Relationships>"#;
    let bytes = zip_of(&[("_rels/.rels", package)]);
    let zip = open(&bytes, &ReadLimits::default()).unwrap();
    assert_that!(main_part(&zip)).is_equal_to(Ok("xl/workbook.xml".to_owned()));
    assert_that!(rels(&zip, "").unwrap().len()).is_equal_to(1);
}

#[test]
fn main_part_without_package_relationships_type_mismatch() {
    let bytes = zip_of(&[("content.xml", "<a/>")]);
    let zip = open(&bytes, &ReadLimits::default()).unwrap();
    assert_that!(main_part(&zip)).is_equal_to(Err(ExtractRefusal::TypeMismatch));
}
