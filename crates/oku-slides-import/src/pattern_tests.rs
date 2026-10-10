use speculoos::prelude::*;

use super::*;
use crate::source::{Fill, Geometry, Shape};
use crate::test_support::{item, text_item};

fn rect(id: &str, frame: (f64, f64, f64, f64)) -> Item {
    let s = Shape {
        geom: Geometry::Preset { name: "rect".into(), adj: BTreeMap::new() },
        fill: Fill::None,
        stroke: None,
        text: None,
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
