//! `oku-slides import --mode hifi`, comparé au rendu de référence du témoin
//! (office2pdf, dans le processus de rendu) : un écart par slide au rapport,
//! sous un seuil par fixture, et des régions qui désignent les écarts connus
//! d'alto (t-863 : la coupure de la ligne de l'encadré p. 7, le retrait des
//! puces p. 8). Polices de l'app (`--fonts assets/fonts`) : celles que le
//! témoin reçoit aussi. Aucun test ne lance LibreOffice.

mod common;

use std::path::{Path, PathBuf};
use std::process::Output;

use common::{assert_success, oku_slides, workdir};
use rstest::rstest;
use serde_json::Value;
use speculoos::prelude::*;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Importe la fixture `name` en hi-fi, avec `extra` ; la sortie et le rapport.
fn import(dir: &Path, name: &str, extra: &[&str]) -> (Output, Value) {
    import_with(dir, name, extra, &[])
}

/// [`import`], avec les variables d'environnement `env`.
fn import_with(dir: &Path, name: &str, extra: &[&str], env: &[(&str, &str)]) -> (Output, Value) {
    let report = dir.join("r.json");
    let out = oku_slides(dir, "en")
        .envs(env.iter().copied())
        .arg("--fonts")
        .arg(root().join("assets/fonts"))
        .arg("import")
        .arg(root().join("fixtures/import").join(name))
        .args(["--mode", "hifi", "--no-system-fonts", "-o"])
        .arg(dir.join("out.oku-slides.pdf"))
        .arg("--report")
        .arg(&report)
        .args(extra)
        .output()
        .unwrap_or_else(|e| panic!("oku-slides ne se lance pas : {e}"));
    assert_success("import", &out, &[]);
    let report = serde_json::from_slice(&std::fs::read(&report).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("rapport : {e}"));
    (out, report)
}

/// Les ids des régions désignées de la slide `n` (1..), les pires d'abord.
fn worst(report: &Value, n: usize) -> Vec<String> {
    let regions = report["slides"][n - 1]["score"]["regions"].as_array().cloned().unwrap_or_default();
    regions.iter().filter_map(|r| r["item"].as_str().map(str::to_owned)).collect()
}

/// Le plus grand écart moyen des slides du rapport ; échoue si une slide n'en a pas.
fn max_score(report: &Value) -> f64 {
    let slides = report["slides"].as_array().unwrap_or_else(|| panic!("rapport sans slides : {report}"));
    slides
        .iter()
        .map(|s| s["score"]["score"].as_f64().unwrap_or_else(|| panic!("slide sans écart : {s}")))
        .fold(0.0, f64::max)
}

#[rstest]
// Écart moyen de la pire slide, mesuré le 2026-10-14 (office2pdf 0.8.1, σ = 2), plus 25 % :
// alto 1,57, libreoffice 2,52.
#[case("alto-wealth.pptx", 2.0)]
#[case("libreoffice.pptx", 3.2)]
fn import_hifi_scores_every_slide_under_its_threshold(#[case] name: &str, #[case] threshold: f64) {
    let dir = workdir("import-reference");
    let (_, report) = import(dir.as_ref(), name, &[]);
    assert_that!(report["reference"]["witness"].as_str()).is_equal_to(Some("office2pdf 0.8.1"));
    assert_that!(max_score(&report)).is_less_than_or_equal_to(threshold);
}

#[test]
fn import_hifi_of_alto_designates_the_cut_line_of_the_callout_on_page_7() {
    let dir = workdir("import-reference-p7");
    let (_, report) = import(dir.as_ref(), "alto-wealth.pptx", &[]);
    // Le titre et le filet de l'encadré le précèdent : le témoin dessine les titres d'alto
    // plus larges (limite connue).
    assert_that!(worst(&report, 7)).contains("s7/21".to_owned());
}

#[test]
fn import_hifi_of_alto_designates_the_bullet_indents_on_page_8() {
    let dir = workdir("import-reference-p8");
    let (_, report) = import(dir.as_ref(), "alto-wealth.pptx", &[]);
    let lists = ["s8/8", "s8/12", "s8/16", "s8/21"];
    let found = worst(&report, 8).iter().filter(|i| lists.contains(&i.as_str())).count();
    assert_that!(found).is_greater_than_or_equal_to(2);
}

#[test]
fn import_hifi_of_libreoffice_designates_its_table_on_page_1() {
    let dir = workdir("import-reference-table");
    let (_, report) = import(dir.as_ref(), "libreoffice.pptx", &[]);
    assert_that!(worst(&report, 1).first().map(String::as_str)).is_equal_to(Some("s1/8"));
}

#[test]
fn import_hifi_says_the_visual_gap_on_a_second_line() {
    let dir = workdir("import-reference-line");
    let (out, _) = import(dir.as_ref(), "libreoffice.pptx", &[]);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert_that!(stdout.lines().nth(1).unwrap_or_default()).starts_with("Visual gap to office2pdf 0.8.1: ");
}

#[test]
fn import_hifi_with_reference_none_writes_no_score() {
    let dir = workdir("import-reference-none");
    let (out, report) = import(dir.as_ref(), "libreoffice.pptx", &["--reference", "none"]);
    assert_that!(report.get("reference")).is_none();
    assert_that!(report["slides"][0].get("score")).is_none();
    assert_that!(String::from_utf8_lossy(&out.stdout).lines().count()).is_equal_to(1);
}

#[test]
fn import_retrofit_with_reference_is_refused_in_french() {
    let dir = workdir("import-reference-retrofit");
    let out = oku_slides(dir.as_ref(), "fr")
        .arg("import")
        .arg(root().join("fixtures/import/libreoffice.pptx"))
        .args(["--pack", "essentiel", "--reference", "auto", "-o"])
        .arg(dir.as_ref().join("out.oku-slides.pdf"))
        .output()
        .unwrap_or_else(|e| panic!("oku-slides ne se lance pas : {e}"));
    assert_that!(out.status.success()).is_false();
    assert_that!(String::from_utf8_lossy(&out.stderr).into_owned()).contains("--reference appartient au mode hifi");
}

#[test]
fn import_hifi_whose_reference_times_out_still_writes_the_deck_without_scores() {
    let dir = workdir("import-reference-timeout");
    let (out, report) =
        import_with(dir.as_ref(), "libreoffice.pptx", &[], &[("OKU_SLIDES_IMPORT__REFERENCE_TIMEOUT_MS", "1")]);
    assert_that!(report["reference"]["unavailable"]["type"].as_str()).is_equal_to(Some("timeout"));
    let scored = report["slides"].as_array().into_iter().flatten().filter(|s| s.get("score").is_some()).count();
    assert_that!(scored).is_equal_to(0);
    let second = String::from_utf8_lossy(&out.stdout).lines().nth(1).unwrap_or_default().to_owned();
    assert_that!(second.starts_with("Visual gap")).is_false();
}

#[test]
fn import_hifi_with_a_negative_threshold_is_refused_in_french() {
    let dir = workdir("import-reference-threshold");
    let out = oku_slides(dir.as_ref(), "fr")
        .env("OKU_SLIDES_IMPORT__SCORE_THRESHOLD", "-1")
        .arg("import")
        .arg(root().join("fixtures/import/libreoffice.pptx"))
        .args(["--mode", "hifi", "-o"])
        .arg(dir.as_ref().join("out.oku-slides.pdf"))
        .output()
        .unwrap_or_else(|e| panic!("oku-slides ne se lance pas : {e}"));
    assert_that!(out.status.success()).is_false();
    assert_that!(String::from_utf8_lossy(&out.stderr).into_owned())
        .contains("doit être un nombre supérieur ou égal à zéro");
}
