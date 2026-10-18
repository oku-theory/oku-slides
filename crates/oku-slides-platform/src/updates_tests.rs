use std::path::Path;

use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

fn host<'a>(os: &'a str, appimage: bool, exe: &'a str, failed_before: bool) -> Host<'a> {
    Host { os, appimage, exe: Path::new(exe), failed_before, debug_build: false }
}

fn debug_host<'a>(os: &'a str, appimage: bool, exe: &'a str) -> Host<'a> {
    Host { debug_build: true, ..host(os, appimage, exe, false) }
}

#[rstest]
#[case(host("macos", false, "/Applications/Oku Slides.app/Contents/MacOS/oku-slides", false), InstallMode::Install)]
#[case(host("windows", false, "C:\\Program Files\\Oku Slides\\oku-slides.exe", false), InstallMode::Install)]
#[case(host("linux", true, "/tmp/.mount_oku/usr/bin/oku-slides", false), InstallMode::Install)]
#[case(host("linux", false, "/usr/bin/oku-slides", false), InstallMode::NotifyOnly)]
#[case(
    host(
        "macos",
        false,
        "/private/var/folders/x/AppTranslocation/1/d/Oku Slides.app/Contents/MacOS/oku-slides",
        false
    ),
    InstallMode::NotifyOnly
)]
#[case(
    host("macos", false, "/Volumes/Oku Slides/Oku Slides.app/Contents/MacOS/oku-slides", false),
    InstallMode::NotifyOnly
)]
#[case(host("macos", false, "/Applications/Oku Slides.app/Contents/MacOS/oku-slides", true), InstallMode::NotifyOnly)]
#[case(debug_host("macos", false, "/Applications/Oku Slides.app/Contents/MacOS/oku-slides"), InstallMode::NotifyOnly)]
#[case(host("macos", false, "/Users/dev/oku/target/debug/oku-slides", false), InstallMode::NotifyOnly)]
#[case(debug_host("linux", true, "/tmp/.mount_oku/usr/bin/oku-slides"), InstallMode::NotifyOnly)]
fn install_mode_detect_by_host(#[case] h: Host<'_>, #[case] expected: InstallMode) {
    assert_that!(InstallMode::detect(&h)).is_equal_to(expected);
}

#[test]
fn update_status_serializes_snake_case() {
    let status = UpdateStatus {
        offer: Some(UpdateOffer { version: "2.2.0".into(), mode: InstallMode::NotifyOnly }),
        installing: false,
    };
    assert_that!(serde_json::to_value(&status).unwrap()).is_equal_to(serde_json::json!({
        "offer": { "version": "2.2.0", "mode": "notify_only" },
        "installing": false
    }));
}

#[test]
fn progress_first_chunk_is_reported() {
    let mut p = Progress::new();
    assert_that!(p.advance(10, Some(1000)))
        .is_equal_to(Advance::Report(UpdateProgress { received: 10, total: Some(1000) }));
}

#[test]
fn progress_below_one_percent_is_quiet() {
    let mut p = Progress::new();
    p.advance(10, Some(1000));
    assert_that!(p.advance(5, Some(1000))).is_equal_to(Advance::Quiet);
}

#[test]
fn progress_one_percent_more_is_reported() {
    let mut p = Progress::new();
    p.advance(10, Some(1000));
    assert_that!(p.advance(10, Some(1000)))
        .is_equal_to(Advance::Report(UpdateProgress { received: 20, total: Some(1000) }));
}

#[test]
fn progress_unknown_total_reports_each_mebibyte() {
    let mut p = Progress::new();
    p.advance(1, None);
    assert_that!(p.advance(512 * 1024, None)).is_equal_to(Advance::Quiet);
    assert_that!(p.advance(512 * 1024, None))
        .is_equal_to(Advance::Report(UpdateProgress { received: 1 + 1024 * 1024, total: None }));
}

#[test]
fn progress_reaching_total_is_reported() {
    let mut p = Progress::new();
    p.advance(995, Some(1000));
    assert_that!(p.advance(5, Some(1000)))
        .is_equal_to(Advance::Report(UpdateProgress { received: 1000, total: Some(1000) }));
}

#[test]
fn progress_past_total_reports_once() {
    let mut p = Progress::new();
    p.advance(1000, Some(1000));
    assert_that!(p.advance(1, Some(1000))).is_equal_to(Advance::Quiet);
}

#[test]
fn progress_zero_total_is_treated_as_unknown() {
    let mut p = Progress::new();
    assert_that!(p.advance(10, Some(0))).is_equal_to(Advance::Report(UpdateProgress { received: 10, total: None }));
}

#[test]
fn progress_over_cap_is_too_large() {
    let mut p = Progress::new();
    p.advance(usize::try_from(MAX_DOWNLOAD_BYTES).unwrap(), None);
    assert_that!(p.advance(1, None)).is_equal_to(Advance::TooLarge);
}

#[test]
fn progress_announced_total_over_cap_is_too_large() {
    let mut p = Progress::new();
    assert_that!(p.advance(1, Some(MAX_DOWNLOAD_BYTES + 1))).is_equal_to(Advance::TooLarge);
}

#[rstest]
#[case("https://github.com/oku-theory/oku-slides/releases/download/v2.2.0/Oku.Slides_aarch64.app.tar.gz", true)]
#[case("http://github.com/oku-theory/oku-slides/releases/download/v2.2.0/x", false)]
#[case("https://github.com/someone/else/releases/download/v2.2.0/x", false)]
#[case("https://github.com.evil.test/oku-theory/oku-slides/releases/download/v2.2.0/x", false)]
#[case("https://github.com/oku-theory/oku-slides/releases/download/../../../x", false)]
// L'hôte passe `Url::as_str()`, déjà normalisé (segments `.` résolus par l'analyseur d'URL) :
// cette ligne ne documente que le comportement sur la chaîne brute.
#[case("https://github.com/oku-theory/oku-slides/releases/download/%2e%2e/x", true)]
fn trusted_download_accepts_only_our_release_assets(#[case] url: &str, #[case] expected: bool) {
    assert_that!(trusted_download(url)).is_equal_to(expected);
}

#[rstest]
#[case(UpdateStage::Check, "update-error-check")]
#[case(UpdateStage::Download, "update-error-download")]
#[case(UpdateStage::Open, "update-error-open")]
#[case(UpdateStage::Unavailable, "update-error-unavailable")]
#[case(UpdateStage::Refused, "update-error-refused")]
fn update_stage_key_exists_in_english_and_french(#[case] stage: UpdateStage, #[case] key: &str) {
    assert_that!(stage.key()).is_equal_to(key);
    assert_that!(Localizer::new(Some("en")).has(key)).is_true();
    assert_that!(Localizer::new(Some("fr")).has(key)).is_true();
}

#[test]
fn update_stage_localize_speaks_french() {
    let fr = UpdateStage::Check.localize(&Localizer::new(Some("fr")));
    assert_that!(fr).is_not_equal_to(UpdateStage::Check.localize(&Localizer::new(Some("en"))));
}
