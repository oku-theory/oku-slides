//! The hi-fi import of the third-party calibration corpus (`fixtures/import/corpus`, t-909),
//! scored against real LibreOffice at the calibrated `score_threshold`: run by `just test-soffice`.
//!
//! Measured on 2026-11-06 (LibreOffice 26.8.2.2, threshold 7, after t-909 kept centered
//! placeholders centered), the slides flagged below are each a real gap, seen side by side with
//! LibreOffice and office2pdf: a background the import draws over the placeholders or loses,
//! a missing SmartArt or chart, table run styles, cropped or rotated pictures (spec, Known limits,
//! amendment of 2026-11-06). A slide outside its deck's list must stay unflagged: a new flag is a
//! regression of the import or a false positive of the threshold. A slide may leave the list as
//! the import improves (poi-themes p. 1, a title at 7.6, is the one borderline flag).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::path::Path;

use common::{assert_success, fixtures, oku_slides, workdir};
use rstest::rstest;
use serde_json::Value;
use speculoos::prelude::*;

/// The limits of LibreOffice (t-905): the only suspect regions under it.
const LIBREOFFICE_LIMITS: [&str; 2] = ["no_substitute", "smart_art_style"];

/// Imports corpus deck `name` in hi-fi against real LibreOffice; its report.
fn import_against_libreoffice(dir: &Path, name: &str) -> Value {
    let report = dir.join("r.json");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = oku_slides(dir, "en")
        .arg("--fonts")
        .arg(root.join("assets/fonts"))
        .arg("import")
        .arg(fixtures().join("corpus").join(format!("{name}.pptx")))
        .args(["--mode", "hifi", "--no-system-fonts", "--reference", "soffice", "-o"])
        .arg(dir.join("out.oku-slides.pdf"))
        .arg("--report")
        .arg(&report)
        .output()
        .unwrap_or_else(|e| panic!("oku-slides does not start: {e}"));
    assert_success("import", &out, &[]);
    serde_json::from_slice(&std::fs::read(&report).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("report: {e}"))
}

#[rstest]
#[case("poi-SmartArt", &[])]
#[case("poi-smartart-simple", &[1])]
#[case("poi-table-with-no-theme", &[1])]
#[case("poi-table-with-theme", &[1])]
#[case("poi-themes", &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10])]
#[case("poi-with_japanese", &[1])]
#[case("powertools-AnglesTheme", &[1])]
#[case("powertools-ApexTheme", &[1])]
#[case("powertools-WaveFormTheme", &[1])]
#[case("pythonpptx-cht-chart-type", &[1, 2, 3, 4, 5, 6, 7, 8])]
#[case("pythonpptx-shp-groupshape", &[1])]
#[case("pythonpptx-shp-picture", &[1, 2])]
#[case("sdk-Chart_2D", &[1])]
#[case("sdk-Photo_Album3", &[])]
#[case("sdk-SmartArt_OrgChart1", &[1])]
#[case("sdk-SmartArt_OrgChart1_3D", &[1])]
#[case("sdk-Table_Large", &[1])]
#[case("tika-testMSChart-govdocs-428996", &[1])]
#[case("tika-testPPT_groups", &[2])]
#[ignore = "real LibreOffice, it opens in the Dock: just test-soffice"]
fn import_hifi_of_the_corpus_against_real_libreoffice_flags_only_measured_gaps(
    #[case] name: &str,
    #[case] measured: &[usize],
) {
    let dir = workdir("import-corpus-soffice");
    let report = import_against_libreoffice(dir.as_ref(), name);
    assert_that!(report["reference"]["unavailable"].is_null()).is_true();
    assert_that!(report["reference"]["threshold"].as_f64()).is_equal_to(Some(7.0));
    let slides = report["slides"].as_array().cloned().unwrap_or_else(|| panic!("no slides in the report"));
    assert_that!(slides).is_not_empty();
    assert_that!(slides.iter().all(|s| s["score"]["score"].is_f64())).is_true();
    let flagged: Vec<usize> =
        (1..=slides.len()).filter(|n| slides[n - 1]["above_threshold"].as_bool() == Some(true)).collect();
    let unexpected: Vec<usize> = flagged.into_iter().filter(|n| !measured.contains(n)).collect();
    assert_that!(unexpected).is_empty();
    let suspects = slides
        .iter()
        .flat_map(|s| s["score"]["regions"].as_array().cloned().unwrap_or_default())
        .filter_map(|r| r["suspect"].as_str().map(str::to_owned));
    assert_that!(suspects.into_iter().all(|s| LIBREOFFICE_LIMITS.contains(&s.as_str()))).is_true();
}
