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
