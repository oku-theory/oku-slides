use rstest::rstest;
use speculoos::prelude::*;

use super::*;

fn path(d: &str) -> ShapePath {
    ShapePath { d: d.to_owned() }
}

#[test]
fn commands_of_a_ring_read_two_subpaths() {
    let d = "M 1 0.5 C 1 0.776 0.776 1 0.5 1 Z M 0.75 0.5 Q 0.75 0.25 0.5 0.25 L .5 +0.75 Z";
    assert_that!(path(d).commands()).is_equal_to(Some(vec![
        PathCommand::Move(1.0, 0.5),
        PathCommand::Cubic(1.0, 0.776, 0.776, 1.0, 0.5, 1.0),
        PathCommand::Close,
        PathCommand::Move(0.75, 0.5),
        PathCommand::Quad(0.75, 0.25, 0.5, 0.25),
        PathCommand::Line(0.5, 0.75),
        PathCommand::Close,
    ]));
}

#[test]
fn commands_accept_any_whitespace_between_words() {
    assert_that!(path("  M 0 0\n\tL 1 1  ").commands())
        .is_equal_to(Some(vec![PathCommand::Move(0.0, 0.0), PathCommand::Line(1.0, 1.0)]));
}

#[rstest]
#[case::empty("")]
#[case::blank("   ")]
#[case::no_move("L 0 0 L 1 1")]
#[case::missing_number("M 0 L 1 1")]
#[case::extra_number("M 0 0 1 L 1 1")]
#[case::unknown_command("M 0 0 A 1 1 0 0 1 1 1")]
#[case::glued_letter("M0 0 L 1 1")]
#[case::exponent("M 0 0 L 1e2 1")]
#[case::far_away("M 0 0 L 1000.5 1")]
#[case::bad_number("M 0 0 L 1.2.3 1")]
#[case::lone_dot("M 0 0 L . 1")]
#[case::lowercase("m 0 0 l 1 1")]
#[case::number_on_close("M 0 0 Z 1")]
fn commands_of_a_malformed_path_return_none(#[case] d: &str) {
    assert_that!(path(d).commands()).is_none();
}

#[test]
fn commands_of_a_path_longer_than_the_bound_return_none() {
    let d = format!("M 0 0{}", " L 1 1".repeat(SHAPE_PATH_MAX / 6));
    assert_that!(d.len()).is_greater_than(SHAPE_PATH_MAX);
    assert_that!(path(&d).commands()).is_none();
}

#[test]
fn from_commands_writes_rounded_decimals_without_exponent() {
    let p = ShapePath::from_commands(&[
        PathCommand::Move(0.0, -0.0),
        PathCommand::Line(1.0 / 3.0, 1e-9),
        PathCommand::Cubic(0.5, 0.25, -0.125, 2.0, 1.0, 1.0),
        PathCommand::Close,
    ]);
    assert_that!(p.map(|p| p.d)).is_equal_to(Some("M 0 0 L 0.333333 0 C 0.5 0.25 -0.125 2 1 1 Z".to_owned()));
}

#[rstest]
#[case::empty(vec![])]
#[case::no_move(vec![PathCommand::Line(1.0, 1.0)])]
#[case::nan(vec![PathCommand::Move(f64::NAN, 0.0)])]
#[case::infinite(vec![PathCommand::Move(0.0, f64::INFINITY)])]
#[case::far_away(vec![PathCommand::Move(0.0, 0.0), PathCommand::Line(SHAPE_PATH_COORD_MAX * 2.0, 0.0)])]
fn from_commands_refused_by_the_syntax_returns_none(#[case] commands: Vec<PathCommand>) {
    assert_that!(ShapePath::from_commands(&commands)).is_none();
}

#[test]
fn from_commands_then_commands_round_trips() {
    let commands = vec![PathCommand::Move(0.0, 0.0), PathCommand::Quad(0.5, 1.0, 1.0, 0.0), PathCommand::Close];
    let p = ShapePath::from_commands(&commands).unwrap();
    assert_that!(p.commands()).is_equal_to(Some(commands));
}

#[test]
fn shape_path_with_an_unknown_key_is_refused() {
    assert_that!(serde_json::from_value::<ShapePath>(serde_json::json!({ "d": "M 0 0", "fill": "x" }))).is_err();
}
