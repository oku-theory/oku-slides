use std::f64::consts::{FRAC_PI_2, PI};

use speculoos::prelude::*;

use super::*;

fn end(c: &PathCommand) -> (f64, f64) {
    match *c {
        PathCommand::Move(x, y) | PathCommand::Line(x, y) | PathCommand::Quad(_, _, x, y) => (x, y),
        PathCommand::Cubic(_, _, _, _, x, y) => (x, y),
        PathCommand::Close => (f64::NAN, f64::NAN),
    }
}

fn assert_point(actual: (f64, f64), expected: (f64, f64)) {
    assert_that!(actual.0).is_close_to(expected.0, 1e-9);
    assert_that!(actual.1).is_close_to(expected.1, 1e-9);
}

#[test]
fn arc_quarter_circle_is_one_cubic_with_the_classic_handle() {
    // Du point (1, 0) d'un cercle de centre (0, 0), un quart de tour horaire : (0, 1).
    let mut pen = Pen::new(1.0, 1.0);
    pen.move_to((1.0, 0.0));
    pen.arc_to(1.0, 1.0, 0.0, FRAC_PI_2);
    let out = pen.finish();
    let k = 4.0 / 3.0 * (PI / 8.0).tan();
    assert_that!(out.len()).is_equal_to(2);
    let PathCommand::Cubic(x1, y1, x2, y2, x, y) = out[1] else { panic!("cubique attendue") };
    for (a, e) in [(x1, 1.0), (y1, k), (x2, k), (y2, 1.0), (x, 0.0), (y, 1.0)] {
        assert_that!(a).is_close_to(e, 1e-9);
    }
}

#[test]
fn arc_full_ellipse_is_four_cubics_back_to_the_start() {
    let mut pen = Pen::new(1.0, 1.0);
    pen.move_to((0.0, 5.0));
    pen.arc_to(10.0, 5.0, PI, 2.0 * PI);
    let out = pen.finish();
    assert_that!(out.len()).is_equal_to(5);
    assert_point(end(&out[1]), (10.0, 0.0));
    assert_point(end(&out[2]), (20.0, 5.0));
    assert_point(end(&out[4]), (0.0, 5.0));
}

#[test]
fn arc_angles_are_visual_on_an_ellipse() {
    // Ellipse 2 × 1 centrée en (0, 0), de l'angle visuel 0 à 45° : le point
    // d'arrivée est sur la diagonale, pas au paramètre 45°.
    let mut pen = Pen::new(1.0, 1.0);
    pen.move_to((2.0, 0.0));
    pen.arc_to(2.0, 1.0, 0.0, PI / 4.0);
    let (x, y) = end(&pen.finish()[1]);
    assert_that!(x).is_close_to(y, 1e-9);
    assert_that!(x * x / 4.0 + y * y).is_close_to(1.0, 1e-9);
}

#[test]
fn arc_counter_clockwise_and_beyond_a_turn_is_capped() {
    let mut pen = Pen::new(1.0, 1.0);
    pen.move_to((1.0, 0.0));
    pen.arc_to(1.0, 1.0, 0.0, -FRAC_PI_2);
    pen.arc_to(1.0, 1.0, -FRAC_PI_2, 7.0 * PI);
    let out = pen.finish();
    assert_point(end(&out[1]), (0.0, -1.0));
    assert_that!(out.len()).is_equal_to(6);
}

#[test]
fn arc_with_zero_sweep_draws_nothing() {
    let mut pen = Pen::new(1.0, 1.0);
    pen.move_to((1.0, 0.0));
    pen.arc_to(1.0, 1.0, 0.0, 0.0);
    assert_that!(pen.finish().len()).is_equal_to(1);
}

#[test]
fn line_after_close_restarts_from_the_subpath_start() {
    let mut pen = Pen::new(0.5, 0.25);
    pen.move_to((2.0, 4.0));
    pen.line_to((4.0, 4.0));
    pen.close();
    pen.line_to((2.0, 0.0));
    assert_that!(pen.finish()).is_equal_to(vec![
        PathCommand::Move(1.0, 1.0),
        PathCommand::Line(2.0, 1.0),
        PathCommand::Close,
        PathCommand::Move(1.0, 1.0),
        PathCommand::Line(1.0, 0.0),
    ]);
}
