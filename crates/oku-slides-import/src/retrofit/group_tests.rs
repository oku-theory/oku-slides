use std::collections::BTreeMap;

use oku_slides_core::{Element, Paragraph};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::report::LossKind;
use crate::retrofit::content::ContentCx;
use crate::source::{self, Fill, Geometry, Rgba, Styled};
use crate::test_support::{PAPIER, colors, item, para, run, text_box, text_item};

/// `items` repliés sur Papier, sous l'id `e1`.
fn fold(items: &[Item], smartart: bool) -> (Option<Element>, Vec<Loss>, Option<String>) {
    let (tokens, assets) = (colors(PAPIER), BTreeMap::new());
    let cx = ContentCx { tokens: &tokens, assets: &assets, track: (0.0, 0.0) };
    let mut losses = Vec::new();
    let (e, fallback) = group(items, smartart, "e1", &cx, &mut losses);
    (e, losses, fallback)
}

fn accent1() -> source::Color {
    Styled::mapped(Rgba::new(0x44, 0x72, 0xc4), "accent1", "accent1", Vec::new())
}

fn bare(id: &str, preset: &str, frame: (f64, f64, f64, f64)) -> Item {
    let s = source::Shape {
        geom: Geometry::Preset { name: preset.into(), adj: BTreeMap::new() },
        fill: Fill::Solid { color: accent1() },
        stroke: None,
        text: None,
        connection: None,
    };
    item(id, frame, ItemKind::Shape(Box::new(s)))
}

fn body_texts(e: &Element) -> Vec<String> {
    let ps: &[Paragraph] = e.params.rich("body").unwrap_or_default();
    ps.iter().map(|p| p.runs.iter().map(|r| r.t.as_str()).collect()).collect()
}

/// Trois boîtes côte à côte (une étape, un titre et sa description chacune), une flèche,
/// la troisième étape dans un sous-groupe.
fn steps() -> Vec<Item> {
    let step = |i: usize| {
        let tb = text_box(vec![para(vec![run(&format!("Étape {i}"))]), para(vec![run(&format!("Détail {i}"))])]);
        let s = source::Shape {
            geom: Geometry::Preset { name: "roundRect".into(), adj: BTreeMap::new() },
            fill: Fill::Solid { color: accent1() },
            stroke: None,
            text: Some(tb),
            connection: None,
        };
        item(&format!("g/{i}"), (i as f64 * 300.0, 0.0, 250.0, 100.0), ItemKind::Shape(Box::new(s)))
    };
    let nested = item("g/sub", (900.0, 0.0, 250.0, 100.0), ItemKind::Group { items: vec![step(3)], smartart: false });
    vec![step(1), bare("g/arrow", "rightArrow", (260.0, 40.0, 30.0, 20.0)), step(2), nested]
}

#[test]
fn group_falls_back_to_a_stack_of_cards() {
    let (e, losses, fallback) = fold(&steps(), false);
    let e = e.unwrap();
    assert_that!((e.kind.as_str(), e.params.text("layout"))).is_equal_to(("stack", Some("columns")));
    let children = e.params.slot("children").unwrap();
    assert_that!(children.iter().map(|c| (c.id.as_str(), c.kind.as_str())).collect::<Vec<_>>()).is_equal_to(vec![
        ("e1-1", "card"),
        ("e1-2", "card"),
        ("e1-3", "card"),
    ]);
    assert_that!(children[0].params.text("title")).is_equal_to(Some("Étape 1"));
    assert_that!(body_texts(&children[2])).is_equal_to(vec!["Détail 3".to_owned()]);
    assert_that!(fallback.as_deref()).is_equal_to(Some("stack"));
    assert_that!(losses).is_empty();
}

#[test]
fn group_of_two_stacked_texts_is_a_silent_row_stack() {
    let items = vec![text_item("a", (0.0, 0.0, 300.0, 50.0), "A"), text_item("b", (0.0, 200.0, 300.0, 50.0), "B")];
    let (e, _, fallback) = fold(&items, false);
    let e = e.unwrap();
    assert_that!(e.params.text("layout")).is_equal_to(Some("rows"));
    assert_that!(e.params.slot("children").unwrap()[0].params.text("title")).is_none();
    assert_that!(fallback).is_none();
}

#[test]
fn group_of_a_smartart_is_always_a_fallback() {
    let items = vec![text_item("a", (0.0, 0.0, 300.0, 50.0), "A"), text_item("b", (0.0, 200.0, 300.0, 50.0), "B")];
    assert_that!(fold(&items, true).2.as_deref()).is_equal_to(Some("stack"));
}

#[test]
fn group_of_bare_shapes_is_nothing() {
    assert_that!(fold(&[bare("a", "rect", (0.0, 0.0, 10.0, 10.0))], false)).is_equal_to((None, Vec::new(), None));
}

fn boxes(frames: &[(f64, f64, f64, f64)]) -> Vec<Frame> {
    frames.iter().map(|&(x, y, w, h)| Frame::new(x, y, w, h)).collect()
}

/// Pas des centres : 300 et 300 pour la rangée régulière ; 300 et 600 pour l'irrégulière (0,5 < 0,75).
#[rstest]
#[case::row(boxes(&[(0.0, 0.0, 250.0, 100.0), (300.0, 0.0, 250.0, 100.0), (600.0, 0.0, 250.0, 100.0)]), true)]
#[case::column(boxes(&[(0.0, 0.0, 250.0, 100.0), (0.0, 150.0, 250.0, 100.0), (0.0, 300.0, 250.0, 100.0)]), true)]
#[case::two(boxes(&[(0.0, 0.0, 250.0, 100.0), (300.0, 0.0, 250.0, 100.0)]), false)]
#[case::uneven_steps(boxes(&[(0.0, 0.0, 250.0, 100.0), (300.0, 0.0, 250.0, 100.0), (900.0, 0.0, 250.0, 100.0)]), false)]
#[case::uneven_sizes(boxes(&[(0.0, 0.0, 250.0, 100.0), (300.0, 0.0, 100.0, 100.0), (600.0, 0.0, 250.0, 100.0)]), false)]
#[case::scattered(boxes(&[(0.0, 0.0, 250.0, 100.0), (300.0, 200.0, 250.0, 100.0), (600.0, 0.0, 250.0, 100.0)]), false)]
fn regular_finds_rows_and_columns_of_similar_evenly_spaced_boxes(#[case] frames: Vec<Frame>, #[case] expected: bool) {
    assert_that!(regular(&frames)).is_equal_to(expected);
}

fn rotated(mut i: Item) -> Item {
    i.frame.rot = 90.0;
    i
}

#[test]
fn group_reports_the_transform_of_a_rotated_leaf() {
    let items =
        vec![rotated(text_item("a", (0.0, 0.0, 300.0, 50.0), "A")), text_item("b", (0.0, 200.0, 300.0, 50.0), "B")];
    assert_that!(fold(&items, false).1).is_equal_to(vec![Loss::on("a", LossKind::Transform)]);
}

#[test]
fn group_reports_the_transform_of_a_flipped_nested_group() {
    let mut inner = item(
        "g/in",
        (0.0, 0.0, 300.0, 50.0),
        ItemKind::Group { items: vec![text_item("a", (0.0, 0.0, 300.0, 50.0), "A")], smartart: false },
    );
    inner.frame.flip_h = true;
    let items = vec![inner, text_item("b", (0.0, 200.0, 300.0, 50.0), "B")];
    assert_that!(fold(&items, false).1).is_equal_to(vec![Loss::on("g/in", LossKind::Transform)]);
}

#[test]
fn group_with_a_nested_smartart_is_a_fallback() {
    let nested = item(
        "g/sa",
        (0.0, 0.0, 300.0, 50.0),
        ItemKind::Group { items: vec![text_item("a", (0.0, 0.0, 300.0, 50.0), "A")], smartart: true },
    );
    let items = vec![nested, text_item("b", (0.0, 200.0, 300.0, 50.0), "B")];
    assert_that!(fold(&items, false).2.as_deref()).is_equal_to(Some("stack"));
}
