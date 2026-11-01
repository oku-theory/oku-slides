use speculoos::prelude::*;

use super::*;
use crate::model::CartesianSeries;
use crate::option::Length;

fn series(stack: Option<&str>, values: Vec<Option<f64>>) -> CartesianSeries {
    CartesianSeries {
        name: String::new(),
        mark: CartesianMark::Bar,
        values,
        label: false,
        stack: stack.map(str::to_owned),
        color: None,
    }
}

fn cartesian(series: Vec<CartesianSeries>, gap: &str) -> Cartesian {
    Cartesian {
        category_label: String::new(),
        categories: vec!["a".into(), "b".into()],
        horizontal: false,
        series,
        smooth: false,
        area: false,
        value_min: None,
        value_max: None,
        value_splits: None,
        bar_gap: Some(Length::Percent(gap.into())),
    }
}

/// A charts-rs bar: `x`, `width`, then its data attributes.
fn rect(x: f32, width: f32) -> String {
    format!(r##"<rect x="{x}" y="10" width="{width}" height="50" fill="#000" data-series="s" data-value="1"/>"##)
}

#[test]
fn slots_stacks_share_a_slot_and_empty_values_draw_nothing() {
    let c = cartesian(
        vec![
            series(Some("t"), vec![Some(1.0), None]),
            series(None, vec![Some(2.0), Some(3.0)]),
            series(Some("t"), vec![Some(4.0), Some(5.0)]),
        ],
        "50%",
    );
    assert_that!(slots(&c)).is_equal_to((vec![0, 1, 1, 0, 0], 2));
}

#[test]
fn slots_horizontal_number_stack_groups_before_plain_series() {
    let mut c = cartesian(
        vec![
            series(None, vec![Some(1.0), Some(2.0)]),
            series(Some("s"), vec![Some(3.0), Some(4.0)]),
            series(Some("s"), vec![Some(5.0), Some(6.0)]),
        ],
        "50%",
    );
    c.horizontal = true;
    assert_that!(slots(&c)).is_equal_to((vec![1, 1, 0, 0, 0, 0], 2));
}

#[test]
fn slots_columns_number_series_in_order() {
    let c = cartesian(
        vec![
            series(None, vec![Some(1.0), Some(2.0)]),
            series(Some("s"), vec![Some(3.0), Some(4.0)]),
            series(Some("s"), vec![Some(5.0), Some(6.0)]),
        ],
        "50%",
    );
    assert_that!(slots(&c)).is_equal_to((vec![0, 0, 1, 1, 0, 0], 2));
}

#[test]
fn slots_without_categories_still_count_the_first_value() {
    let mut c = cartesian(vec![series(None, vec![Some(1.0), Some(2.0)])], "50%");
    c.categories.clear();
    assert_that!(slots(&c)).is_equal_to((vec![0], 1));
}

#[test]
fn narrow_one_slot_keeps_the_centre_and_frees_the_gap() {
    // Band of 100: margins 5 + 90 of bar; a 60 % gap leaves a bar of 40 around 50.
    let svg = format!("<svg>{}{}</svg>", rect(5.0, 90.0), rect(105.0, 90.0));
    let c = cartesian(vec![series(None, vec![Some(1.0), Some(2.0)])], "60%");
    let out = narrow(svg, &c).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(out).is_equal_to(format!("<svg>{}{}</svg>", rect(30.0, 40.0), rect(130.0, 40.0)));
}

#[test]
fn narrow_two_slots_touch_around_the_band_centre() {
    // Band of 100: 5 + 43.5 + 3 + 43.5 + 5; a 20 % gap leaves two bars of 40 from 10 to 90.
    let svg = format!("{}{}", rect(5.0, 43.5), rect(51.5, 43.5));
    let mut c = cartesian(vec![series(None, vec![Some(1.0)]), series(None, vec![Some(2.0)])], "20%");
    c.categories.truncate(1);
    let out = narrow(svg, &c).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(out).is_equal_to(format!("{}{}", rect(10.0, 40.0), rect(50.0, 40.0)));
}

#[test]
fn narrow_moves_a_value_label_with_its_bar() {
    // The first bar's centre goes from 26.75 to 30; the axis text before the bars stays.
    let svg = format!(
        r#"<text x="26.8">T1</text>{}{}<text x="26.8" dx="-4" dy="-8">1</text>"#,
        rect(5.0, 43.5),
        rect(51.5, 43.5)
    );
    let mut c = cartesian(vec![series(None, vec![Some(1.0)]), series(None, vec![Some(2.0)])], "20%");
    c.categories.truncate(1);
    let out = narrow(svg, &c).unwrap_or_else(|e| panic!("{e}"));
    assert_that!((out.contains(r#"<text x="26.8">T1"#), out.contains(r#"<text x="30" dx="-4""#)))
        .is_equal_to((true, true));
}

#[test]
fn narrow_bar_count_off_fails_as_a_render_error() {
    let c = cartesian(vec![series(None, vec![Some(1.0), Some(2.0)])], "60%");
    let err = narrow(rect(5.0, 90.0), &c).err().map(|e| e.kind());
    assert_that!(err).is_equal_to(Some(crate::ChartErrorKind::Render));
}

#[test]
fn narrow_without_gap_returns_the_svg_unchanged() {
    let mut c = cartesian(vec![series(None, vec![Some(1.0), Some(2.0)])], "60%");
    c.bar_gap = None;
    assert_that!(narrow("<svg/>".into(), &c).ok()).is_equal_to(Some("<svg/>".to_owned()));
}

#[test]
fn number_writes_one_decimal_without_trailing_zero() {
    assert_that!((number(30.0), number(12.34))).is_equal_to(("30".into(), "12.3".into()));
}
