use oku_slides_chart::{Cell, ChartLabels, ChartOption};
use oku_slides_core::{Element, Op, ParamValue, Slide, SlotRef, Target, WidgetRegistry};
use oku_slides_i18n::Localizer;
use serde_json::json;
use speculoos::prelude::*;

use super::canonicalize_charts;

fn registry() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/builtin-registry.json");
    let text = std::fs::read_to_string(path).unwrap();
    WidgetRegistry::from_probe(serde_json::from_str(&text).unwrap()).unwrap()
}

fn inline_option() -> serde_json::Value {
    json!({
        "xAxis": { "type": "category", "data": ["1", "2"] },
        "series": [{ "type": "bar", "data": [3, 5] }]
    })
}

fn element(id: &str, kind: &str, param: &str) -> Element {
    serde_json::from_value(json!({ "id": id, "kind": kind, "params": { param: inline_option() } })).unwrap()
}

fn inline_chart(id: &str) -> Element {
    element(id, oku_slides_core::kinds::base::CHART, "option")
}

/// En-têtes du `dataset` du paramètre `option` ; vide sans dataset.
fn header(e: &Element) -> Vec<Cell> {
    let Some(ParamValue::Value(v)) = e.params.get("option") else { return Vec::new() };
    let option: ChartOption = serde_json::from_value(v.clone()).unwrap();
    option.dataset.map(|d| d.source[0].clone()).unwrap_or_default()
}

fn french() -> ChartLabels {
    ChartLabels::new(&Localizer::new(Some("fr")))
}

fn slot(slide: &str, param: &str) -> SlotRef {
    SlotRef { owner: Target { slide: slide.into(), element: None }, param: param.into() }
}

#[test]
fn canonicalize_charts_moves_inline_data_to_a_localized_dataset() {
    let mut ops =
        vec![Op::AddElement { slot: slot("s", "blocks"), index: None, defaults: false, element: inline_chart("g") }];
    canonicalize_charts(&mut ops, &french(), &registry());
    let Op::AddElement { element, .. } = &ops[0] else { panic!("add_element attendu") };
    assert_that!(header(element)).is_equal_to(vec![Cell::Text("Catégorie".into()), Cell::Text("Série 1".into())]);
}

#[test]
fn canonicalize_charts_reaches_slot_children_and_new_slides() {
    let stack: Element = serde_json::from_value(json!({
        "id": "grp", "kind": oku_slides_core::kinds::base::STACK, "params": { "children": [serde_json::to_value(inline_chart("g1")).unwrap()] }
    }))
    .unwrap();
    let mut slide = Slide::new("s2", oku_slides_core::kinds::essentiel::CONTENT);
    slide.params.set("blocks", ParamValue::Slot(vec![inline_chart("g2")]));
    let mut ops = vec![
        Op::ReplaceElement { slide: "s".into(), element: stack },
        Op::AddSlide { index: None, defaults: false, slide },
    ];
    canonicalize_charts(&mut ops, ChartLabels::english(), &registry());
    let Op::ReplaceElement { element, .. } = &ops[0] else { panic!("replace_element attendu") };
    let child = &element.params.slot("children").unwrap()[0];
    assert_that!(header(child)[0].clone()).is_equal_to(Cell::Text("Category".into()));
    let Op::AddSlide { slide, .. } = &ops[1] else { panic!("add_slide attendu") };
    assert_that!(header(&slide.params.slot("blocks").unwrap()[0]).len()).is_equal_to(2);
}

#[test]
fn canonicalize_charts_leaves_params_that_are_not_charts_alone() {
    // `option` n'est un graphique que pour un widget qui le déclare `host(chart)`.
    let mut ops = vec![
        Op::AddElement {
            slot: slot("s", "blocks"),
            index: None,
            defaults: false,
            element: element("t", oku_slides_core::kinds::base::TEXT, "option"),
        },
        Op::AddElement {
            slot: slot("s", "blocks"),
            index: None,
            defaults: false,
            element: element("u", "inconnu", "option"),
        },
    ];
    let before = ops.clone();
    canonicalize_charts(&mut ops, &french(), &registry());
    assert_that!(ops).is_equal_to(before);
}

#[test]
fn canonicalize_charts_leaves_other_ops_alone() {
    let mut ops = vec![Op::RemoveElement { slide: "s".into(), element: "g".into() }];
    let before = ops.clone();
    canonicalize_charts(&mut ops, &french(), &registry());
    assert_that!(ops).is_equal_to(before);
}
