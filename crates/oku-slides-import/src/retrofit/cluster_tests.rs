use std::collections::BTreeMap;

use speculoos::prelude::*;

use super::*;
use crate::source::{self, Geometry, Rgba, Stroke, Styled};
use crate::test_support::{item, text_item};

fn shape(id: &str, frame: (f64, f64, f64, f64), fill: Fill, stroke: Option<Stroke>) -> Item {
    let s = source::Shape {
        geom: Geometry::Preset { name: "rect".into(), adj: BTreeMap::new() },
        fill,
        stroke,
        text: None,
        connection: None,
    };
    item(id, frame, ItemKind::Shape(Box::new(s)))
}

fn filled(id: &str, frame: (f64, f64, f64, f64)) -> Item {
    shape(id, frame, Fill::Solid { color: Styled::explicit(Rgba::new(255, 255, 255)) }, None)
}

/// Une cellule par tranche de 100 pt en largeur et en hauteur.
fn cell(f: &Frame) -> Cell {
    // Truncated toward zero: a positive value's band, 0 for a negative one (saturation).
    let at = |v: f64| oku_slides_core::num::trunc_u32(v / 100.0);
    let (col, row) = (at(f.x), at(f.y));
    Cell { col, row, cols: at(f.x + f.w - 0.01) - col + 1, rows: at(f.y + f.h - 0.01) - row + 1 }
}

fn ids(units: &[Unit<'_>]) -> Vec<Vec<String>> {
    units.iter().map(|u| u.items().iter().map(|i| i.id.clone()).collect()).collect()
}

/// Slide 2 d'alto : une carte = un fond blanc, un filet d'accent en haut, un titre, un corps.
#[test]
fn units_make_a_card_of_a_filled_shape_and_the_texts_on_it() {
    let rule = Stroke {
        width: 2.0,
        color: Styled::explicit(Rgba::new(0, 0xbe, 0xf4)),
        dash: source::Dash::Solid,
        head: None,
        tail: None,
    };
    let items = [
        filled("ground", (64.0, 176.0, 277.0, 284.0)),
        shape("rule", (64.0, 177.0, 277.0, 0.0), Fill::None, Some(rule)),
        text_item("title", (80.0, 196.0, 253.0, 18.0), "The only window"),
        text_item("body", (80.0, 220.0, 253.0, 69.0), "The agent becomes…"),
        text_item("outside", (64.0, 497.0, 270.0, 12.0), "Pied"),
    ];
    let refs: Vec<&Item> = items.iter().collect();
    let units = units(&refs, cell);
    assert_that!(ids(&units)).is_equal_to(vec![
        vec!["ground".to_owned(), "title".to_owned(), "body".to_owned(), "rule".to_owned()],
        vec!["outside".to_owned()],
    ]);
    assert_that!(matches!(units[0], Unit::Card { .. })).is_true();
}

/// Un texte posé sur deux fonds emboîtés va sur le plus petit ; un texte qui ne
/// recouvre un fond qu'à moitié reste seul.
#[test]
fn units_put_a_text_on_the_smallest_ground_that_holds_it() {
    let items = [
        filled("panel", (0.0, 0.0, 900.0, 500.0)),
        filled("card", (100.0, 100.0, 200.0, 200.0)),
        text_item("in-card", (110.0, 110.0, 180.0, 40.0), "Carte"),
        text_item("in-panel", (500.0, 100.0, 200.0, 40.0), "Panneau"),
        text_item("half", (250.0, 350.0, 100.0, 300.0), "À cheval"),
    ];
    let refs: Vec<&Item> = items.iter().collect();
    let units = units(&refs, cell);
    assert_that!(ids(&units)).is_equal_to(vec![
        vec!["panel".to_owned(), "in-panel".to_owned()],
        vec!["card".to_owned(), "in-card".to_owned()],
        vec!["half".to_owned()],
    ]);
}

#[test]
fn units_merge_neighbouring_texts_whose_cells_overlap() {
    let items = [
        text_item("a", (0.0, 410.0, 300.0, 15.0), "Distribution"),
        text_item("far", (0.0, 100.0, 300.0, 15.0), "Loin"),
        text_item("b", (0.0, 450.0, 300.0, 15.0), "Matthieu"),
        text_item("c", (500.0, 450.0, 300.0, 15.0), "Ailleurs"),
    ];
    let refs: Vec<&Item> = items.iter().collect();
    let units = units(&refs, cell);
    assert_that!(ids(&units)).is_equal_to(vec![
        vec!["a".to_owned(), "b".to_owned()],
        vec!["far".to_owned()],
        vec!["c".to_owned()],
    ]);
    assert_that!(units[0].frame()).is_equal_to(Frame::new(0.0, 410.0, 300.0, 55.0));
}

#[test]
fn units_leave_a_bare_shape_without_text_alone() {
    let items = [filled("a", (0.0, 0.0, 100.0, 100.0)), filled("b", (10.0, 10.0, 20.0, 20.0))];
    let refs: Vec<&Item> = items.iter().collect();
    assert_that!(ids(&units(&refs, cell))).is_equal_to(vec![vec!["a".to_owned()], vec!["b".to_owned()]]);
}

#[test]
fn units_emit_a_merged_text_at_the_draw_position_of_its_earliest_member() {
    let items = [
        text_item("lower", (0.0, 450.0, 300.0, 15.0), "B"),
        filled("bar", (700.0, 0.0, 50.0, 50.0)),
        text_item("upper", (0.0, 410.0, 300.0, 15.0), "A"),
    ];
    let refs: Vec<&Item> = items.iter().collect();
    assert_that!(ids(&units(&refs, cell)))
        .is_equal_to(vec![vec!["upper".to_owned(), "lower".to_owned()], vec!["bar".to_owned()]]);
}
