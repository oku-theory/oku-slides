use std::collections::BTreeMap;

use oku_slides_core::{Op, asset_id};
use speculoos::prelude::*;

use super::*;
use crate::source::{Asset, Picture, Size, SourceSlide, Warning, WarningKind};
use crate::test_support::{
    PAPIER, applied, colors, essentiel, item, layout, para, registry, run, run_with, source_slide, text_box, text_item,
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

fn plan_of(source: &SourceDeck, templates: &[Template]) -> ImportPlan {
    let registry = registry();
    let target = RetrofitTarget { templates, registry: &registry, colors: colors(PAPIER), page: PageTokens::default() };
    project(source, &target)
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
        item("p", (66.0, 143.0, 300.0, 200.0), ItemKind::Picture(Picture { asset: png(), crop: Default::default() }));
    let source = deck(vec![source_slide(1, Some("l2"), vec![pic]), source_slide(4, Some("l2"), Vec::new())]);
    let plan = plan_of(&source, &essentiel());
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
    let plan = plan_of(&source, &essentiel());
    let requests: Vec<(&str, Vec<String>, usize)> =
        plan.widget_requests.iter().map(|r| (r.fallback.as_str(), r.slides.clone(), r.sample.len())).collect();
    assert_that!(requests).is_equal_to(vec![("stack", vec!["s1".to_owned(), "s3".to_owned()], 3)]);
    assert_that!(plan.widget_requests[0].signature.as_str()).is_equal_to("text×3|0014,2034,3044");
    applied(plan.ops, &[]);
}

#[test]
fn project_copies_the_reader_warnings() {
    let mut source = deck(Vec::new());
    let warning = Warning { slide: Some(2), item: None, kind: WarningKind::UnsupportedChart("bubbleChart".into()) };
    source.warnings.push(warning.clone());
    assert_that!(plan_of(&source, &essentiel()).report.warnings).is_equal_to(vec![warning]);
}

#[test]
fn project_with_an_empty_pack_makes_bare_content_slides() {
    let source = deck(vec![source_slide(1, Some("l2"), vec![text_item("b", (66.0, 143.0, 828.0, 342.0), "Corps")])]);
    let plan = plan_of(&source, &[]);
    let Some(Op::AddSlide { slide, .. }) = plan.ops.first() else { panic!("AddSlide attendu") };
    assert_that!((slide.kind.as_str(), plan.report.slides[0].template.as_str())).is_equal_to(("content", "content"));
    applied(plan.ops, &[]);
}

#[test]
fn project_is_deterministic() {
    let source = deck(vec![source_slide(1, Some("l2"), vec![frieze("a", (60.0, 100.0), 100.0)])]);
    assert_that!(plan_of(&source, &essentiel())).is_equal_to(plan_of(&source, &essentiel()));
}

#[test]
fn project_measures_the_coverage_of_its_own_deck() {
    let pic =
        item("p", (500.0, 143.0, 300.0, 200.0), ItemKind::Picture(Picture { asset: png(), crop: Default::default() }));
    let body = text_item("b", (66.0, 143.0, 300.0, 200.0), "Corps");
    let source = deck(vec![source_slide(1, Some("l2"), vec![body, pic])]);
    let c = plan_of(&source, &essentiel()).report.coverage.unwrap();
    assert_that!((c.runs.found, c.runs.total, c.images.found, c.images.total)).is_equal_to((1, 1, 1, 1));
}

#[test]
fn project_sets_the_deck_language_of_most_of_the_text() {
    let tb = text_box(vec![para(vec![run_with("مرحبا بكم جميعا", |s| s.lang = Some("ar-SA".into())), run("x")])]);
    let source =
        deck(vec![source_slide(1, Some("l2"), vec![item("t", (66.0, 143.0, 300.0, 200.0), ItemKind::Text(tb))])]);
    let plan = plan_of(&source, &essentiel());
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
    let plan = plan_of(&source, &essentiel());
    assert_that!(plan.ops.iter().any(|op| matches!(op, Op::SetDeckParam { key, .. } if key == "lang"))).is_false();
}
