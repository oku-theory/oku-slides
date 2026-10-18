use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::{Diagnostic, RenderError, RenderErrorKind, Severity};

fn diag() -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        place: Some("packs/p/widgets.typ:3".into()),
        message: "boom".into(),
        hints: vec!["h".into()],
        at: None,
    }
}

#[rstest]
#[case(RenderError::timeout(10_000), RenderErrorKind::Timeout)]
#[case(RenderError::out_of_memory(2048), RenderErrorKind::OutOfMemory)]
#[case(RenderError::worker_lost("exit 9"), RenderErrorKind::WorkerLost)]
#[case(RenderError::frame("too large"), RenderErrorKind::Frame)]
#[case(RenderError::stale(), RenderErrorKind::Stale)]
#[case(RenderError::pack_disabled("mien"), RenderErrorKind::PackDisabled)]
fn wire_error_round_trip_keeps_render_kind(#[case] e: RenderError, #[case] kind: RenderErrorKind) {
    let json = serde_json::to_string(&WireError::from_render(&e)).unwrap();
    let back: WireError = serde_json::from_str(&json).unwrap();
    assert_that!(back.into_render().kind()).is_equal_to(kind);
}

#[test]
fn wire_error_round_trip_keeps_compile_diagnostics() {
    let e: RenderError = crate::error::CompileSnafu { diagnostics: vec![diag()] }.build().into();
    let back = WireError::from_render(&e).into_render();
    assert_that!(back.diagnostics().to_vec()).is_equal_to(vec![diag()]);
}

#[test]
fn wire_error_round_trip_keeps_registry_compile_diagnostics() {
    let e: crate::RegistryError = crate::error::RegistryCompileSnafu { diagnostics: vec![diag()] }.build().into();
    let back = WireError::from_registry(&e).into_registry();
    assert_that!(back.kind()).is_equal_to(crate::RegistryErrorKind::Compile);
}

#[test]
fn wire_error_round_trip_keeps_registry_duplicate_pack() {
    let e: crate::RegistryError = crate::error::DuplicatePackSnafu { pack: "acme" }.build().into();
    let json = serde_json::to_string(&WireError::from_registry(&e)).unwrap();
    let back: WireError = serde_json::from_str(&json).unwrap();
    assert_that!(back.into_registry().kind()).is_equal_to(crate::RegistryErrorKind::DuplicatePack);
}

#[test]
fn registry_worker_error_exposes_its_render_error() {
    let e = crate::RegistryError::worker(RenderError::timeout(5));
    assert_that!(e.render_error().map(RenderError::kind)).is_equal_to(Some(RenderErrorKind::Timeout));
}

#[rstest]
#[case(RenderErrorKind::Timeout, true)]
#[case(RenderErrorKind::OutOfMemory, true)]
#[case(RenderErrorKind::WorkerLost, true)]
#[case(RenderErrorKind::PackDisabled, true)]
#[case(RenderErrorKind::Compile, false)]
#[case(RenderErrorKind::Stale, false)]
fn render_error_kind_is_bounded_matches_worker_failures(#[case] kind: RenderErrorKind, #[case] bounded: bool) {
    assert_that!(kind.is_bounded()).is_equal_to(bounded);
}

#[test]
fn layout_round_trips_through_json() {
    let layout = crate::Layout { slides: vec![crate::SlidePage { slide: "s".into(), page: 1 }], ..Default::default() };
    let back: crate::Layout = serde_json::from_str(&serde_json::to_string(&layout).unwrap()).unwrap();
    assert_that!(back.slides).is_equal_to(layout.slides);
}

fn round_trip(e: &RenderError) -> RenderError {
    let json = serde_json::to_string(&WireError::from_render(e)).unwrap();
    serde_json::from_str::<WireError>(&json).unwrap().into_render()
}

#[test]
fn wire_error_round_trip_keeps_pdf_diagnostics() {
    let e: RenderError = crate::error::PdfSnafu { diagnostics: vec![diag()] }.build().into();
    let back = round_trip(&e);
    assert_that!(back.kind()).is_equal_to(RenderErrorKind::Pdf);
    assert_that!(back.diagnostics().to_vec()).is_equal_to(vec![diag()]);
}

#[test]
fn wire_error_round_trip_keeps_no_page_index() {
    let e: RenderError = crate::error::NoPageSnafu { index: 7usize }.build().into();
    let back = round_trip(&e);
    assert_that!(back.kind()).is_equal_to(RenderErrorKind::NoPage);
    assert_that!(back.to_string()).is_equal_to(e.to_string());
}

#[test]
fn wire_error_round_trip_keeps_image_message() {
    let e: RenderError = crate::error::ImageSnafu { message: "bad pixmap".to_owned() }.build().into();
    let back = round_trip(&e);
    assert_that!(back.kind()).is_equal_to(RenderErrorKind::Image);
    assert_that!(back.to_string()).is_equal_to(e.to_string());
}

fn registry_round_trip(e: &crate::RegistryError) -> crate::RegistryError {
    let json = serde_json::to_string(&WireError::from_registry(e)).unwrap();
    serde_json::from_str::<WireError>(&json).unwrap().into_registry()
}

#[test]
fn wire_error_round_trip_keeps_registry_bad_pack_id() {
    let e: crate::RegistryError = crate::error::BadPackIdSnafu { id: "../x" }.build().into();
    let back = registry_round_trip(&e);
    assert_that!(back.kind()).is_equal_to(crate::RegistryErrorKind::BadPackId);
    assert_that!(back.to_string()).is_equal_to(e.to_string());
}

#[test]
fn wire_error_round_trip_keeps_registry_missing() {
    let e: crate::RegistryError = crate::error::MissingSnafu.build().into();
    assert_that!(registry_round_trip(&e).kind()).is_equal_to(crate::RegistryErrorKind::Missing);
}

#[test]
fn wire_error_round_trip_keeps_registry_schema_message() {
    let e: crate::RegistryError = crate::error::RemoteSchemaSnafu { message: "bad param" }.build().into();
    let back = registry_round_trip(&e);
    assert_that!(back.kind()).is_equal_to(crate::RegistryErrorKind::Schema);
    assert_that!(back.to_string()).is_equal_to(e.to_string());
}

#[test]
fn wire_error_round_trip_keeps_registry_worker_render_kind() {
    let e = crate::RegistryError::worker(RenderError::out_of_memory(9));
    let back = registry_round_trip(&e);
    assert_that!(back.render_error().map(RenderError::kind)).is_equal_to(Some(RenderErrorKind::OutOfMemory));
}
