use speculoos::prelude::*;

use super::*;
use crate::source::{Arrow, ArrowKind, ArrowSize, Dash, Fill, Geometry, Rgba, Shape, Stroke, Styled};
use crate::test_support::{item, text_item};

fn rect(id: &str, frame: (f64, f64, f64, f64)) -> Item {
    let s = Shape {
        geom: Geometry::Preset { name: "rect".into(), adj: BTreeMap::new() },
        fill: Fill::None,
        stroke: None,
        text: None,
        connection: None,
    };
    item(id, frame, ItemKind::Shape(Box::new(s)))
}

/// Trois formes de 200 × 100 à x = 0, 300, 600, décalées de `(dx, dy)` et agrandies de `k`.
fn row(dx: f64, dy: f64, k: f64) -> Vec<Item> {
    (0..3).map(|i| rect(&format!("r{i}"), (dx + k * 300.0 * f64::from(i), dy, k * 200.0, k * 100.0))).collect()
}

#[test]
fn signature_counts_kinds_and_normalizes_boxes() {
    assert_that!(signature(&row(0.0, 0.0, 1.0))).is_equal_to("shape×3|0014,2034,3044".to_owned());
}

#[test]
fn signature_ignores_translation_and_scale() {
    assert_that!(signature(&row(500.0, 80.0, 2.5))).is_equal_to(signature(&row(0.0, 0.0, 1.0)));
}

#[test]
fn signature_ignores_drawing_order() {
    let mut items = row(0.0, 0.0, 1.0);
    items.reverse();
    assert_that!(signature(&items)).is_equal_to(signature(&row(0.0, 0.0, 1.0)));
}

#[test]
fn signature_tells_a_row_from_a_column() {
    let column: Vec<Item> = (0..3).map(|i| rect(&format!("c{i}"), (0.0, 300.0 * f64::from(i), 200.0, 100.0))).collect();
    assert_that!(signature(&column)).is_not_equal_to(signature(&row(0.0, 0.0, 1.0)));
}

#[test]
fn signature_tells_kinds_apart() {
    let mut items = row(0.0, 0.0, 1.0);
    items[1] = text_item("t", (300.0, 0.0, 200.0, 100.0), "Texte");
    assert_that!(signature(&items)).is_equal_to("shape×2,text×1|0014,2034,3044".to_owned());
}

#[test]
fn signature_of_one_item_or_none_is_stable() {
    assert_that!(signature(&[rect("a", (5.0, 5.0, 10.0, 10.0))])).is_equal_to("shape×1|0044".to_owned());
    assert_that!(signature(&[rect("a", (5.0, 5.0, 0.0, 0.0))])).is_equal_to("shape×1|0000".to_owned());
    assert_that!(signature(&[])).is_equal_to("|".to_owned());
}

/// Un connecteur droit de `frame`, fléché à la fin (`tail`) ou au début.
fn arrow_line(id: &str, frame: (f64, f64, f64, f64), flip_h: bool, tail: bool) -> Item {
    let arrow = Some(Arrow { kind: ArrowKind::Triangle, width: ArrowSize::Medium, length: ArrowSize::Medium });
    let stroke = Stroke {
        width: 1.0,
        color: Styled::explicit(Rgba::new(0, 0, 0)),
        dash: Dash::Solid,
        head: if tail { None } else { arrow },
        tail: if tail { arrow } else { None },
    };
    let s = Shape {
        geom: Geometry::Preset { name: "straightConnector1".into(), adj: BTreeMap::new() },
        fill: Fill::None,
        stroke: Some(stroke),
        text: None,
        connection: None,
    };
    let mut it = item(id, frame, ItemKind::Shape(Box::new(s)));
    it.frame.flip_h = flip_h;
    it
}

#[test]
fn signature_of_an_arrow_carries_its_direction() {
    let items = vec![rect("a", (0.0, 0.0, 100.0, 100.0)), arrow_line("l", (100.0, 50.0, 100.0, 0.0), false, true)];
    assert_that!(signature(&items)).is_equal_to("shape×2|0024,2242>e".to_owned());
}

#[test]
fn signature_of_opposite_arrows_differs() {
    let right = vec![arrow_line("l", (0.0, 0.0, 100.0, 0.0), false, true)];
    let left_by_flip = vec![arrow_line("l", (0.0, 0.0, 100.0, 0.0), true, true)];
    let left_by_head = vec![arrow_line("l", (0.0, 0.0, 100.0, 0.0), false, false)];
    assert_that!(signature(&right)).is_equal_to("shape×1|0040>e".to_owned());
    assert_that!(signature(&left_by_flip)).is_equal_to("shape×1|0040>w".to_owned());
    assert_that!(signature(&left_by_head)).is_equal_to(signature(&left_by_flip));
}
