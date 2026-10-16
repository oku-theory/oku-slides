//! Un deck de retrofit rendu (t-816) : les blocs d'une slide en flux restent
//! dans sa zone de corps, au-dessus de la marge basse, sinon le plan dit le
//! débordement (`Overflow`). La présentation passe par l'import réel
//! (`oku-slides-import`, dépendance de test) : le projecteur estime la
//! hauteur du flux sans rendu, ce test la confronte au rendu Typst.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use oku_slides_core::tokens::effective_tokens;
use oku_slides_core::{Deck, Template, apply_all};
use oku_slides_import::report::LossKind;
use oku_slides_import::retrofit::{ColorTokens, PageTokens, project};
use oku_slides_import::source::SourceDeck;
use oku_slides_import::{ReadLimits, RetrofitTarget, read_pptx};
use oku_slides_render::{Fonts, Renderer, load_registry};
use rstest::rstest;
use serde_json::{Map, Value};
use speculoos::prelude::*;

mod common;

/// Marge tolérée sous la zone, en pt (arrondis du rendu).
const TOLERANCE: f64 = 2.0;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let p = e.unwrap_or_else(|e| panic!("{e}")).path();
        if p.file_name().is_some_and(|n| n == ".DS_Store") {
            continue;
        }
        if p.is_dir() { files(&p, out) } else { out.push(p) }
    }
}

/// Les octets de la présentation `fixture` : `alto` (fichier) ou un dossier OOXML zippé ici.
fn pptx(fixture: &str) -> Vec<u8> {
    let import = root().join("fixtures/import");
    if fixture == "alto" {
        return fs::read(import.join("alto-wealth.pptx")).unwrap_or_else(|e| panic!("{e}"));
    }
    let dir = import.join(fixture);
    let mut paths = Vec::new();
    files(&dir, &mut paths);
    paths.sort_by_key(|p| (p.file_name() != Some("[Content_Types].xml".as_ref()), p.clone()));
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for p in paths {
        let rel = p.strip_prefix(&dir).unwrap_or_else(|e| panic!("{e}")).to_string_lossy().replace('\\', "/");
        w.start_file(rel, zip::write::SimpleFileOptions::default()).unwrap_or_else(|e| panic!("{e}"));
        w.write_all(&fs::read(&p).unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("{e}"));
    }
    w.finish().unwrap_or_else(|e| panic!("{e}")).into_inner()
}

fn templates(pack: &str) -> Vec<Template> {
    let text = fs::read_to_string(root().join(format!("templates/{pack}/pack.json"))).unwrap_or_else(|e| panic!("{e}"));
    let pack: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}"));
    serde_json::from_value(pack["templates"].clone()).unwrap_or_else(|e| panic!("{e}"))
}

/// Les jetons effectifs du thème intégré `theme` : les défauts, puis les siens.
fn tokens(theme: &str) -> Map<String, Value> {
    let path = root().join("typst/oku-slides/src/default-tokens.json");
    let defaults: Value = serde_json::from_str(&fs::read_to_string(path).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"));
    let own = common::theme_tokens(Some(theme)).unwrap_or_else(|| panic!("thème {theme}"));
    effective_tokens(defaults.as_object().unwrap_or_else(|| panic!("jetons par défaut")), &own)
}

#[rstest]
fn import_retrofit_flow_blocks_stay_in_the_body_zone_or_are_reported(
    #[values("alto", "kitchen")] fixture: &str,
    #[values("papier", "pro")] theme: &str,
) {
    let source: SourceDeck = read_pptx(&pptx(fixture), &ReadLimits::default()).unwrap_or_else(|e| panic!("{e}"));
    let fonts = Fonts::default();
    let registry = load_registry(&fonts, None).unwrap_or_else(|e| panic!("{e}"));
    let (templates, tokens) = (templates("essentiel"), tokens(theme));
    let page = PageTokens::from_tokens(&tokens);
    let target =
        RetrofitTarget { templates: &templates, registry: &registry, colors: ColorTokens::from_tokens(&tokens), page };
    let plan = project(&source, &target);
    let mut deck = Deck::empty();
    apply_all(&mut deck, plan.ops.clone(), &registry).unwrap_or_else(|e| panic!("{e:?}"));
    let assets: BTreeMap<String, Vec<u8>> = plan.assets.iter().map(|(id, a)| (id.clone(), a.bytes.clone())).collect();
    let mut r = Renderer::new(fonts).unwrap_or_else(|e| panic!("{e}"));
    r.set_theme(common::theme_tokens(Some(theme))).unwrap_or_else(|e| panic!("{e}"));
    let compiled = r.render(&deck, &assets).unwrap_or_else(|e| panic!("rendu : {e}"));
    let layout = compiled.layout();
    let bottom = page.height - page.margin_bottom + TOLERANCE;
    let mut checked = 0;
    for (slide, report) in deck.slides.iter().zip(&plan.report.slides) {
        let reported = report.losses.iter().any(|l| matches!(l.kind, LossKind::Overflow { .. }));
        if slide.params.slot("blocks").is_none_or(<[_]>::is_empty) || reported {
            continue;
        }
        checked += 1;
        let page_no = layout.page_of(&slide.id).unwrap_or_else(|| panic!("{} sans page", slide.id));
        for b in layout.boxes_on(page_no) {
            asserting(&format!("{fixture}/{theme} {} : bas de {} à {:.0}", slide.id, b.id, b.y + b.h))
                .that(&(b.y + b.h <= bottom))
                .is_true();
        }
    }
    assert_that!(checked).is_greater_than(0);
}
