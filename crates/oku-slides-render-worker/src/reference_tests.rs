use std::path::Path;

use oku_slides_platform::RenderLaunch;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::{Fonts, RenderErrorKind};
use speculoos::prelude::*;

use crate::{ReferenceWitness, RenderWorker};

fn worker(timeout_ms: u64) -> RenderWorker {
    RenderWorker::new(
        RenderLaunch::InProcess,
        RenderSettings {
            timeout_ms,
            probe_budget_ms: timeout_ms,
            memory_mib: 4096,
            max_frame_mib: 64,
            max_widget_kib: 256,
            max_widgets: 64,
        },
    )
}

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn pptx() -> Vec<u8> {
    std::fs::read(root().join("fixtures/import/libreoffice.pptx")).unwrap()
}

fn fonts() -> Fonts {
    Fonts::new([std::fs::read(root().join("assets/fonts/carlito-400.ttf")).unwrap()])
}

#[test]
fn reference_gives_the_witness_and_its_pages() {
    let r = worker(30_000).reference(&ReferenceWitness::Office2Pdf, &pptx(), &fonts(), None).unwrap();
    assert_that!((r.witness(), r.page_count())).is_equal_to(("office2pdf 0.8.1", 3));
}

#[test]
fn reference_png_has_the_requested_width() {
    let r = worker(30_000).reference(&ReferenceWitness::Office2Pdf, &pptx(), &fonts(), None).unwrap();
    let png = oku_slides_reference::Raster::from_png(&r.png(0, 960).unwrap()).unwrap();
    assert_that!((png.width(), png.height())).is_equal_to((960, 540));
}

#[test]
fn reference_of_garbage_is_an_error_and_the_worker_goes_on() {
    let w = worker(30_000);
    assert_that!(w.reference(&ReferenceWitness::Office2Pdf, b"not a pptx", &fonts(), None).is_err()).is_true();
    assert_that!(w.reference(&ReferenceWitness::Office2Pdf, &pptx(), &fonts(), None).is_ok()).is_true();
}

#[test]
fn reference_beyond_the_timeout_is_a_timeout() {
    let err = worker(1).reference(&ReferenceWitness::Office2Pdf, &pptx(), &fonts(), None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Timeout);
}

#[test]
fn remote_reference_is_send() {
    fn send<T: Send>() {}
    send::<crate::RemoteReference>();
}

#[test]
fn reference_of_a_slide_span_has_its_pages_only() {
    let span = oku_slides_reference::SlideSpan { first: 2, last: 2 };
    let r = worker(30_000).reference(&ReferenceWitness::Office2Pdf, &pptx(), &fonts(), Some(span)).unwrap();
    assert_that!(r.page_count()).is_equal_to(1);
}

/// Un faux `soffice` dans `dir`, de corps `body`, comme témoin.
#[cfg(unix)]
fn soffice(dir: &Path, body: &str, timeout_ms: u64) -> ReferenceWitness {
    let program = oku_slides_reference::test_support::fake_soffice(dir, body);
    ReferenceWitness::Soffice { program, timeout_ms }
}

#[cfg(unix)]
#[test]
fn reference_by_soffice_gives_its_version_and_pages() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("f.pdf");
    std::fs::write(&pdf, oku_slides_reference::test_support::blank_pdf(2, 960, 540)).unwrap();
    let witness = soffice(dir.path(), &oku_slides_reference::test_support::converting(&pdf, None), 120_000);
    let r = worker(150_000).reference(&witness, &pptx(), &fonts(), None).unwrap();
    assert_that!((r.witness(), r.page_count())).is_equal_to(("LibreOffice 99.1.0.1", 2));
}

#[cfg(unix)]
#[test]
fn reference_by_a_hanging_soffice_is_a_timeout_and_the_worker_goes_on() {
    let dir = tempfile::tempdir().unwrap();
    let w = worker(30_000);
    let err = w.reference(&soffice(dir.path(), "sleep 30", 300), &pptx(), &fonts(), None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Timeout);
    assert_that!(w.reference(&ReferenceWitness::Office2Pdf, &pptx(), &fonts(), None).is_ok()).is_true();
}

#[test]
fn reference_witness_crosses_the_pipe_unchanged() {
    let witness = ReferenceWitness::Soffice { program: "/opt/soffice".into(), timeout_ms: 5 };
    let json = serde_json::to_string(&witness).unwrap();
    assert_that!(serde_json::from_str::<ReferenceWitness>(&json).unwrap()).is_equal_to(witness);
}
