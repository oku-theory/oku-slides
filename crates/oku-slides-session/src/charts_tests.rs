use oku_slides_chart::{Cell, ChartLabels};
use oku_slides_core::{ElementBody, Op, Slide};
use oku_slides_i18n::Localizer;
use serde_json::json;
use speculoos::prelude::*;

use super::canonicalize_charts;

fn inline_chart(id: &str) -> serde_json::Value {
    json!({ "id": id, "kind": "chart", "option": {
        "xAxis": { "type": "category", "data": ["1", "2"] },
        "series": [{ "type": "bar", "data": [3, 5] }]
    } })
}

fn header(e: &oku_slides_core::Element) -> Vec<Cell> {
    match &e.body {
        ElementBody::Chart { option } => option.dataset.as_ref().map(|d| d.source[0].clone()).unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn french() -> ChartLabels {
    ChartLabels::new(&Localizer::new(Some("fr")))
}

#[test]
fn canonicalize_charts_moves_inline_data_to_a_localized_dataset() {
    let element = serde_json::from_value(inline_chart("g")).unwrap();
    let mut ops = vec![Op::AddElement {
        slide: "s".into(),
        zone: oku_slides_core::Zone::Blocks,
        parent: None,
        index: None,
        element,
    }];
    canonicalize_charts(&mut ops, &french());
    let Op::AddElement { element, .. } = &ops[0] else { panic!("add_element attendu") };
    assert_that!(header(element)).is_equal_to(vec![Cell::Text("Catégorie".into()), Cell::Text("Série 1".into())]);
}

#[test]
fn canonicalize_charts_reaches_group_children_and_new_slides() {
    let group =
        serde_json::from_value(json!({ "id": "grp", "kind": "group", "children": [inline_chart("g1")] })).unwrap();
    let mut slide = Slide::new("s2", oku_slides_core::SlideKind::Content);
    slide.blocks.push(serde_json::from_value(inline_chart("g2")).unwrap());
    let mut ops = vec![Op::ReplaceElement { slide: "s".into(), element: group }, Op::AddSlide { index: None, slide }];
    canonicalize_charts(&mut ops, ChartLabels::english());
    let Op::ReplaceElement { element, .. } = &ops[0] else { panic!("replace_element attendu") };
    assert_that!(header(&element.children()[0])[0].clone()).is_equal_to(Cell::Text("Category".into()));
    let Op::AddSlide { slide, .. } = &ops[1] else { panic!("add_slide attendu") };
    assert_that!(header(&slide.blocks[0]).len()).is_equal_to(2);
}

#[test]
fn canonicalize_charts_leaves_other_ops_alone() {
    let mut ops = vec![Op::RemoveElement { slide: "s".into(), element: "g".into() }];
    let before = ops.clone();
    canonicalize_charts(&mut ops, &french());
    assert_that!(ops).is_equal_to(before);
}
