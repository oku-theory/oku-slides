//! Documents refused whole: damaged, of another kind, or not read yet.

#![expect(clippy::panic, reason = "test helpers fail loudly on a broken fixture")]

mod common;

use common::extract;
use common::{limits, ods, read, xlsx};
use oku_slides_core::knowledge::{DocumentType, ExtractRefusal};
use oku_slides_import::ReadLimits;
use speculoos::prelude::*;

fn refusal(bytes: &[u8], kind: DocumentType) -> Option<ExtractRefusal> {
    extract(bytes, kind, &limits(), &read(), "0123456789abcdef").err()
}

#[test]
fn damaged_zip_is_damaged() {
    let mut bytes = xlsx(2, 2);
    bytes.truncate(bytes.len() / 2);
    assert_that!(refusal(&bytes, DocumentType::Xlsx)).is_equal_to(Some(ExtractRefusal::Damaged));
    assert_that!(refusal(&bytes, DocumentType::Ods)).is_equal_to(Some(ExtractRefusal::Damaged));
}

#[test]
fn xlsx_bytes_as_ods_type_mismatch() {
    assert_that!(refusal(&xlsx(2, 2), DocumentType::Ods)).is_equal_to(Some(ExtractRefusal::TypeMismatch));
    assert_that!(refusal(&ods("<table:table table:name=\"T\"/>"), DocumentType::Xlsx))
        .is_equal_to(Some(ExtractRefusal::TypeMismatch));
}

#[test]
fn text_bytes_as_xlsx_type_mismatch() {
    assert_that!(refusal(b"just some text", DocumentType::Xlsx)).is_equal_to(Some(ExtractRefusal::TypeMismatch));
}

#[test]
fn input_past_max_input_bytes_too_large() {
    let tight = ReadLimits::for_extract(4, 512 << 20);
    let err = extract(b"hello", DocumentType::Text, &limits(), &tight, "t").err();
    assert_that!(err).is_equal_to(Some(ExtractRefusal::TooLarge));
}
