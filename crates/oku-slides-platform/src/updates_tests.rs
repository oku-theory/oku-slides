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
    let status =
        UpdateStatus::new(Some(UpdateOffer { version: "2.2.0".into(), mode: InstallMode::NotifyOnly }), false, None);
    assert_that!(serde_json::to_value(&status).unwrap()).is_equal_to(serde_json::json!({
        "offer": { "version": "2.2.0", "mode": "notify_only" },
        "installing": false,
        "dismissed": false
    }));
}

fn offer(version: &str) -> UpdateOffer {
    UpdateOffer { version: version.into(), mode: InstallMode::Install }
}

#[rstest]
#[case(Some(offer("8.1.0")), false, Some("8.1.0"), true)]
#[case(Some(offer("8.1.0")), true, Some("8.1.0"), true)]
#[case(Some(offer("8.2.0")), false, Some("8.1.0"), false)]
#[case(Some(offer("8.1.0")), false, None, false)]
#[case(None, false, Some("8.1.0"), false)]
fn update_status_new_dismissed_only_for_the_version_put_off(
    #[case] offer: Option<UpdateOffer>,
    #[case] installing: bool,
    #[case] later: Option<&str>,
    #[case] dismissed: bool,
) {
    let status = UpdateStatus::new(offer, installing, later);
    assert_that!((status.dismissed, status.installing)).is_equal_to((dismissed, installing));
}

const HOUR: Duration = Duration::from_secs(3600);

fn at(secs: u64) -> std::time::SystemTime {
    std::time::UNIX_EPOCH + Duration::from_secs(secs)
}

#[rstest]
#[case(None, at(0), 6 * HOUR, true)]
#[case(Some(at(1_000_000)), at(1_000_000 + 6 * 3600 - 1), 6 * HOUR, false)]
#[case(Some(at(1_000_000)), at(1_000_000 + 6 * 3600), 6 * HOUR, true)]
#[case(Some(at(1_000_000)), at(1_000_000 + 30 * 3600), 6 * HOUR, true)]
#[case(Some(at(1_000_000)), at(999_000), 6 * HOUR, true)]
#[case(Some(at(1_000_000)), at(2_000_000), Duration::from_secs(u64::MAX), false)]
fn check_due_by_wall_clock(
    #[case] last: Option<std::time::SystemTime>,
    #[case] now: std::time::SystemTime,
    #[case] interval: Duration,
    #[case] due: bool,
) {
    assert_that!(check_due(last, now, interval)).is_equal_to(due);
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
#[case("https://updates.okutheory.com/v2.2.0/Oku.Slides_aarch64.app.tar.gz", true)]
#[case("http://updates.okutheory.com/v2.2.0/x", false)]
#[case("https://updates.okutheory.com.evil.test/v2.2.0/x", false)]
#[case("https://updates.okutheory.com/v2.2.0/../../x", false)]
#[case("https://updates.okutheory.com/latest.json", false)]
#[case("https://updates.okutheory.com/vendor/x", false)]
#[case("https://updates.okutheory.com/v2.2.0x/x", false)]
#[case("https://updates.okutheory.com/v2.2.0/", false)]
#[case("https://updates.okutheory.com/v2.2.0/a/b", false)]
#[case("https://updates.okutheory.com/v2.2/x", false)]
// L'hôte passe `Url::as_str()`, déjà normalisé (segments `.` résolus par l'analyseur d'URL) :
// cette ligne ne documente que le comportement sur la chaîne brute.
#[case("https://github.com/oku-theory/oku-slides/releases/download/%2e%2e/x", true)]
fn download_source_accepts_only_our_release_assets(#[case] url: &str, #[case] expected: bool) {
    assert_that!(download_source(url).is_some()).is_equal_to(expected);
}

#[rstest]
#[case(UpdateStage::Check, "update-error-check")]
#[case(UpdateStage::Download, "update-error-download")]
#[case(UpdateStage::Open, "update-error-open")]
#[case(UpdateStage::Unavailable, "update-error-unavailable")]
#[case(UpdateStage::Refused, "update-error-refused")]
#[case(UpdateStage::Busy, "update-error-busy")]
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

#[rstest]
#[case("en", "8.1.0")]
#[case("fr", "8.1.0")]
fn install_failure_localized_names_the_version(#[case] locale: &str, #[case] version: &str) {
    let l = Localizer::new(Some(locale));
    assert_that!(l.has("update-error-install")).is_true();
    let text = InstallFailure { version: version.into() }.localize(&l);
    assert_that!(text.contains(version)).is_true();
    // Its own key, not another one that also names the version.
    assert_that!(text).is_not_equal_to(UpdateStage::Busy.localize(&l));
}

#[test]
fn install_failure_speaks_french() {
    let failure = InstallFailure { version: "8.1.0".into() };
    let fr = failure.localize(&Localizer::new(Some("fr")));
    assert_that!(fr).is_not_equal_to(failure.localize(&Localizer::new(Some("en"))));
}

#[test]
fn update_stage_busy_speaks_french() {
    let fr = UpdateStage::Busy.localize(&Localizer::new(Some("fr")));
    assert_that!(fr).is_not_equal_to(UpdateStage::Busy.localize(&Localizer::new(Some("en"))));
}

#[test]
fn mirror_prefix_matches_tauri_endpoint_and_workflow() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let conf = std::fs::read_to_string(root.join("src-tauri/tauri.conf.json")).unwrap();
    let conf: serde_json::Value = serde_json::from_str(&conf).unwrap();
    let host = MIRROR_DOWNLOAD_PREFIX.strip_suffix("/v").unwrap();
    let workflow = std::fs::read_to_string(root.join(".github/workflows/update-mirror.yml")).unwrap();
    assert_that!(conf["plugins"]["updater"]["endpoints"][0].as_str())
        .is_equal_to(Some(format!("{host}/latest.json").as_str()));
    assert_that!(workflow.contains(&format!("MIRROR_URL: {host}\n"))).is_true();
    // t-1210: the signature must bind the version latest.json announces.
    assert_that!(conf["plugins"]["updater"]["requireSignedVersion"].as_bool()).is_equal_to(Some(true));
}

#[rstest]
#[case(UpdateSource::Mirror, "mirror")]
#[case(UpdateSource::GitHub, "github")]
fn update_source_display_is_its_label(#[case] source: UpdateSource, #[case] label: &str) {
    assert_that!(source.to_string()).is_equal_to(label.to_owned());
}
