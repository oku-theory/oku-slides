use std::collections::BTreeMap;

use oku_slides_core::kinds::base;
use oku_slides_core::{DeckLoad, Op, apply_all, asset_id};
use oku_slides_data::limits::{MAX_DATA, MAX_DECK_OVERLAY, MAX_ROWS};
use speculoos::prelude::*;

use super::*;
use crate::ImportErrorKind;
use crate::report::{Loss, LossKind};
use crate::source::{self, ChartKind, Crop, Grouping};
use crate::source::{Asset, Picture, Size, SourceSlide, Warning, WarningKind};
use crate::test_support::{
    PAPIER, applied, builtin_pins, colors, essentiel_profiles, item, layout, para, registry, run, run_with,
    source_slide, text_box, text_item,
};

fn png() -> String {
    asset_id(b"png")
}

fn jpeg() -> String {
    asset_id(b"jpeg")
}

fn deck(slides: Vec<SourceSlide>) -> SourceDeck {
    SourceDeck {
        size: Size { w: 960.0, h: 540.0 },
        themes: Vec::new(),
        masters: Vec::new(),
        layouts: vec![layout("l1", "title"), layout("l2", "obj")],
        slides,
        assets: BTreeMap::from([
            (png(), Asset { mime: "image/png".into(), bytes: b"png".to_vec() }),
            (jpeg(), Asset { mime: "image/jpeg".into(), bytes: b"jpeg".to_vec() }),
        ]),
        fonts: Vec::new(),
        warnings: Vec::new(),
    }
}

fn target<'a>(profiles: &'a [(Namespace, LayoutProfile)], registry: &'a WidgetRegistry) -> RetrofitTarget<'a> {
    RetrofitTarget {
        profiles,
        registry,
        colors: colors(PAPIER),
        page: PageTokens::default(),
        labels: oku_slides_chart::ChartLabels::english(),
    }
}

fn plan_of(source: &SourceDeck, profiles: &[(Namespace, LayoutProfile)]) -> ImportPlan {
    let registry = registry();
    project(source, &target(profiles, &registry)).unwrap()
}

/// Un groupe de trois boîtes de texte en frise, à `(x, y)`, de largeur `w` par boîte.
fn frieze(id: &str, (x, y): (f64, f64), w: f64) -> Item {
    let children =
        (0..3).map(|i| text_item(&format!("{id}/{i}"), (x + f64::from(i) * 1.5 * w, y, w, w / 2.0), "Étape")).collect();
    item(id, (x, y, 3.5 * w, w / 2.0), ItemKind::Group { items: children, smartart: None })
}

#[test]
fn project_names_slides_by_number_and_keeps_only_referenced_assets() {
    let pic =
        item("p", (66.0, 143.0, 300.0, 200.0), ItemKind::Picture(Picture { asset: png(), crop: Crop::default() }));
    let source = deck(vec![source_slide(1, Some("l2"), vec![pic]), source_slide(4, Some("l2"), Vec::new())]);
    let plan = plan_of(&source, &essentiel_profiles());
    let ids: Vec<&str> = plan.report.slides.iter().map(|s| s.slide.as_str()).collect();
    assert_that!(ids).is_equal_to(vec!["s1", "s4"]);
    assert_that!(plan.assets.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec![png()]);
    applied(plan.ops, &[png()]);
}

#[test]
fn project_groups_widget_requests_by_signature_across_slides() {
    let source = deck(vec![
        source_slide(1, Some("l2"), vec![frieze("a", (60.0, 100.0), 100.0)]),
        source_slide(2, Some("l2"), vec![text_item("x", (60.0, 100.0, 400.0, 100.0), "Seul")]),
        source_slide(3, Some("l2"), vec![frieze("b", (200.0, 300.0), 50.0)]),
    ]);
    let plan = plan_of(&source, &essentiel_profiles());
    let requests: Vec<(&str, Vec<String>, usize)> =
        plan.widget_requests.iter().map(|r| (r.fallback.as_str(), r.slides.clone(), r.sample.len())).collect();
    assert_that!(requests).is_equal_to(vec![(base::STACK, vec!["s1".to_owned(), "s3".to_owned()], 3)]);
    assert_that!(plan.widget_requests[0].signature.as_str()).is_equal_to("text×3|0014,2034,3044");
    applied(plan.ops, &[]);
}

#[test]
fn project_copies_the_reader_warnings() {
    let mut source = deck(Vec::new());
    let warning = Warning { slide: Some(2), item: None, kind: WarningKind::UnsupportedChart("bubbleChart".into()) };
    source.warnings.push(warning.clone());
    assert_that!(plan_of(&source, &essentiel_profiles()).report.warnings).is_equal_to(vec![warning]);
}

#[test]
fn project_without_profiles_is_refused() {
    let source = deck(vec![source_slide(1, Some("l2"), vec![text_item("b", (66.0, 143.0, 828.0, 342.0), "Corps")])]);
    let registry = registry();
    let err = project(&source, &target(&[], &registry)).map(|_| ()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::NoProfiles);
}

#[test]
fn project_is_deterministic() {
    let source = deck(vec![source_slide(1, Some("l2"), vec![frieze("a", (60.0, 100.0), 100.0)])]);
    assert_that!(plan_of(&source, &essentiel_profiles())).is_equal_to(plan_of(&source, &essentiel_profiles()));
}

#[test]
fn project_measures_the_coverage_of_its_own_deck() {
    let pic =
        item("p", (500.0, 143.0, 300.0, 200.0), ItemKind::Picture(Picture { asset: png(), crop: Crop::default() }));
    let body = text_item("b", (66.0, 143.0, 300.0, 200.0), "Corps");
    let source = deck(vec![source_slide(1, Some("l2"), vec![body, pic])]);
    let c = plan_of(&source, &essentiel_profiles()).report.coverage.unwrap();
    assert_that!((c.runs.found, c.runs.total, c.images.found, c.images.total)).is_equal_to((1, 1, 1, 1));
}

#[test]
fn project_sets_the_deck_language_of_most_of_the_text() {
    let tb = text_box(vec![para(vec![run_with("مرحبا بكم جميعا", |s| s.lang = Some("ar-SA".into())), run("x")])]);
    let source =
        deck(vec![source_slide(1, Some("l2"), vec![item("t", (66.0, 143.0, 300.0, 200.0), ItemKind::Text(tb))])]);
    let plan = plan_of(&source, &essentiel_profiles());
    let lang = plan.ops.iter().find_map(|op| match op {
        Op::SetDeckParam { key, value } if key == "lang" => Some(value.clone()),
        _ => None,
    });
    assert_that!(lang).is_equal_to(Some(Value::String("ar-SA".into())));
    applied(plan.ops, &[]);
}

#[test]
fn project_without_language_sets_none() {
    let source = deck(vec![source_slide(1, Some("l2"), vec![text_item("t", (66.0, 143.0, 300.0, 200.0), "Texte")])]);
    let plan = plan_of(&source, &essentiel_profiles());
    assert_that!(plan.ops.iter().any(|op| matches!(op, Op::SetDeckParam { key, .. } if key == "lang"))).is_false();
}

/// A slide `number` holding one column chart `c<number>` of two series of `points` values.
fn chart_slide(number: u32, points: usize) -> SourceSlide {
    let values: Vec<Option<f64>> = (0..points).map(|i| Some(oku_slides_core::num::count_f64(i))).collect();
    let series = |name: &str| source::Series {
        name: name.to_owned(),
        values: values.clone(),
        color: None,
        x: None,
        format: None,
    };
    let chart = source::Chart {
        kind: ChartKind::Column,
        grouping: Grouping::Clustered,
        title: Some(format!("Ventes {number}")),
        categories: Vec::new(),
        series: vec![series("a"), series("b")],
        legend: true,
        axes: source::Axes { x: true, y: true },
        value_axis: source::ValueAxis::default(),
        gap_width: None,
    };
    let c = item(&format!("c{number}"), (66.0, 143.0, 828.0, 342.0), ItemKind::Chart(chart));
    source_slide(number, Some("l2"), vec![c])
}

/// The deck of `plan`, applied as the session applies it.
fn deck_of(plan: &ImportPlan) -> oku_slides_core::Deck {
    let mut deck = oku_slides_core::Deck::empty();
    let mut ops = builtin_pins();
    ops.extend(plan.ops.iter().cloned());
    apply_all(&mut deck, ops, &registry()).unwrap();
    deck
}

/// The `ChartTruncated` losses of each slide, by item.
fn truncations(plan: &ImportPlan) -> Vec<(String, Vec<Loss>)> {
    plan.report
        .slides
        .iter()
        .map(|s| (s.slide.clone(), s.losses.iter().filter(|l| l.kind == LossKind::ChartTruncated).cloned().collect()))
        .collect()
}

/// Two charts of 196,608 cells each pass the deck's 300,000 overlay cells: the first is kept
/// whole, the second cut to what is left, and reported on its item.
#[test]
fn import_over_deck_budget_cuts_next_table_and_reports() {
    let source = deck(vec![chart_slide(1, MAX_ROWS), chart_slide(2, MAX_ROWS)]);
    let plan = plan_of(&source, &essentiel_profiles());
    let deck = deck_of(&plan);
    let rows: Vec<usize> = deck.data.values().map(|d| d.table().unwrap().rows.len()).collect();
    assert_that!(rows.len()).is_equal_to(2);
    assert_that!(rows.contains(&MAX_ROWS)).is_true();
    let cut = rows.iter().copied().min().unwrap();
    assert_that!(cut).is_greater_than(0);
    assert_that!(cut).is_less_than(MAX_ROWS);
    let load = DeckLoad::of_deck(&deck);
    assert_that!(load.overlay).is_less_than_or_equal_to(MAX_DECK_OVERLAY);
    // One more row would not have fit.
    assert_that!(load.overlay + 3).is_greater_than(MAX_DECK_OVERLAY);
    assert_that!(truncations(&plan)).is_equal_to(vec![
        ("s1".to_owned(), Vec::new()),
        ("s2".to_owned(), vec![Loss::on("c2", LossKind::ChartTruncated)]),
    ]);
}

/// Past `MAX_DATA` data, a chart comes in without data, reported on its item.
#[test]
fn import_past_max_data_imports_element_without_data() {
    let last = u32::try_from(MAX_DATA).unwrap() + 1;
    let source = deck((1..=last).map(|n| chart_slide(n, 2)).collect());
    let plan = plan_of(&source, &essentiel_profiles());
    let deck = applied(plan.ops.clone(), &[]);
    assert_that!(deck.data.len()).is_equal_to(MAX_DATA);
    let charts: Vec<&oku_slides_core::Element> = deck
        .slides
        .last()
        .unwrap()
        .params
        .slots()
        .flat_map(|(_, children)| children)
        .filter(|e| e.kind == base::CHART)
        .collect();
    assert_that!(charts.len()).is_equal_to(1);
    assert_that!(charts[0].params.value("option")).is_none();
    let reported = truncations(&plan);
    assert_that!(reported[..MAX_DATA].iter().all(|(_, l)| l.is_empty())).is_true();
    assert_that!(reported[MAX_DATA].1).is_equal_to(vec![Loss::on(&format!("c{last}"), LossKind::ChartTruncated)]);
}
