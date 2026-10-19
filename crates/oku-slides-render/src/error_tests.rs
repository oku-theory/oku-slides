use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

fn en() -> Localizer {
    Localizer::new(Some("en"))
}

#[test]
fn timeout_localizes_in_whole_seconds_at_least_one() {
    let l = en();
    assert_that!(RenderError::timeout(250).localize(&l))
        .is_equal_to(l.tr_with("render-timeout", &[("seconds", 1u64.into())]));
    assert_that!(RenderError::timeout(10_000).localize(&l))
        .is_equal_to(l.tr_with("render-timeout", &[("seconds", 10u64.into())]));
}

#[test]
fn out_of_memory_localizes_with_its_limit() {
    let l = en();
    assert_that!(RenderError::out_of_memory(2048).localize(&l))
        .is_equal_to(l.tr_with("render-out-of-memory", &[("mib", 2048u64.into())]));
}

#[test]
fn pack_disabled_localizes_with_its_pack() {
    let l = en();
    assert_that!(RenderError::pack_disabled("mien").localize(&l))
        .is_equal_to(l.tr_with("render-pack-disabled", &[("pack", "mien".into())]));
}

#[rstest]
#[case(RenderError::worker_lost("exit 9"), "render-worker-lost")]
#[case(RenderError::frame("too large"), "render-frame")]
#[case(RenderError::stale(), "render-stale")]
#[case(RenderError::protocol(2, Some(1)), "render-protocol")]
#[case(RemoteSnafu { message: "x".to_owned() }.build().into(), "render-remote")]
#[case(NoPageSnafu { index: 3usize }.build().into(), "render-no-page")]
#[case(ImageSnafu { message: "x".to_owned() }.build().into(), "render-image")]
fn worker_and_export_errors_localize_to_their_key(#[case] e: RenderError, #[case] key: &str) {
    let l = en();
    assert_that!(e.localize(&l)).is_equal_to(l.tr(key));
}

#[test]
fn page_too_large_is_a_frame_error_with_its_own_message() {
    let l = en();
    let e = RenderError::page_too_large(128);
    assert_that!(e.kind()).is_equal_to(RenderErrorKind::Frame);
    assert_that!(e.localize(&l)).is_equal_to(l.tr_with("render-page-too-large", &[("mib", 128u64.into())]));
    assert_that!(e.localize(&l)).is_not_equal_to(l.tr("render-frame"));
}

#[rstest]
#[case("en")]
#[case("fr")]
fn protocol_mismatch_is_its_own_unbounded_kind_with_its_message(#[case] locale: &str) {
    let l = Localizer::new(Some(locale));
    let e = RenderError::protocol(2, None);
    assert_that!(e.kind()).is_equal_to(RenderErrorKind::Protocol);
    assert_that!(e.kind().is_bounded()).is_false();
    assert_that!(e.localize(&l)).is_equal_to(l.tr("render-protocol"));
    assert_that!(e.localize(&l)).is_not_equal_to(l.tr("render-setup"));
}

#[rstest]
#[case("en")]
#[case("fr")]
fn remote_failure_has_its_own_message_not_the_setup_one(#[case] locale: &str) {
    let l = Localizer::new(Some(locale));
    let e: RenderError = RemoteSnafu { message: "unknown slot 3".to_owned() }.build().into();
    assert_that!(e.kind()).is_equal_to(RenderErrorKind::Setup);
    assert_that!(e.localize(&l)).is_equal_to(l.tr("render-remote"));
    assert_that!(e.localize(&l)).is_not_equal_to(l.tr("render-setup"));
}

#[rstest]
#[case("en", "reinstall the app")]
#[case("fr", "réinstallez")]
fn base_pack_failure_is_its_own_kind_and_localizes_with_its_file(#[case] locale: &str, #[case] fragment: &str) {
    let l = Localizer::new(Some(locale));
    let e: RegistryError = BasePackSnafu { file: "widgets.json".to_owned() }.build().into();
    assert_that!(e.kind()).is_equal_to(RegistryErrorKind::BasePack);
    let text = e.localize(&l);
    assert_that!(text).contains("widgets.json");
    assert_that!(text).contains(fragment);
}
