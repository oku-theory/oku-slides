use oku_slides_import::report::{Mode, Placement, SlideReport};
use oku_slides_render::{FieldBox, SlidePage};
use serde_json::json;
use speculoos::prelude::*;

use super::*;

fn field(owner: &str, page: usize, (x, y, w, h): (f64, f64, f64, f64)) -> FieldBox {
    FieldBox {
        owner: owner.into(),
        field: "body".into(),
        page,
        x,
        y,
        w,
        h,
        ax: None,
        aw: None,
        style: None,
        empty: false,
        side: None,
    }
}

/// Deux slides (pages 1 et 2) sur une page de 1920 × 1080.
fn layout(fields: Vec<FieldBox>) -> Layout {
    Layout {
        slides: vec![SlidePage { slide: "s1".into(), page: 1 }, SlidePage { slide: "s2".into(), page: 2 }],
        fields,
        tokens: Some(json!({"page": {"width": 1920, "height": 1080}})),
        ..Layout::default()
    }
}

fn report() -> ImportReport {
    let slide = |n: u32| SlideReport {
        source: n,
        slide: format!("s{n}"),
        template: "t".into(),
        placement: Placement::Flow,
        losses: vec![],
        fallbacks: vec![],
        score: None,
    };
    ImportReport { slides: vec![slide(1), slide(2)], ..ImportReport::new(Mode::Retrofit) }
}

fn overflows(r: &ImportReport) -> Vec<(u32, String)> {
    r.slides
        .iter()
        .flat_map(|s| s.losses.iter().map(move |l| (s.source, l)))
        .filter_map(|(n, l)| match &l.kind {
            LossKind::Overflow { element } => Some((n, element.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn note_overflows_reports_a_field_below_the_page_on_its_slide() {
    let mut r = report();
    note_overflows(&layout(vec![field("s2-1", 2, (100.0, 900.0, 800.0, 300.0))]), &mut r);
    assert_that!(overflows(&r)).is_equal_to(vec![(2, "s2-1/body".to_owned())]);
}

#[test]
fn note_overflows_ignores_a_field_inside_the_page() {
    let mut r = report();
    note_overflows(&layout(vec![field("s1-1", 1, (0.0, 0.0, 1920.0, 1080.5))]), &mut r);
    assert_that!(overflows(&r)).is_empty();
}

#[test]
fn note_overflows_ignores_an_empty_field() {
    let mut r = report();
    let empty = FieldBox { empty: true, ..field("s1-1", 1, (0.0, 1000.0, 10.0, 200.0)) };
    note_overflows(&layout(vec![empty]), &mut r);
    assert_that!(overflows(&r)).is_empty();
}

#[test]
fn note_overflows_reports_a_field_once() {
    let mut r = report();
    let f = field("s1-1", 1, (1800.0, 0.0, 300.0, 10.0));
    note_overflows(&layout(vec![f.clone(), f]), &mut r);
    assert_that!(overflows(&r).len()).is_equal_to(1);
}

#[test]
fn note_overflows_reads_the_page_size_from_the_tokens() {
    let mut r = report();
    let small = Layout { tokens: Some(json!({"page": {"width": 960, "height": 540}})), ..layout(vec![]) };
    note_overflows(&Layout { fields: vec![field("s1-1", 1, (0.0, 500.0, 100.0, 100.0))], ..small }, &mut r);
    assert_that!(overflows(&r).len()).is_equal_to(1);
}
