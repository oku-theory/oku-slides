//! The third-party calibration corpus (`fixtures/import/corpus`, t-909): every deck is read,
//! projected in hi-fi and in retrofit, and the plan applies on an empty deck without any `Issue`.
//! These decks calibrate, they are never snapshot (fixtures rule): the tests assert only that the
//! import holds, and the measured reach of the witness's `DefaultFont` limit (t-958).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::collections::HashSet;

use oku_slides_core::{Deck, Namespace, apply_all, validate};
use oku_slides_import::retrofit::{ColorTokens, PageTokens, project};
use oku_slides_import::source::{Item, ItemKind, SourceDeck};
use oku_slides_import::{HifiTarget, PackLimits, RetrofitTarget, hifi, read_pptx};
use speculoos::prelude::*;
use uuid::Uuid;

/// The corpus decks, without their `.pptx` extension, in directory order.
const CORPUS: [&str; 19] = [
    "poi-SmartArt",
    "poi-smartart-simple",
    "poi-table-with-no-theme",
    "poi-table-with-theme",
    "poi-themes",
    "poi-with_japanese",
    "powertools-AnglesTheme",
    "powertools-ApexTheme",
    "powertools-WaveFormTheme",
    "pythonpptx-cht-chart-type",
    "pythonpptx-shp-groupshape",
    "pythonpptx-shp-picture",
    "sdk-Chart_2D",
    "sdk-Photo_Album3",
    "sdk-SmartArt_OrgChart1",
    "sdk-SmartArt_OrgChart1_3D",
    "sdk-Table_Large",
    "tika-testMSChart-govdocs-428996",
    "tika-testPPT_groups",
];

/// The corpus deck `name`, read with the default limits.
fn source(name: &str) -> SourceDeck {
    let bytes = common::file(&format!("corpus/{name}.pptx"));
    read_pptx(&bytes, &common::limits()).unwrap_or_else(|e| panic!("{name}: read: {e} (Git LFS pulled?)"))
}

/// Applies `ops` on an empty deck under `registry` and returns the `Issue`s of the result.
fn issues(
    name: &str,
    ops: Vec<oku_slides_core::Op>,
    registry: &oku_slides_core::WidgetRegistry,
    assets: &HashSet<String>,
) -> Vec<String> {
    let mut deck = Deck::empty();
    apply_all(&mut deck, ops, registry).unwrap_or_else(|e| panic!("{name}: the core refuses the plan: {e:?}"));
    validate(&deck, registry, Some(assets)).iter().map(|i| format!("{i:?}")).collect()
}

#[test]
fn corpus_list_matches_the_corpus_directory() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/import/corpus");
    let mut found: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap_or_else(|e| panic!("{e}")).path())
        .filter(|p| p.extension().is_some_and(|x| x == "pptx"))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect();
    found.sort();
    let mut listed: Vec<String> = CORPUS.iter().map(|s| (*s).to_owned()).collect();
    listed.sort();
    assert_that!(found).is_equal_to(listed);
}

/// The hi-fi import of corpus deck `name`: the `Issue`s of the deck it gives.
fn hifi_issues(name: &str) -> Vec<String> {
    let (base, fonts) = (common::registry(), common::app_fonts());
    let target = HifiTarget {
        registry: &base,
        base: common::base_pin(),
        essentiel: common::essentiel_pin(),
        fonts: &fonts,
        pack_id: Uuid::from_u128(1),
        theme: Namespace::parse("neutre").unwrap_or_else(|| panic!("namespace")),
        name: "Import".to_owned(),
        created: "2026-11-06T08:00:00Z".to_owned(),
        limits: PackLimits::default(),
    };
    let src = source(name);
    let plan = hifi::project(&src, &target).unwrap_or_else(|e| panic!("{name}: hi-fi: {e}"));
    let registry = common::registry_with_deck_layouts(&plan.schemas);
    let assets: HashSet<String> = plan.assets.keys().cloned().collect();
    assert_that!(plan.report.slides.len()).is_equal_to(src.slides.len());
    issues(name, common::with_theme_closure(plan.ops.clone()), &registry, &assets)
}

/// The retrofit of corpus deck `name` on `essentiel` and Papier: the `Issue`s of the deck it gives.
fn retrofit_issues(name: &str) -> Vec<String> {
    let ns = Namespace::parse("essentiel").unwrap_or_else(|| panic!("namespace"));
    let profiles: Vec<_> = common::profiles("essentiel").into_iter().map(|p| (ns.clone(), p)).collect();
    let registry = common::registry();
    let tokens = common::tokens("papier");
    let target = RetrofitTarget {
        profiles: &profiles,
        registry: &registry,
        colors: ColorTokens::from_tokens(&tokens),
        page: PageTokens::from_tokens(&tokens),
    };
    let src = source(name);
    let plan = project(&src, &target).unwrap_or_else(|e| panic!("{name}: retrofit: {e}"));
    let mut ops = common::builtin_pins();
    ops.extend(plan.ops.clone());
    assert_that!(plan.report.slides.len()).is_equal_to(src.slides.len());
    issues(name, ops, &registry, &plan.assets.keys().cloned().collect())
}

/// Each corpus deck whose import by `run` leaves an `Issue`, with them.
fn failing(run: fn(&str) -> Vec<String>) -> Vec<(&'static str, Vec<String>)> {
    CORPUS.iter().map(|name| (*name, run(name))).filter(|(_, found)| !found.is_empty()).collect()
}

#[test]
fn corpus_hifi_applies_every_deck_without_issue() {
    assert_that!(failing(hifi_issues)).is_empty();
}

#[test]
fn corpus_retrofit_applies_every_deck_without_issue() {
    assert_that!(failing(retrofit_issues)).is_empty();
}

/// The items, group children included, holding a written run whose font only the defaults name.
fn default_font_items(items: &[Item]) -> usize {
    items
        .iter()
        .map(|i| {
            if let ItemKind::Group { items, .. } = &i.kind {
                return default_font_items(items);
            }
            let runs = i.text().into_iter().flat_map(|t| &t.paragraphs).flat_map(|p| &p.runs);
            usize::from(runs.filter(|r| !r.text.trim().is_empty()).any(|r| r.style.font_from_defaults))
        })
        .sum()
}

// t-958, measured on 2026-11-06: every corpus master's `otherStyle` names `+mn-lt`, as PowerPoint
// writes it, so office2pdf finds the font of every ordinary text and `WitnessLimit::DefaultFont`
// marks nothing there. Only a master that names no font (kitchen, written by hand) reaches it.
#[test]
fn corpus_default_font_reaches_no_item() {
    let reached: Vec<(&str, usize)> = CORPUS
        .iter()
        .map(|name| (*name, source(name).slides.iter().map(|s| default_font_items(&s.items)).sum()))
        .filter(|(_, n)| *n > 0)
        .collect();
    assert_that!(reached).is_empty();
}

// Kitchen's "Gauche" and "Droite" (p. 2), and its SmartArt's two shapes (p. 3), which office2pdf's
// SmartArt limit marks first.
#[test]
fn kitchen_default_font_reaches_its_bare_boxes_and_smartart_shapes() {
    let src = read_pptx(&common::pptx("kitchen"), &common::limits()).unwrap_or_else(|e| panic!("kitchen: {e}"));
    let reached: usize = src.slides.iter().map(|s| default_font_items(&s.items)).sum();
    assert_that!(reached).is_equal_to(4);
}
