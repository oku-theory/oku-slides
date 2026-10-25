use oku_slides_core::PathCommand;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::source::{Dash, Rgba, Styled};

fn stroke(head: Option<ArrowKind>, tail: Option<ArrowKind>) -> Stroke {
    let arrow = |kind| Arrow { kind, width: ArrowSize::Medium, length: ArrowSize::Medium };
    Stroke {
        width: 2.0,
        color: Styled::explicit(Rgba::new(0, 0, 0)),
        dash: Dash::Solid,
        head: head.map(arrow),
        tail: tail.map(arrow),
    }
}

/// Une ligne horizontale de 100 pt, de gauche à droite.
const LINE: [PathCommand; 2] = [PathCommand::Move(0.0, 0.0), PathCommand::Line(1.0, 0.0)];

fn rounded(cmds: &[PathCommand]) -> Vec<(i64, i64)> {
    cmds.iter()
        .filter_map(|c| match *c {
            PathCommand::Move(x, y) | PathCommand::Line(x, y) => {
                Some((oku_slides_core::num::round_i64(x * 10.0), oku_slides_core::num::round_i64(y * 10.0)))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn heads_tail_triangle_points_along_the_line_at_its_end() {
    let h = heads(&LINE, 100.0, 0.0, 2.0, &stroke(None, Some(ArrowKind::Triangle)));
    assert_that!(h).has_length(1);
    // Longueur et largeur moyennes : 3 × 2 pt ; la pointe en (100, 0), la base à 94.
    assert_that!(rounded(&h[0].cmds)).is_equal_to(vec![(1000, 0), (940, 30), (940, -30)]);
    assert_that!(h[0].filled).is_true();
}

#[test]
fn heads_head_points_backwards_at_the_start() {
    let h = heads(&LINE, 100.0, 0.0, 2.0, &stroke(Some(ArrowKind::Triangle), None));
    assert_that!(rounded(&h[0].cmds)).is_equal_to(vec![(0, 0), (60, -30), (60, 30)]);
}

#[test]
fn heads_follow_the_last_segment_of_a_bent_path() {
    // Coude : vers la droite puis vers le bas ; la pointe de fin regarde vers le bas.
    let bent = [PathCommand::Move(0.0, 0.0), PathCommand::Line(1.0, 0.0), PathCommand::Line(1.0, 1.0)];
    let h = heads(&bent, 50.0, 50.0, 2.0, &stroke(None, Some(ArrowKind::Triangle)));
    assert_that!(rounded(&h[0].cmds)).is_equal_to(vec![(500, 500), (470, 440), (530, 440)]);
}

#[test]
fn heads_open_arrow_is_stroked_not_filled() {
    let h = heads(&LINE, 100.0, 0.0, 2.0, &stroke(None, Some(ArrowKind::Open)));
    assert_that!(h[0].filled).is_false();
    assert_that!(h[0].cmds.iter().any(|c| matches!(c, PathCommand::Close))).is_false();
}

#[rstest]
#[case(ArrowKind::Stealth)]
#[case(ArrowKind::Diamond)]
#[case(ArrowKind::Oval)]
fn heads_closed_kinds_are_filled_and_closed(#[case] kind: ArrowKind) {
    let h = heads(&LINE, 100.0, 0.0, 2.0, &stroke(Some(kind), Some(kind)));
    assert_that!(h).has_length(2);
    assert_that!(h.iter().all(|h| h.filled && h.cmds.last() == Some(&PathCommand::Close))).is_true();
}

#[test]
fn heads_of_a_closed_path_are_none() {
    let closed = [PathCommand::Move(0.0, 0.0), PathCommand::Line(1.0, 0.0), PathCommand::Close];
    assert_that!(heads(&closed, 10.0, 10.0, 1.0, &stroke(None, Some(ArrowKind::Triangle)))).is_empty();
}

#[test]
fn heads_of_a_point_are_none() {
    let point = [PathCommand::Move(0.5, 0.5), PathCommand::Line(0.5, 0.5)];
    assert_that!(heads(&point, 10.0, 10.0, 1.0, &stroke(None, Some(ArrowKind::Triangle)))).is_empty();
}

#[test]
fn heads_of_a_hairline_keep_a_visible_size() {
    let h = heads(&LINE, 100.0, 0.0, 0.25, &stroke(None, Some(ArrowKind::Triangle)));
    // Épaisseur prise à 1 pt au moins : 3 pt de long.
    assert_that!(rounded(&h[0].cmds)[1].0).is_equal_to(970);
}
