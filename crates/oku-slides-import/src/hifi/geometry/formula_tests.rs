use rstest::rstest;
use speculoos::prelude::*;

use super::*;

fn eval(fmla: &str) -> Option<f64> {
    let mut env = Env::new(200.0, 100.0);
    env.set("g", -3.0);
    Formula::parse(fmla)?.eval(&env)
}

#[rstest]
#[case::mul_div("*/ w 1 4", 50.0)]
#[case::add_sub("+- 10 5 20", -5.0)]
#[case::extra_operand_ignored("+- 10 5 20 99", -5.0)]
#[case::add_div("+/ 10 20 3", 10.0)]
#[case::if_positive("?: 1 7 9", 7.0)]
#[case::if_zero("?: 0 7 9", 9.0)]
#[case::abs("abs g", 3.0)]
#[case::at2_right_angle("at2 0 5", 5_400_000.0)]
#[case::cat2("cat2 10 1 0", 10.0)]
#[case::cos_half_turn("cos 10 cd2", -10.0)]
#[case::max("max w h", 200.0)]
#[case::min("min w h", 100.0)]
#[case::modulus("mod 3 4 0", 5.0)]
#[case::pin_below("pin 0 g 10", 0.0)]
#[case::pin_above("pin 0 50 10", 10.0)]
#[case::pin_inside("pin 0 5 10", 5.0)]
#[case::sat2("sat2 10 0 1", 10.0)]
#[case::sin_quarter("sin 10 cd4", 10.0)]
#[case::sqrt("sqrt 16", 4.0)]
#[case::tan_eighth("tan 10 cd8", 10.0)]
#[case::val("val g", -3.0)]
#[case::builtins_sides("+- ss ls 0", 300.0)]
#[case::builtin_divided("+- wd4 hd2 ssd8", 87.5)]
#[case::builtin_angles("+- 3cd4 7cd8 5cd8", 21_600_000.0)]
#[case::centre("+- hc vc 0", 150.0)]
#[case::edges("+- r b t", 300.0)]
fn formula_known_cases_evaluate(#[case] fmla: &str, #[case] expected: f64) {
    assert_that!(eval(fmla).unwrap()).is_close_to(expected, 1e-9);
}

#[rstest]
#[case::division_by_zero("*/ w 1 0", 0.0)]
#[case::sum_over_zero("+/ w h 0", 0.0)]
#[case::negative_root("sqrt g", 0.0)]
fn formula_degenerate_operation_yields_zero(#[case] fmla: &str, #[case] expected: f64) {
    assert_that!(eval(fmla)).is_equal_to(Some(expected));
}

#[rstest]
#[case::unknown_operator("pow 2 3")]
#[case::wrong_arity("*/ 1 2")]
#[case::unknown_name("val nope")]
#[case::bad_divisor_name("val wd0")]
#[case::not_finite("*/ 1e308 1e308 1")]
#[case::empty("")]
fn formula_unusable_yields_none(#[case] fmla: &str) {
    assert_that!(eval(fmla)).is_none();
}

#[test]
fn operand_named_like_a_float_stays_a_name() {
    assert_that!(Operand::parse("inf")).is_equal_to(Operand::Name("inf".to_owned()));
    assert_that!(Operand::parse("-21600")).is_equal_to(Operand::Lit(-21600.0));
}
