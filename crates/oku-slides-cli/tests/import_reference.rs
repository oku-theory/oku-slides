//! `oku-slides import --mode hifi`, comparé au rendu de référence du témoin
//! (office2pdf, dans le processus de rendu) : un écart par slide au rapport,
//! sous un seuil par fixture, et des régions qui désignent les écarts connus
//! d'alto (t-863 : la coupure de la ligne de l'encadré p. 7 ; les puces p. 8,
//! à leur retrait depuis t-863, n'en sont plus) et signalent ces slides ; les
//! régions suspectes (erreurs connues d'office2pdf) passent en dernier et ne
//! signalent rien (t-884), comme les boîtes de kitchen p. 2 (t-904). Une
//! fixture décompressée (kitchen) est zippée à la volée. Polices de l'app
//! (`--fonts assets/fonts`) : celles que le témoin reçoit aussi. Le vrai LibreOffice ne tourne qu'avec
//! `OKU_SLIDES_TEST_SOFFICE=1` ; sinon `--reference soffice` a un faux `soffice`
//! (un script qui copie un PDF blanc), refusé sur macOS (t-912), dont les régions
//! suspectes sont celles des limites de LibreOffice ailleurs (t-905). Sans
//! `--reference`, le témoin est celui de `[import] witness`.

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
// Écart moyen de la pire slide, mesuré le 2026-10-14 (office2pdf 0.8.1, σ = 2), plus 25 % :
// alto 1,57, libreoffice 2,52 ; kitchen 2,7 (p. 2, le 2026-10-18).
#[case("alto-wealth.pptx", 2.0)]
#[case("libreoffice.pptx", 3.2)]
#[case("kitchen", 3.4)]
fn import_hifi_scores_every_slide_under_its_threshold(#[case] name: &str, #[case] threshold: f64) {
    let dir = workdir("import-reference");
    let (_, report) = import(dir.as_ref(), name, &[]);
    assert_that!(report["reference"]["witness"].as_str().unwrap_or_default()).starts_with("office2pdf ");
    assert_that!(max_score(&report)).is_less_than_or_equal_to(threshold);
}

// Pire région non suspecte mesurée le 2026-10-16 (office2pdf 0.8.1, seuil 7) : pp. 1 à 5 et 8 au
// plus 3,4 (les titres, ajustés par le témoin, sont suspects ; les puces p. 8 à leur retrait depuis
// t-863) ; p. 6 9,38 (la ligne d'introduction coupée ailleurs que dans PowerPoint), p. 7 15,71 (le
// filet de l'encadré, puis sa ligne coupée 12,81).
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

// Pire région non suspecte mesurée le 2026-10-18 (office2pdf 0.8.1, seuil 7) : p. 1 20,34 (le
// tableau), p. 2 3,96 et p. 3 5,69 (les titres). p. 2 valait 8,7 avant t-904 : « Gauche » et
// « Droite », dont seul le `defaultTextStyle` de la présentation nomme la police, qu'office2pdf
// dessine dans sa police de repli ; elles sont désormais suspectes (`default_font`).
#[test]
fn import_hifi_of_kitchen_flags_page_1_only() {
    let dir = workdir("import-reference-kitchen");
    let (_, report) = import(dir.as_ref(), "kitchen", &[]);
    assert_that!(flagged(&report)).is_equal_to(vec![1]);
}

#[test]
fn import_hifi_of_libreoffice_designates_its_table_on_page_1() {
    let dir = workdir("import-reference-table");
    let (_, report) = import(dir.as_ref(), "libreoffice.pptx", &[]);
    assert_that!(worst(&report, 1).first().map(String::as_str)).is_equal_to(Some("s1/8"));
}

// p. 3 : le SmartArt (10,41 et 10,33, au-delà du seuil) est suspect ; les autres régions valent
// au plus 5,8.
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
fn import_hifi_with_a_script_soffice_still_writes_the_deck_and_says_it_failed() {
    use oku_slides_reference::test_support::{blank_pdf, converting, fake_soffice};
    let dir = workdir("import-reference-soffice-script");
    let pdf = dir.as_ref().join("blanc.pdf");
    std::fs::write(&pdf, blank_pdf(3, 960, 540)).unwrap_or_else(|e| panic!("{e}"));
    soffice_config(dir.as_ref(), &fake_soffice(dir.as_ref(), &converting(&pdf, None)));
    let (_, report) = import(dir.as_ref(), "libreoffice.pptx", &["--reference", "soffice"]);
    assert_that!(report["reference"]["unavailable"]["type"].as_str()).is_equal_to(Some("failed"));
    assert_that!(dir.as_ref().join("out.oku-slides.pdf").exists()).is_true();
}

/// LibreOffice réel lancé ? (`OKU_SLIDES_TEST_SOFFICE=1`, il s'ouvre dans le Dock.)
fn real_soffice() -> bool {
    let on = std::env::var("OKU_SLIDES_TEST_SOFFICE").as_deref() == Ok("1");
    if !on {
        eprintln!("OKU_SLIDES_TEST_SOFFICE absent : test sauté");
    }
    on
}

/// LibreOffice réel, sous le bac à sable du processus de rendu (t-912) : `OKU_SLIDES_TEST_SOFFICE=1`.
#[test]
fn import_hifi_with_reference_soffice_scores_against_real_libreoffice() {
    if !real_soffice() {
        return;
    }
    let dir = workdir("import-reference-soffice-real");
    let (out, report) = import(dir.as_ref(), "libreoffice.pptx", &["--reference", "soffice"]);
    assert_that!(report["reference"]["unavailable"].is_null()).is_true();
    assert_that!(report["reference"]["witness"].as_str().unwrap_or_default()).starts_with("LibreOffice ");
    assert_that!(max_score(&report)).is_greater_than(0.0);
    let second = String::from_utf8_lossy(&out.stdout).lines().nth(1).unwrap_or_default().to_owned();
    assert_that!(second).starts_with("Visual gap to LibreOffice ");
}

// Mesuré le 2026-10-18 (LibreOffice 26.8.1.1, seuil 7), pire région non suspecte :
// - alto : pp. 1 à 5 et 7 au plus 6,77 (les titres interlettrés, à 1 px de largeur près) ; p. 6
//   9,16 (s6/17, à 10,24 pt : LibreOffice coupe « the agent » une ligne plus tôt qu'Oku et
//   office2pdf, qui partagent le façonnage de Typst : non tranché) ; p. 8 8,76 (les cartes des
//   piliers 2 et 3, dont il rétrécit les listes `normAutofit` qui débordent, PowerPoint non : la
//   source ne le dit pas, non marqué) ;
// - libreoffice et kitchen : p. 1 18,93 et 18,16 (le tableau, où Oku s'écarte des deux témoins) ;
//   p. 2 46,09 (le graphique : axes `delete` et sans légende, qu'Oku dessine) ; p. 3 au plus 0,1.
// Les titres en Calibri Light (14,7 à 29) et le SmartArt de libreoffice (21, dont le texte ne nomme
// ni couleur ni police) sont suspects ; celui de kitchen (1,68 et 3,45, que nomme le
// `defaultTextStyle`) ne l'est plus depuis t-936.
// Pire écart moyen d'une slide : alto 0,99, libreoffice et kitchen 3,05 ; plus 25 %.
#[rstest]
#[case("alto-wealth.pptx", 1.25, vec![6, 8])]
#[case("libreoffice.pptx", 3.8, vec![1, 2])]
#[case("kitchen", 3.8, vec![1, 2])]
fn import_hifi_against_real_libreoffice_flags_the_measured_slides(
    #[case] name: &str,
    #[case] threshold: f64,
    #[case] expected: Vec<usize>,
) {
    if !real_soffice() {
        return;
    }
    let dir = workdir("import-reference-soffice-real-flags");
    let (_, report) = import(dir.as_ref(), name, &["--reference", "soffice"]);
    assert_that!(max_score(&report)).is_less_than_or_equal_to(threshold);
    assert_that!(flagged(&report)).is_equal_to(expected);
    assert_that!(suspects(&report).iter().all(|s| LIBREOFFICE_LIMITS.contains(&s.as_str()))).is_true();
}

#[test]
fn import_hifi_against_real_libreoffice_sets_its_calibri_light_titles_aside() {
    if !real_soffice() {
        return;
    }
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
