//! `oku-slides import --mode hifi`, comparé au rendu de référence du témoin
//! (office2pdf, dans le processus de rendu) : un écart par slide au rapport,
//! sous un seuil par fixture, et des régions qui désignent les écarts connus
//! d'alto (t-863 : la coupure de la ligne de l'encadré p. 7 ; les puces p. 8,
//! à leur retrait depuis t-863, n'en sont plus) et signalent ces slides ; les
//! régions suspectes (erreurs connues d'office2pdf) passent en dernier et ne
//! signalent rien (t-884), comme les boîtes de kitchen p. 2 (t-904). Une
//! fixture décompressée (kitchen) est zippée à la volée. Polices de l'app
//! (`--fonts assets/fonts`) : celles que le témoin reçoit aussi. Le vrai LibreOffice ne tourne que dans
//! les tests ignorés, lancés par `just test-soffice` (t-960) ; sinon `--reference soffice` a un faux `soffice`
//! (un script qui copie un PDF blanc), refusé sur macOS (t-912), dont les régions
//! suspectes sont celles des limites de LibreOffice ailleurs (t-905). Sans
//! `--reference`, le témoin est celui de `[import] witness`.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::path::{Path, PathBuf};
use std::process::Output;

use common::{assert_success, fixtures, oku_slides, pptx, workdir};
use rstest::rstest;
use serde_json::Value;
use speculoos::prelude::*;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// La fixture `name` : un `.pptx`, ou un dossier décompressé, zippé dans `dir`.
fn input(dir: &Path, name: &str) -> PathBuf {
    let path = fixtures().join(name);
    if path.is_dir() { pptx(dir, name) } else { path }
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
        .arg(input(dir, name))
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

/// Les numéros (1..) des slides que le rapport signale (`above_threshold`).
fn flagged(report: &Value) -> Vec<usize> {
    let slides = report["slides"].as_array().cloned().unwrap_or_default();
    slides
        .iter()
        .enumerate()
        .filter(|(_, s)| s["above_threshold"].as_bool() == Some(true))
        .map(|(i, _)| i + 1)
        .collect()
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
// Mean gap of the worst slide, measured on 2026-11-01 (office2pdf 0.8.1, σ = 2; 0.10.0 scores the
// slides before t-1384 alike), plus 25 %: alto 1.57, libreoffice 1.23 and kitchen 1.74 (p. 2, the
// chart: 3.22 and 3.66 before t-1384 drew no gridlines and the source's bar width; office2pdf still
// draws its deleted axes, 46.07, so the chart region stays suspect).
#[case("alto-wealth.pptx", 2.0)]
#[case("libreoffice.pptx", 1.6)]
#[case("kitchen", 2.2)]
fn import_hifi_scores_every_slide_under_its_threshold(#[case] name: &str, #[case] threshold: f64) {
    let dir = workdir("import-reference");
    let (_, report) = import(dir.as_ref(), name, &[]);
    assert_that!(report["reference"]["witness"].as_str().unwrap_or_default()).starts_with("office2pdf ");
    assert_that!(max_score(&report)).is_less_than_or_equal_to(threshold);
}

// Worst non-suspect region measured on 2026-11-01 (office2pdf 0.10.0, threshold 7, as under 0.8.1):
// pp. 1 to 5 and 8 at most 3.4 (the titles, which the witness fits, are suspect; the p. 8 bullets at
// their indent since t-863); p. 6 9.38 (the intro line breaks elsewhere than in PowerPoint), p. 7
// 15.71 (the callout's rule, then its broken line 12.81).
#[test]
fn import_hifi_of_alto_flags_pages_6_and_7_only() {
    let dir = workdir("import-reference-flags");
    let (_, report) = import(dir.as_ref(), "alto-wealth.pptx", &[]);
    assert_that!(flagged(&report)).is_equal_to(vec![6, 7]);
}

#[test]
fn import_hifi_of_alto_designates_the_cut_line_of_the_callout_on_page_7() {
    let dir = workdir("import-reference-p7");
    let (_, report) = import(dir.as_ref(), "alto-wealth.pptx", &[]);
    // Le titre, que le témoin rétrécit à sa boîte, est suspect : il passe après.
    assert_that!(worst(&report, 7)).contains("s7/21".to_owned());
}

#[test]
fn import_hifi_of_alto_no_longer_designates_the_bullet_indents_on_page_8() {
    let dir = workdir("import-reference-p8");
    let (_, report) = import(dir.as_ref(), "alto-wealth.pptx", &[]);
    // Avant t-863, au moins deux des quatre listes (retrait de 27 pt ignoré) étaient désignées.
    // Les régions suspectes classées après les autres, une liste peut rester écrite : jamais
    // au-dessus du seuil.
    let lists = ["s8/8", "s8/12", "s8/16", "s8/21"];
    let threshold = report["reference"]["threshold"].as_f64().unwrap_or_else(|| panic!("seuil : {report}"));
    let regions = report["slides"][7]["score"]["regions"].as_array().cloned().unwrap_or_default();
    let above = regions
        .iter()
        .filter(|r| r["item"].as_str().is_some_and(|i| lists.contains(&i)))
        .filter(|r| r["score"].as_f64().is_some_and(|s| s > threshold))
        .count();
    assert_that!(above).is_equal_to(0);
}

// Worst non-suspect region measured on 2026-11-01 (office2pdf 0.10.0, threshold 7): p. 1 4.17 (the
// table: 6.89 under 0.8.1, which set its lines tighter; 20.34 before t-955 gave it its column
// widths, row heights and margins), p. 2 3.96 and
// p. 3 5.69 (the titles). p. 2 was 8.7 before t-904: "Gauche" and "Droite", whose font only the
// presentation's `defaultTextStyle` names, which office2pdf draws in its fallback font; they are
// suspect since (`default_font`).
#[test]
fn import_hifi_of_kitchen_flags_no_page() {
    let dir = workdir("import-reference-kitchen");
    let (_, report) = import(dir.as_ref(), "kitchen", &[]);
    assert_that!(flagged(&report)).is_empty();
}

// p. 1 4.24 (s1/8, the table) is the libreoffice deck's worst region (7.26 under office2pdf 0.8.1,
// which set its lines tighter than PowerPoint; 3.47 under LibreOffice).
#[test]
fn import_hifi_of_libreoffice_designates_its_table_on_page_1() {
    let dir = workdir("import-reference-table");
    let (_, report) = import(dir.as_ref(), "libreoffice.pptx", &[]);
    assert_that!(worst(&report, 1).first().map(String::as_str)).is_equal_to(Some("s1/8"));
}

// p. 3: the SmartArt (10.41, above the threshold) is suspect; the other regions are at most 5.8.
#[test]
fn import_hifi_of_libreoffice_ranks_its_smartart_last_without_flagging_page_3() {
    let dir = workdir("import-reference-smartart");
    let (_, report) = import(dir.as_ref(), "libreoffice.pptx", &[]);
    let regions = report["slides"][2]["score"]["regions"].as_array().cloned().unwrap_or_default();
    let last = regions.last().cloned().unwrap_or_default();
    assert_that!((last["suspect"].as_str(), report["slides"][2].get("above_threshold")))
        .is_equal_to((Some("smart_art"), None));
}

#[test]
fn import_hifi_says_the_visual_gap_on_a_second_line() {
    let dir = workdir("import-reference-line");
    let (out, _) = import(dir.as_ref(), "libreoffice.pptx", &[]);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert_that!(stdout.lines().nth(1).unwrap_or_default()).starts_with("Visual gap to office2pdf 0.10.0: ");
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

/// Écrit `config.toml` dans le dossier de données de `dir` : `[import] soffice = [program]`.
fn soffice_config(dir: &Path, program: &Path) {
    import_config(dir, &format!("soffice = [{:?}]\n", program.display().to_string()));
}

/// Écrit `config.toml` dans le dossier de données de `dir` : la section `[import]` `body`.
fn import_config(dir: &Path, body: &str) {
    let data = dir.join("data");
    std::fs::create_dir_all(&data).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(data.join("config.toml"), format!("[import]\n{body}")).unwrap_or_else(|e| panic!("{e}"));
}

/// Les limites des régions suspectes de toutes les slides du rapport.
fn suspects(report: &Value) -> Vec<String> {
    let slides = report["slides"].as_array().cloned().unwrap_or_default();
    slides
        .iter()
        .flat_map(|s| s["score"]["regions"].as_array().cloned().unwrap_or_default())
        .filter_map(|r| r["suspect"].as_str().map(str::to_owned))
        .collect()
}

/// Les limites de LibreOffice (t-905) : ses seules régions suspectes.
const LIBREOFFICE_LIMITS: [&str; 2] = ["no_substitute", "smart_art_style"];

#[test]
fn import_hifi_without_reference_follows_the_witness_setting() {
    let dir = workdir("import-reference-setting");
    import_config(dir.as_ref(), "witness = \"soffice\"\nsoffice = [\"/nonexistent/soffice\"]\n");
    let (_, report) = import(dir.as_ref(), "libreoffice.pptx", &[]);
    assert_that!(report["reference"]["unavailable"]["type"].as_str()).is_equal_to(Some("missing"));
    assert_that!(report["reference"]["witness"].as_str()).is_equal_to(Some("LibreOffice"));
}

#[test]
fn import_hifi_with_reference_office2pdf_overrides_the_witness_setting() {
    let dir = workdir("import-reference-setting-flag");
    import_config(dir.as_ref(), "witness = \"soffice\"\nsoffice = [\"/nonexistent/soffice\"]\n");
    let (_, report) = import(dir.as_ref(), "libreoffice.pptx", &["--reference", "office2pdf"]);
    assert_that!(report["reference"]["witness"].as_str().unwrap_or_default()).starts_with("office2pdf ");
    assert_that!(report["reference"]["unavailable"].is_null()).is_true();
}

#[cfg(target_os = "macos")]
#[test]
fn import_hifi_with_a_script_soffice_still_writes_the_deck_and_says_it_is_refused() {
    use oku_slides_reference::test_support::{blank_pdf, converting, fake_soffice};
    let dir = workdir("import-reference-soffice-script");
    let pdf = dir.as_ref().join("blanc.pdf");
    std::fs::write(&pdf, blank_pdf(3, 960, 540)).unwrap_or_else(|e| panic!("{e}"));
    soffice_config(dir.as_ref(), &fake_soffice(dir.as_ref(), &converting(&pdf, None)));
    let (_, report) = import(dir.as_ref(), "libreoffice.pptx", &["--reference", "soffice"]);
    assert_that!(report["reference"]["unavailable"]["type"].as_str()).is_equal_to(Some("refused"));
    assert_that!(dir.as_ref().join("out.oku-slides.pdf").exists()).is_true();
}

/// Real LibreOffice, under the render worker's sandbox (t-912).
#[test]
#[ignore = "real LibreOffice, it opens in the Dock: just test-soffice"]
fn import_hifi_with_reference_soffice_scores_against_real_libreoffice() {
    let dir = workdir("import-reference-soffice-real");
    let (out, report) = import(dir.as_ref(), "libreoffice.pptx", &["--reference", "soffice"]);
    assert_that!(report["reference"]["unavailable"].is_null()).is_true();
    assert_that!(report["reference"]["witness"].as_str().unwrap_or_default()).starts_with("LibreOffice ");
    assert_that!(max_score(&report)).is_greater_than(0.0);
    let second = String::from_utf8_lossy(&out.stdout).lines().nth(1).unwrap_or_default().to_owned();
    assert_that!(second).starts_with("Visual gap to LibreOffice ");
}

// Measured on 2026-10-31 (LibreOffice 26.8.1.1, threshold 7), worst non-suspect region:
// - alto: pp. 1 to 5 and 7 at most 6.77 (the spaced-out titles, within 1 px of width); p. 6 9.16
//   (s6/17, at 10.24 pt: LibreOffice breaks "the agent" one line earlier than Oku and office2pdf,
//   which share Typst's shaping: not settled); p. 8 8.76 (the cards of pillars 2 and 3, whose
//   overflowing `normAutofit` lists it shrinks, PowerPoint not: the source does not say, unmarked);
// - libreoffice and kitchen: p. 1 3.47 and 3.68 (the table, 18.9 before t-955); p. 2 19.9 (the
//   chart, 46.09 before t-1384 dropped the gridlines and took the source's bar width, 33.6 before
//   t-1493 gave it LibreOffice's automatic scale and 35.4 before t-1514 its 2 % plot inset: the
//   title stands 2 px higher and LibreOffice draws a hairline for T2, the category without values; the slide's
//   mean gap went 0.67, 0.58, 0.46 on LibreOffice 26.8.2.2); p. 3 at most 0.1.
// The Calibri Light titles (14.7 to 29) and libreoffice's SmartArt (21, whose text names neither
// color nor font) are suspect; kitchen's (1.68 and 3.45, named by the `defaultTextStyle`) no
// longer is since t-936.
// Worst mean gap of a slide: alto 0.99, libreoffice 1.24 and kitchen 1.28 (3.34 and 3.35 before
// t-1384); plus 25 %.
#[rstest]
#[case("alto-wealth.pptx", 1.25, vec![6, 8])]
#[case("libreoffice.pptx", 1.6, vec![2])]
#[case("kitchen", 1.6, vec![2])]
#[ignore = "real LibreOffice, it opens in the Dock: just test-soffice"]
fn import_hifi_against_real_libreoffice_flags_the_measured_slides(
    #[case] name: &str,
    #[case] threshold: f64,
    #[case] expected: Vec<usize>,
) {
    let dir = workdir("import-reference-soffice-real-flags");
    let (_, report) = import(dir.as_ref(), name, &["--reference", "soffice"]);
    assert_that!(max_score(&report)).is_less_than_or_equal_to(threshold);
    assert_that!(flagged(&report)).is_equal_to(expected);
    assert_that!(suspects(&report).iter().all(|s| LIBREOFFICE_LIMITS.contains(&s.as_str()))).is_true();
}

#[test]
#[ignore = "real LibreOffice, it opens in the Dock: just test-soffice"]
fn import_hifi_against_real_libreoffice_sets_its_calibri_light_titles_aside() {
    let dir = workdir("import-reference-soffice-real-titles");
    let (_, report) = import(dir.as_ref(), "kitchen", &["--reference", "soffice"]);
    // p. 1 : le titre (14,74) passe après le tableau. p. 3 n'écrit plus le sien : ses trois régions
    // non suspectes, dont le SmartArt (t-936), le précèdent.
    let regions = report["slides"][0]["score"]["regions"].as_array().cloned().unwrap_or_default();
    let title = regions.iter().find(|r| r["item"].as_str() == Some("s1/2")).cloned().unwrap_or_default();
    assert_that!(title["suspect"].as_str()).is_equal_to(Some("no_substitute"));
}

// Sous LibreOffice, seules ses limites (t-905) rendent une région suspecte : ni le graphique de
// p. 2 ni la police des défauts, suspects sous office2pdf. Hors macOS seulement : là, le faux
// `soffice` (un script) est refusé (t-912) ; les tests du vrai LibreOffice y vérifient la même chose.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn import_hifi_with_reference_soffice_marks_only_its_own_limits_suspect() {
    use oku_slides_reference::test_support::{blank_pdf, converting, fake_soffice};
    let dir = workdir("import-reference-soffice-suspect");
    let pdf = dir.as_ref().join("blanc.pdf");
    std::fs::write(&pdf, blank_pdf(3, 960, 540)).unwrap_or_else(|e| panic!("{e}"));
    soffice_config(dir.as_ref(), &fake_soffice(dir.as_ref(), &converting(&pdf, None)));
    let (_, report) = import(dir.as_ref(), "libreoffice.pptx", &["--reference", "soffice"]);
    let suspects = suspects(&report);
    assert_that!(suspects.is_empty()).is_false();
    assert_that!(suspects.iter().all(|s| LIBREOFFICE_LIMITS.contains(&s.as_str()))).is_true();
}

#[test]
fn import_hifi_with_reference_soffice_absent_still_writes_the_deck_and_says_why() {
    let dir = workdir("import-reference-soffice-absent");
    soffice_config(dir.as_ref(), Path::new("/nonexistent/soffice"));
    let (out, report) = import(dir.as_ref(), "libreoffice.pptx", &["--reference", "soffice"]);
    assert_that!(report["reference"]["unavailable"]["type"].as_str()).is_equal_to(Some("missing"));
    assert_that!(report["reference"]["witness"].as_str()).is_equal_to(Some("LibreOffice"));
    assert_that!(dir.as_ref().join("out.oku-slides.pdf").exists()).is_true();
    let second = String::from_utf8_lossy(&out.stdout).lines().nth(1).unwrap_or_default().to_owned();
    assert_that!(second).contains("LibreOffice (soffice) was not found");
}
