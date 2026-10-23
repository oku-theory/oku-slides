//! Le hi-fi de bout en bout : les fixtures `minimal`, `kitchen`,
//! `libreoffice.pptx` et le vrai deck `alto-wealth.pptx`, projetés sur un
//! thème et un pack générés. Le plan s'applique en une transaction sur un
//! deck vide (registre : les packs intégrés plus les schémas déclarés),
//! épingle `base`, `essentiel` et le pack de layouts `deck-layouts` qu'il
//! porte, et le deck obtenu ne lève aucune `Issue`.

mod common;

use std::collections::{BTreeMap, HashSet};

use oku_slides_core::kinds;
use oku_slides_core::{
    Deck, Element, Namespace, Op, PackKind, ParamValue, Params, WidgetRegistry, apply_all, validate,
};
use oku_slides_import::source::SourceDeck;
use oku_slides_import::{HifiTarget, ImportPlan, PackLimits, hifi, read_pptx};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

fn source(fixture: &str) -> SourceDeck {
    let bytes = match fixture {
        "alto" => common::file("alto-wealth.pptx"),
        "libreoffice" => common::file("libreoffice.pptx"),
        name => common::pptx(name),
    };
    read_pptx(&bytes, &common::limits()).unwrap_or_else(|e| panic!("lecture de {fixture} : {e}"))
}

/// Le plan hi-fi de `fixture`, avec les polices de l'app.
fn plan(fixture: &str) -> ImportPlan {
    let (base, fonts) = (common::registry(), common::app_fonts());
    let target = HifiTarget {
        registry: &base,
        base: common::base_pin(),
        essentiel: common::essentiel_pin(),
        fonts: &fonts,
        pack_id: Uuid::from_u128(1),
        theme: Namespace::parse("neutre").unwrap_or_else(|| panic!("espace de noms")),
        name: "Import".to_owned(),
        created: "2026-10-11T08:00:00Z".to_owned(),
        limits: PackLimits::default(),
    };
    hifi::project(&source(fixture), &target).unwrap_or_else(|e| panic!("projection : {e}"))
}

/// Le registre du deck importé : les packs intégrés et les schémas déclarés du pack.
fn registry(plan: &ImportPlan) -> WidgetRegistry {
    common::registry_with_deck_layouts(&plan.schemas)
}

/// Le plan de `fixture` et le deck obtenu, contrôlé.
fn import(fixture: &str) -> (ImportPlan, Deck) {
    let plan = plan(fixture);
    let registry = registry(&plan);
    let mut deck = Deck::empty();
    apply_all(&mut deck, common::with_theme_closure(plan.ops.clone()), &registry)
        .unwrap_or_else(|e| panic!("{fixture} : {e:?}"));
    let assets: HashSet<String> = plan.assets.keys().cloned().collect();
    let issues = validate(&deck, &registry, Some(&assets));
    assert!(issues.is_empty(), "{fixture} : {issues:?}");
    (plan, deck)
}

#[rstest]
fn hifi_applies_without_issue_and_covers_everything(
    #[values("minimal", "kitchen", "libreoffice", "alto")] fixture: &str,
) {
    let (plan, deck) = import(fixture);
    assert_that!(deck.slides.len()).is_equal_to(source(fixture).slides.len());
    let pack = plan.pack.as_ref().unwrap();
    assert_that!(deck.theme.as_ref().map(Namespace::as_str)).is_equal_to(Some("neutre"));
    assert_that!(deck.layouts.get(&Namespace::deck_layouts()).map(|p| (p.id, p.version.clone())))
        .is_equal_to(Some((Uuid::from_u128(1), pack.version())));
    assert_that!(deck.tokens["weight"]["strong"].clone()).is_equal_to(json!(700));
    // Un fichier par widget déclaré, plus `_commun`.
    assert_that!((pack.kind(), pack.widgets().len() - 1)).is_equal_to((PackKind::Layouts, plan.schemas.len()));
    let c = plan.report.coverage.clone().unwrap_or_else(|| panic!("couverture mesurée"));
    assert_that!(c.missing).is_empty();
    assert_that!([c.runs, c.images, c.tables, c.charts].iter().all(|r| r.is_complete())).is_true();
}

#[test]
fn hifi_plan_pins_base_essentiel_then_deck_layouts() {
    let ops = plan("minimal").ops;
    let pins: Vec<(PackKind, &str)> = ops
        .iter()
        .filter_map(|o| match o {
            Op::AddPack { kind, namespace, .. } => Some((*kind, namespace.as_str())),
            _ => None,
        })
        .collect();
    assert_that!(pins).is_equal_to(vec![
        (PackKind::Widgets, "base"),
        (PackKind::Layouts, "essentiel"),
        (PackKind::Layouts, "deck-layouts"),
    ]);
}

#[test]
fn hifi_element_kinds_are_all_base() {
    let (_, deck) = import("kitchen");
    let kinds = common::deck_kinds(&deck);
    let stray: Vec<&String> = kinds.iter().filter(|k| !kinds::base::ALL.contains(&k.as_str())).collect();
    assert_that!((kinds.is_empty(), stray.is_empty())).is_equal_to((false, true));
}

#[test]
fn hifi_slide_kinds_are_deck_layouts_or_essentiel_blank() {
    let (_, deck) = import("kitchen");
    let stray: Vec<&str> = deck
        .slides
        .iter()
        .map(|s| s.kind.as_str())
        .filter(|k| !k.starts_with("deck-layouts/") && *k != kinds::essentiel::BLANK)
        .collect();
    assert_that!(stray).is_empty();
}

#[test]
fn hifi_twice_gives_the_same_plan() {
    assert_that!(plan("kitchen")).is_equal_to(plan("kitchen"));
}

#[test]
fn hifi_kitchen_coverage_counts_every_content() {
    let c = plan("kitchen").report.coverage.unwrap_or_else(|| panic!("couverture mesurée"));
    assert_that!((c.runs.total, c.images.total, c.tables.total, c.charts.total)).is_equal_to((12, 2, 1, 1));
}

#[test]
fn hifi_minimal_ops_match() {
    insta::assert_json_snapshot!("minimal_hifi_ops", plan("minimal").ops);
}

/// JSON compact.
fn compact(v: &impl serde::Serialize) -> String {
    serde_json::to_string(v).unwrap_or_else(|e| panic!("{e}"))
}

/// Les éléments `elements` du slot `slot`, une ligne chacun (id, sorte,
/// cadre, style, paramètres hors slots), puis ceux de leurs slots, en retrait.
fn element_lines(lines: &mut Vec<String>, depth: usize, slot: &str, elements: &[Element]) {
    let pad = "  ".repeat(depth);
    for e in elements {
        let style = compact(&e.style);
        lines.push(format!("{pad}{slot} {} {} {style} {}", e.id, describe(e), compact(&scalars(&e.params))));
        for (slot, children) in e.params.slots() {
            element_lines(lines, depth + 1, slot, children);
        }
    }
}

/// Les paramètres hors slots.
fn scalars(params: &Params) -> BTreeMap<&str, &ParamValue> {
    params.iter().filter(|(_, v)| !matches!(v, ParamValue::Slot(_))).collect()
}

/// Les ops d'un plan, une ligne par op et par élément : une slide et ses
/// paramètres hors slots, puis les éléments de ses slots ; toute autre op
/// en JSON compact.
fn op_lines(plan: &ImportPlan) -> String {
    let mut lines = Vec::new();
    for op in &plan.ops {
        let Op::AddSlide { slide, .. } = op else {
            lines.push(compact(op));
            continue;
        };
        lines.push(format!("slide {} {} {}", slide.id, slide.kind, compact(&scalars(&slide.params))));
        for (slot, elements) in slide.params.slots() {
            element_lines(&mut lines, 1, slot, elements);
        }
    }
    lines.join("\n")
}

/// D'un processus à l'autre (ordre d'une table de hachage, adresse…), le
/// plan d'un vrai deck ne change pas.
#[test]
fn hifi_alto_ops_match() {
    insta::assert_snapshot!("alto_hifi_ops", op_lines(&plan("alto")));
}

/// Un élément : sorte, cadre arrondi au pt.
fn describe(e: &Element) -> String {
    let place = e.frame.map(|f| format!(" @{:.0},{:.0} {:.0}×{:.0}", f.x, f.y, f.w, f.h)).unwrap_or_default();
    format!("{}{place}", e.kind)
}

/// Résumé : par slide, le widget, les paramètres remplis, les éléments, les
/// pertes par sorte ; puis la couverture, les polices et les assets.
fn summary(plan: &ImportPlan) -> Value {
    let slides: Vec<Value> = plan
        .ops
        .iter()
        .filter_map(|op| match op {
            Op::AddSlide { slide, .. } => Some(slide),
            _ => None,
        })
        .zip(&plan.report.slides)
        .map(|(slide, report)| {
            let params: Vec<&str> =
                slide.params.iter().filter(|(_, v)| !matches!(v, ParamValue::Slot(_))).map(|(k, _)| k).collect();
            let elements: Vec<String> = slide.params.slots().flat_map(|(_, list)| list).map(describe).collect();
            let mut losses: BTreeMap<String, usize> = BTreeMap::new();
            for l in &report.losses {
                let kind = serde_json::to_value(&l.kind).unwrap_or_default();
                *losses.entry(kind["type"].as_str().unwrap_or_default().to_owned()).or_default() += 1;
            }
            json!({ "slide": slide.id, "kind": slide.kind, "params": params, "elements": elements, "losses": losses })
        })
        .collect();
    let fonts: Vec<(&str, Option<&str>)> =
        plan.report.fonts.iter().map(|f| (f.family.as_str(), f.used.as_deref())).collect();
    json!({
        "slides": slides,
        "coverage": plan.report.coverage,
        "fonts": fonts,
        "embedded": plan.fonts.keys().collect::<Vec<_>>(),
        "assets": plan.assets.len(),
    })
}

#[rstest]
#[case("minimal")]
#[case("kitchen")]
#[case("libreoffice")]
#[case("alto")]
fn hifi_summary_matches(#[case] fixture: &str) {
    insta::assert_json_snapshot!(format!("{fixture}_hifi_summary"), summary(&plan(fixture)));
}
