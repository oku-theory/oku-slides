use oku_slides_core::knowledge::{DocumentType, ExtractLimits, ExtractRefusal};
use oku_slides_import::ReadLimits;
use speculoos::prelude::*;

use super::{PANIC_PROBE, extract};

fn limits() -> ExtractLimits {
    ExtractLimits {
        max_text_bytes: 1 << 20,
        sheet_rows: 100,
        sheet_columns: 20,
        max_images: 20,
        max_image_pixels: 1 << 26,
        max_image_bytes: 1 << 20,
    }
}

#[test]
fn extract_panic_becomes_damaged() {
    let read = ReadLimits::for_extract(1 << 20, 512 << 20);
    let err = extract(PANIC_PROBE, DocumentType::Text, &limits(), &read, "t").map(Result::err);
    assert_that!(err.ok()).is_equal_to(Some(Some(ExtractRefusal::Damaged)));
}

#[test]
fn profile_is_not_panic_abort() {
    assert_that!(cfg!(panic = "unwind")).is_true();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for file in ["Cargo.toml", ".cargo/config.toml"] {
        let text = std::fs::read_to_string(root.join(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
        let aborts =
            text.lines().map(|l| l.replace([' ', '\t'], "")).any(|l| l.starts_with("panic=") && l.contains("abort"));
        assert_that!(aborts).named(file).is_false();
    }
}
