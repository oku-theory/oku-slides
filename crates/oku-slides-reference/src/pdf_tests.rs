use std::path::Path;

use speculoos::prelude::*;

use super::*;
use crate::{Office2Pdf, ReferenceErrorKind, ReferenceRenderer};

/// Le PDF du témoin pour `libreoffice.pptx` (16:9, trois slides).
fn libreoffice() -> ReferencePdf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pptx = std::fs::read(root.join("fixtures/import/libreoffice.pptx")).unwrap();
    let font = std::fs::read(root.join("assets/fonts/carlito-400.ttf")).unwrap();
    Office2Pdf.render(&pptx, vec![font], None).unwrap()
}

#[test]
fn raster_keeps_the_page_ratio() {
    let r = libreoffice().raster(0, 960).unwrap();
    assert_that!((r.width(), r.height())).is_equal_to((960, 540));
}

#[test]
fn raster_draws_the_page_content() {
    let r = libreoffice().raster(0, 960).unwrap();
    let not_white = r.rgba().as_chunks::<4>().0.iter().filter(|p| p[..3] != [255, 255, 255]).count();
    assert_that!(not_white).is_greater_than(1000);
}

#[test]
fn raster_is_opaque() {
    let r = libreoffice().raster(1, 480).unwrap();
    assert_that!(r.rgba().as_chunks::<4>().0.iter().all(|p| p[3] == 255)).is_true();
}

#[test]
fn raster_beyond_last_page_is_no_page() {
    let err = libreoffice().raster(3, 960).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::NoPage);
}

#[test]
fn raster_too_wide_is_a_size_error() {
    let err = libreoffice().raster(0, MAX_RASTER_SIDE + 1).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Size);
}

#[test]
fn new_without_pdf_is_a_pdf_error() {
    let err = ReferencePdf::new(b"%PDF-garbage".to_vec()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Pdf);
}
