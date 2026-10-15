//! `oku-slides import`, lancée pour de vrai : une présentation PowerPoint en
//! `.oku-slides.pdf` rouvrable. Chaque test a son dossier de données
//! (OKU_SLIDES_PATHS__DATA, par `oku_slides`) ; aucun ne lance LibreOffice.

mod common;

use std::collections::HashSet;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

use common::{assert_success, oku_slides, workdir};
use oku_slides_core::{SourceBundle, SourceLimits, WidgetRegistry, validate};
use oku_slides_session::{DEFAULT_PACK, GRID_PACK, PRO_THEME};
use rstest::rstest;
use speculoos::prelude::*;
use zip::write::SimpleFileOptions;

const LIMITS: SourceLimits = SourceLimits { replica_bytes: 64 << 20, total_bytes: 512 << 20, compact_after_secs: 0 };

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/import")
}

fn run(cmd: &mut Command) -> Output {
    cmd.output().unwrap_or_else(|e| panic!("oku-slides ne se lance pas : {e}"))
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let p = e.unwrap_or_else(|e| panic!("{e}")).path();
        if p.file_name().is_some_and(|n| n == ".DS_Store") {
            continue;
        }
        if p.is_dir() { files(&p, out) } else { out.push(p) }
    }
}

/// Le `.pptx` de la fixture décompressée `name`, zippé dans `dir` ; son chemin.
fn pptx(dir: &Path, name: &str) -> PathBuf {
    pptx_with(dir, name, &[])
}

/// Comme [`pptx`], les parts de `replaced` (chemin dans le zip, contenu) remplaçant celles de la fixture.
fn pptx_with(dir: &Path, name: &str, replaced: &[(&str, &[u8])]) -> PathBuf {
    let root = fixtures().join(name);
    let mut paths = Vec::new();
    files(&root, &mut paths);
    paths.sort_by_key(|p| (p.file_name() != Some("[Content_Types].xml".as_ref()), p.clone()));
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for p in paths {
        let rel = p.strip_prefix(&root).unwrap_or_else(|e| panic!("{e}")).to_string_lossy().replace('\\', "/");
        let data = match replaced.iter().find(|(part, _)| *part == rel) {
            Some((_, bytes)) => bytes.to_vec(),
            None => std::fs::read(&p).unwrap_or_else(|e| panic!("{e}")),
        };
        w.start_file(rel, SimpleFileOptions::default()).unwrap_or_else(|e| panic!("zip : {e}"));
        w.write_all(&data).unwrap_or_else(|e| panic!("zip : {e}"));
    }
    let bytes = w.finish().unwrap_or_else(|e| panic!("zip : {e}")).into_inner();
    let path = dir.join(format!("{name}.pptx"));
    std::fs::write(&path, bytes).unwrap_or_else(|e| panic!("{e}"));
    path
}

fn base_registry() -> Arc<WidgetRegistry> {
    let probe = serde_json::from_str(include_str!("../../../fixtures/widgets/base-registry.json"))
        .unwrap_or_else(|e| panic!("registre de base : {e}"));
    Arc::new(WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("registre de base : {e}")))
}

/// Les sources d'un `.oku-slides.pdf`, relues ; échoue si le deck lève une `Issue`.
fn reopened(pdf: &Path) -> SourceBundle {
    let bytes = std::fs::read(pdf).unwrap_or_else(|e| panic!("lecture du PDF : {e}"));
    let found = oku_slides_pdf::extract_sources(&bytes)
        .unwrap_or_else(|e| panic!("sources du PDF : {e}"))
        .unwrap_or_else(|| panic!("PDF sans sources"));
    let (bundle, issues) =
        SourceBundle::from_zip(&found.zip, LIMITS, base_registry()).unwrap_or_else(|e| panic!("sources : {e}"));
    assert_that!(issues).is_empty();
    let assets: HashSet<String> = bundle.assets.keys().cloned().collect();
    assert_that!(validate(bundle.deck(), &base_registry(), Some(&assets))).is_empty();
    bundle
}

#[test]
fn import_minimal_on_essentiel_and_pro_writes_a_reopenable_deck() {
    let dir = workdir("import-minimal");
    let (input, pdf) = (pptx(&dir, "minimal"), dir.join("minimal.oku-slides.pdf"));
    let out = run(oku_slides(&dir, "en")
        .arg("import")
        .arg(&input)
        .args(["--pack", "Essentiel", "--theme", "Pro", "-o"])
        .arg(&pdf));
    assert_success("import", &out, &[]);
    // Une perte : le second niveau de puce de la slide 2, aplati.
    assert_that!(stdout(&out))
        .is_equal_to(format!("{}: 3 slides, 3 pages, 1 loss, 0 widget requests\n", pdf.display()));
    let bundle = reopened(&pdf);
    let deck = bundle.deck();
    assert_that!((deck.slides.len(), deck.params.title.as_deref())).is_equal_to((3, Some("Rapport annuel")));
    assert_that!(deck.templates.as_ref().map(|r| r.id)).is_equal_to(Some(DEFAULT_PACK));
    assert_that!(deck.theme.as_ref().map(|r| r.id)).is_equal_to(Some(PRO_THEME));
    assert_that!(deck.slides[1].notes.starts_with("Dire bonjour.")).is_true();
}

/// `pptx` avec la part `name` remplacée par `content`.
fn replaced(pptx: &Path, name: &str, content: &str) {
    let bytes = std::fs::read(pptx).unwrap_or_else(|e| panic!("{e}"));
    let mut src = zip::ZipArchive::new(Cursor::new(bytes)).unwrap_or_else(|e| panic!("zip : {e}"));
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..src.len() {
        let entry = src.by_index_raw(i).unwrap_or_else(|e| panic!("zip : {e}"));
        if entry.name() == name {
            continue;
        }
        w.raw_copy_file(entry).unwrap_or_else(|e| panic!("zip : {e}"));
    }
    w.start_file(name, SimpleFileOptions::default()).unwrap_or_else(|e| panic!("zip : {e}"));
    w.write_all(content.as_bytes()).unwrap_or_else(|e| panic!("zip : {e}"));
    let out = w.finish().unwrap_or_else(|e| panic!("zip : {e}")).into_inner();
    std::fs::write(pptx, out).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn import_retrofit_of_a_text_too_long_for_its_place_reports_an_overflow() {
    let dir = workdir("import-overflow");
    let input = pptx(&dir, "minimal");
    let slide =
        std::fs::read_to_string(fixtures().join("minimal/ppt/slides/slide3.xml")).unwrap_or_else(|e| panic!("{e}"));
    let paragraphs: String = (0..80)
        .map(|i| format!(r#"<a:p><a:r><a:rPr lang="fr-FR"/><a:t>Ligne {i} d'un texte trop long</a:t></a:r></a:p>"#))
        .collect();
    let long =
        slide.replace(r#"<a:p><a:r><a:rPr lang="fr-FR"/><a:t>Texte sans couleur</a:t></a:r></a:p>"#, &paragraphs);
    assert_that!(long.len()).is_greater_than(slide.len());
    replaced(&input, "ppt/slides/slide3.xml", &long);
    let report = dir.join("r.json");
    let out = run(oku_slides(&dir, "en")
        .arg("import")
        .arg(&input)
        .args(["--pack", "essentiel", "-o"])
        .arg(dir.join("o.oku-slides.pdf"))
        .arg("--report")
        .arg(&report));
    assert_success("import", &out, &[]);
    let r: serde_json::Value = serde_json::from_slice(&std::fs::read(&report).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("rapport : {e}"));
    let kinds: Vec<&str> =
        r["slides"][2]["losses"].as_array().into_iter().flatten().filter_map(|l| l["kind"]["type"].as_str()).collect();
    assert_that!(kinds).contains("overflow");
}

#[test]
fn import_kitchen_on_grilles_writes_its_report_with_coverage_and_messages() {
    let dir = workdir("import-kitchen");
    let (input, pdf, report) = (pptx(&dir, "kitchen"), dir.join("k.oku-slides.pdf"), dir.join("k.json"));
    let out = run(oku_slides(&dir, "fr")
        .arg("import")
        .arg(&input)
        .args(["--mode", "retrofit", "--pack", "grilles", "-o"])
        .arg(&pdf)
        .arg("--report")
        .arg(&report));
    assert_success("import", &out, &[]);
    let bundle = reopened(&pdf);
    assert_that!(bundle.deck().templates.as_ref().map(|r| r.id)).is_equal_to(Some(GRID_PACK));
    assert_that!(bundle.assets.len()).is_equal_to(1);
    let r: serde_json::Value = serde_json::from_slice(&std::fs::read(&report).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("rapport : {e}"));
    assert_that!(r["mode"]).is_equal_to(serde_json::json!("retrofit"));
    assert_that!(r["coverage"]["runs"]["found"]).is_equal_to(r["coverage"]["runs"]["total"].clone());
    assert_that!(r["coverage"]["missing"]).is_equal_to(serde_json::json!([]));
    assert_that!(r["warnings"][0]["message"])
        .is_equal_to(serde_json::json!("Graphique non pris en charge : combo: barChart"));
    assert_that!(r["widget_requests"].as_array().map(Vec::len)).is_equal_to(Some(1));
}

#[test]
fn import_alto_with_pack_and_theme_ids_keeps_its_images() {
    let dir = workdir("import-alto");
    let pdf = dir.join("alto.oku-slides.pdf");
    let out = run(oku_slides(&dir, "en")
        .arg("import")
        .arg(fixtures().join("alto-wealth.pptx"))
        .args(["--pack", &DEFAULT_PACK.to_string(), "--theme", &PRO_THEME.to_string(), "-o"])
        .arg(&pdf));
    assert_success("import", &out, &[]);
    let bundle = reopened(&pdf);
    assert_that!(bundle.deck().slides.len()).is_equal_to(8);
    assert_that!(bundle.assets.is_empty()).is_false();
}

#[test]
fn import_with_a_pack_folder_uses_that_pack() {
    let dir = workdir("import-folder");
    let pack = dir.join("mon-pack");
    std::fs::create_dir_all(&pack).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(pack.join("pack.json"), include_str!("../../../templates/grilles/pack.json"))
        .unwrap_or_else(|e| panic!("{e}"));
    let (input, pdf) = (pptx(&dir, "minimal"), dir.join("m.oku-slides.pdf"));
    let out = run(oku_slides(&dir, "en").arg("import").arg(&input).arg("--pack").arg(&pack).arg("-o").arg(&pdf));
    assert_success("import", &out, &[]);
    assert_that!(reopened(&pdf).deck().templates.as_ref().map(|r| r.id)).is_equal_to(Some(GRID_PACK));
}

/// Les sources d'un `.oku-slides.pdf`, relues avec la seule base : les
/// widgets du pack embarqué n'y sont pas contrôlés (`oku-slides open` le fait).
fn sources(pdf: &Path) -> SourceBundle {
    let bytes = std::fs::read(pdf).unwrap_or_else(|e| panic!("lecture du PDF : {e}"));
    let found = oku_slides_pdf::extract_sources(&bytes)
        .unwrap_or_else(|e| panic!("sources du PDF : {e}"))
        .unwrap_or_else(|| panic!("PDF sans sources"));
    SourceBundle::from_zip(&found.zip, LIMITS, base_registry()).unwrap_or_else(|e| panic!("sources : {e}")).0
}

/// Importe `input` en hi-fi (sans les polices du système : le résultat ne
/// dépend pas de la machine) vers `<dir>/<stem>.oku-slides.pdf`, rapport en
/// `<dir>/<stem>.json` ; la sortie de la commande.
fn import_hifi(dir: &Path, input: &Path, stem: &str) -> Output {
    run(oku_slides(dir, "en")
        .arg("import")
        .arg(input)
        // L'écart visuel a ses propres tests (`import_reference.rs`) : ceux-ci s'en passent.
        .args(["--mode", "hifi", "--no-system-fonts", "--reference", "none", "-o"])
        .arg(dir.join(format!("{stem}.oku-slides.pdf")))
        .arg("--report")
        .arg(dir.join(format!("{stem}.json"))))
}

fn report(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("rapport : {e}"))
}

/// Les fichiers de la bibliothèque de l'utilisateur (packs et thèmes) sous `dir/data`.
fn library_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for sub in ["templates", "themes"] {
        let at = dir.join("data").join(sub);
        if at.exists() {
            files(&at, &mut found);
        }
    }
    found
}

/// Le deck rouvert par `oku-slides open` (pack sondé, deck validé), extrait dans `<dir>/open`.
fn open_hifi(dir: &Path, pdf: &Path) -> Output {
    let out = run(oku_slides(dir, "en").arg("open").arg(pdf).arg("--out").arg(dir.join("open")));
    assert_success("open", &out, &[]);
    assert_that!(stderr(&out)).does_not_contain("issue:");
    out
}

#[test]
fn import_hifi_of_minimal_embeds_its_own_pack_and_theme_and_leaves_the_library_alone() {
    let dir = workdir("import-hifi-minimal");
    let input = pptx(&dir, "minimal");
    let out = import_hifi(&dir, &input, "minimal");
    assert_success("import", &out, &[]);
    let pdf = dir.join("minimal.oku-slides.pdf");
    assert_that!(stdout(&out)).starts_with(format!("{}: 3 slides, 3 pages, ", pdf.display()));

    let bundle = sources(&pdf);
    let deck = bundle.deck();
    assert_that!(deck.slides.len()).is_equal_to(3);
    let pack = deck.templates.clone().unwrap_or_else(|| panic!("deck sans pack"));
    let theme = deck.theme.clone().unwrap_or_else(|| panic!("deck sans thème"));
    assert_that!(bundle.packs.keys().collect::<Vec<_>>()).is_equal_to(vec![&pack.version]);
    assert_that!(bundle.themes.keys().collect::<Vec<_>>()).is_equal_to(vec![&theme.version]);
    assert_that!(pack.id).is_not_equal_to(DEFAULT_PACK);
    assert_that!(theme.id).is_not_equal_to(PRO_THEME);

    let opened = open_hifi(&dir, &pdf);
    assert_that!(stdout(&opened)).contains(": 3 slides, ");
    assert_that!(dir.join("open/packs").join(pack.version.hex()).join("pack.json").exists()).is_true();
    assert_that!(library_files(&dir)).is_empty();

    let r = report(&dir.join("minimal.json"));
    assert_that!(r["mode"]).is_equal_to(serde_json::json!("hifi"));
    assert_that!(r["coverage"]["runs"]["found"]).is_equal_to(r["coverage"]["runs"]["total"].clone());
    let fonts = r["fonts"].as_array().unwrap_or_else(|| panic!("rapport sans polices : {r}"));
    assert_that!(fonts.is_empty()).is_false();
}

#[test]
fn import_hifi_of_alto_writes_a_reopenable_deck_with_its_images_and_report() {
    let dir = workdir("import-hifi-alto");
    let out = import_hifi(&dir, &fixtures().join("alto-wealth.pptx"), "alto");
    assert_success("import", &out, &[]);
    let pdf = dir.join("alto.oku-slides.pdf");
    assert_that!(stdout(&out)).starts_with(format!("{}: 8 slides, 8 pages, ", pdf.display()));
    let bundle = sources(&pdf);
    assert_that!((bundle.deck().slides.len(), bundle.packs.len(), bundle.themes.len())).is_equal_to((8, 1, 1));
    assert_that!(bundle.assets.is_empty()).is_false();
    assert_that!(stdout(&open_hifi(&dir, &pdf))).contains(": 8 slides, ");
    assert_that!(library_files(&dir)).is_empty();
    let r = report(&dir.join("alto.json"));
    assert_that!(r["coverage"]["missing"]).is_equal_to(serde_json::json!([]));
    assert_that!(r["fonts"].as_array().map(Vec::is_empty)).is_equal_to(Some(false));
}

#[test]
fn import_hifi_twice_of_the_same_file_generates_the_same_pack_and_theme_ids() {
    let dir = workdir("import-hifi-twice");
    let input = pptx(&dir, "minimal");
    let ids = |stem: &str| {
        assert_success("import", &import_hifi(&dir, &input, stem), &[]);
        let bundle = sources(&dir.join(format!("{stem}.oku-slides.pdf")));
        let deck = bundle.deck();
        (deck.templates.as_ref().map(|r| r.id), deck.theme.as_ref().map(|r| r.id))
    };
    let first = ids("a");
    assert_that!(first.0.is_some() && first.1.is_some()).is_true();
    assert_that!(ids("b")).is_equal_to(first);
}

#[rstest]
#[case("--pack", "Essentiel")]
#[case("--theme", "Pro")]
fn import_hifi_with_a_target_is_refused_with_a_localized_message(#[case] arg: &str, #[case] value: &str) {
    let dir = workdir("import-hifi-target");
    let out = run(oku_slides(&dir, "en")
        .arg("import")
        .arg(fixtures().join("alto-wealth.pptx"))
        .args(["--mode", "hifi", arg, value, "-o"])
        .arg(dir.join("x.oku-slides.pdf")));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("--pack and --theme belong to retrofit mode");
    assert_that!(dir.join("x.oku-slides.pdf").exists()).is_false();
}

#[test]
fn import_without_pack_is_refused_in_retrofit_mode() {
    let dir = workdir("import-nopack");
    let out = run(oku_slides(&dir, "fr")
        .arg("import")
        .arg(fixtures().join("alto-wealth.pptx"))
        .arg("-o")
        .arg(dir.join("x.pdf")));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("--pack est obligatoire en mode retrofit");
}

#[test]
fn import_with_an_unknown_pack_fails_with_its_name() {
    let dir = workdir("import-unknown");
    let out = run(oku_slides(&dir, "en")
        .arg("import")
        .arg(fixtures().join("alto-wealth.pptx"))
        .args(["--pack", "Nope", "-o"])
        .arg(dir.join("x.pdf")));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("unknown pack: Nope");
}

#[test]
fn import_with_a_pack_folder_whose_manifest_is_not_a_pack_fails_with_its_name() {
    let dir = workdir("import-badpack");
    let pack = dir.join("faux-pack");
    std::fs::create_dir_all(&pack).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(pack.join("pack.json"), r#"{"templates": "aucun"}"#).unwrap_or_else(|e| panic!("{e}"));
    let out = run(oku_slides(&dir, "fr")
        .arg("import")
        .arg(fixtures().join("alto-wealth.pptx"))
        .arg("--pack")
        .arg(&pack)
        .arg("-o")
        .arg(dir.join("x.pdf")));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("pack illisible");
}

/// Un fichier creux de 257 Mio (au-delà des 256 Mio de `ReadLimits::default()`) :
/// refusé sur sa taille, sans être lu.
#[test]
fn import_of_a_file_beyond_the_input_limit_is_refused_before_reading() {
    let dir = workdir("import-toolarge");
    let input = dir.join("huge.pptx");
    let file = std::fs::File::create(&input).unwrap_or_else(|e| panic!("{e}"));
    file.set_len(257 << 20).unwrap_or_else(|e| panic!("{e}"));
    let out = run(oku_slides(&dir, "en")
        .arg("import")
        .arg(&input)
        .args(["--pack", "Essentiel", "-o"])
        .arg(dir.join("x.pdf")));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("This presentation is too large to import");
}

#[test]
fn import_of_a_file_that_is_not_a_zip_fails_with_the_reader_message() {
    let dir = workdir("import-notpptx");
    let input = dir.join("deck.pptx");
    std::fs::write(&input, b"not a zip").unwrap_or_else(|e| panic!("{e}"));
    let out = run(oku_slides(&dir, "en")
        .arg("import")
        .arg(&input)
        .args(["--pack", "Essentiel", "-o"])
        .arg(dir.join("x.pdf")));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("This presentation could not be read");
}

#[test]
fn import_help_is_in_the_configured_language() {
    let dir = workdir("import-help");
    let out = run(oku_slides(&dir, "fr").args(["import", "--help"]));
    assert_that!(stdout(&out)).contains("Importe une présentation PowerPoint (.pptx)");
}

/// `minimal` dont la slide 1 porte `boxes` zones d'un même texte répété, zippé dans `dir` ; son chemin.
fn minimal_inflating(dir: &Path, boxes: u32) -> PathBuf {
    let text = "Oku ".repeat(2_000);
    let shapes: String = (100..100 + boxes)
        .map(|id| {
            format!(
                r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="Texte {id}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1270000" cy="1270000"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr><p:txBody><a:bodyPr/><a:p><a:r><a:t>{text}</a:t></a:r></a:p></p:txBody></p:sp>"#
            )
        })
        .collect();
    let slide = std::fs::read_to_string(fixtures().join("minimal/ppt/slides/slide1.xml"))
        .unwrap_or_else(|e| panic!("{e}"))
        .replace("</p:spTree>", &format!("{shapes}</p:spTree>"));
    pptx_with(dir, "minimal", &[("ppt/slides/slide1.xml", slide.as_bytes())])
}

/// Message (français) d'un import refusé parce que le deck produit serait trop gros.
const OUTPUT_TOO_LARGE_FR: &str = "Le deck obtenu dépasserait la taille admise pour un import";

/// Un petit zip dont le deck dépasserait `[sources] max_sources_mib` (t-753) : refusé avant toute écriture.
#[test]
fn import_hifi_whose_output_exceeds_max_sources_is_refused() {
    let dir = workdir("import-output-budget");
    let input = minimal_inflating(&dir, 160);
    let output = dir.join("x.pdf");
    let out = run(oku_slides(&dir, "fr")
        .env("OKU_SLIDES_SOURCES__MAX_SOURCES_MIB", "1")
        .arg("import")
        .arg(&input)
        .args(["--mode", "hifi", "--no-system-fonts", "-o"])
        .arg(&output));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains(OUTPUT_TOO_LARGE_FR);
    assert_that!(output.exists()).is_false();
}

/// En hi-fi, des ops plus grosses que `[sources] max_replica_mib` (t-753) : refusées avant toute écriture.
#[test]
fn import_hifi_whose_ops_exceed_max_replica_is_refused() {
    let dir = workdir("import-replica-budget");
    let input = minimal_inflating(&dir, 160);
    let output = dir.join("x.pdf");
    let out = run(oku_slides(&dir, "fr")
        .env("OKU_SLIDES_SOURCES__MAX_REPLICA_MIB", "1")
        .arg("import")
        .arg(&input)
        .args(["--mode", "hifi", "--no-system-fonts", "-o"])
        .arg(&output));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains(OUTPUT_TOO_LARGE_FR);
    assert_that!(output.exists()).is_false();
}

/// En retrofit, une image qui se décompresse au-delà de `[sources] max_sources_mib` (t-753) : refusée
/// avant toute écriture (le retrofit coupe les textes, une image seule peut faire déborder le deck).
#[test]
fn import_retrofit_whose_output_exceeds_max_sources_is_refused() {
    let dir = workdir("import-retrofit-budget");
    // Signature et `IHDR` d'une image 1 × 1 : un raster lisible n'est refusé que par le budget.
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR\0\0\0\x01\0\0\0\x01\x08\x06\0\0\0".to_vec();
    png.resize(2 << 20, 0);
    let input = pptx_with(&dir, "kitchen", &[("ppt/media/image1.png", &png)]);
    let output = dir.join("x.pdf");
    let out = run(oku_slides(&dir, "fr")
        .env("OKU_SLIDES_SOURCES__MAX_SOURCES_MIB", "1")
        .arg("import")
        .arg(&input)
        .args(["--pack", "grilles", "-o"])
        .arg(&output));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains(OUTPUT_TOO_LARGE_FR);
    assert_that!(output.exists()).is_false();
}
