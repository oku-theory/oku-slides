//! Widget `table`: `column-widths` and `row-heights` weights drawn by Typst (t-955, t-1385).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_core::kinds::{base, essentiel};
use oku_slides_render::Compiled;
use serde_json::{Value, json};
use speculoos::prelude::*;

/// A blank slide holding only the table `data`, in a 300×120 pt frame, rules painted as the page.
fn compile(data: &Value) -> Compiled {
    let deck = json!({
        "tokens": { "component": { "base": { "table": { "rule-color": "bg" } } } },
        "slides": [{ "id": "s", "kind": essentiel::BLANK, "params": { "elements": [
            { "id": "t", "kind": base::TABLE, "params": { "data": data },
                "frame": { "x": 10, "y": 10, "w": 300, "h": 120 } }] } }]
    });
    let deck = Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("{e}"));
    common::renderer().render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"))
}

fn with(mut data: Value, key: &str, v: Value) -> Value {
    data[key] = v;
    data
}

/// One row `X | X`, the first cell at the end of its column, the second at its start: the gap
/// between the two inks is centred on the column boundary.
fn two_columns() -> Value {
    json!({ "header-row": false, "align": ["end", "start"], "dataset": { "source": [["X", "X"]] } })
}

/// The x of the boundary between the two columns of `data` (see [`two_columns`]), in pt from the frame.
fn column_boundary(data: &Value) -> f64 {
    let runs = common::ink_runs(&compile(data), "t", true);
    let [(_, end), (start, _)] = runs[..] else { panic!("two inked columns expected, got {runs:?}") };
    oku_slides_core::num::count_f64(end + start) / 2.0
}

/// Two rows `X`, one column.
fn two_rows() -> Value {
    json!({ "header-row": false, "dataset": { "source": [["X"], ["X"]] } })
}

/// The y centres of the two rows' inks of `data` (see [`two_rows`]), in pt from the frame.
fn row_centres(data: &Value) -> (f64, f64) {
    let runs = common::ink_runs(&compile(data), "t", false);
    let centre = |(a, b): (usize, usize)| oku_slides_core::num::count_f64(a + b) / 2.0;
    let [first, second] = runs[..] else { panic!("two inked rows expected, got {runs:?}") };
    (centre(first), centre(second))
}

#[test]
fn table_column_widths_one_to_two_draws_the_second_column_twice_as_wide() {
    let boundary = column_boundary(&with(two_columns(), "column-widths", json!([1, 2])));
    // The first column takes a third of the 300 pt frame.
    assert_that!(boundary).is_close_to(100.0, 2.0);
}

#[test]
fn table_without_column_widths_draws_equal_columns() {
    assert_that!(column_boundary(&two_columns())).is_close_to(150.0, 2.0);
}

#[rstest::rstest]
#[case::too_many(json!([1, 2, 3]))]
#[case::too_few(json!([1]))]
fn table_column_widths_of_the_wrong_count_draws_unweighted(#[case] weights: Value) {
    let weighted = column_boundary(&with(two_columns(), "column-widths", weights));
    assert_that!(weighted).is_close_to(column_boundary(&two_columns()), 0.5);
}

#[test]
fn table_row_heights_one_to_three_draws_the_second_row_three_times_as_tall() {
    let (first, second) = row_centres(&with(two_rows(), "row-heights", json!([1, 3])));
    // Rows of 30 and 90 pt, text centred in each.
    assert_that!(first).is_close_to(15.0, 2.0);
    assert_that!(second).is_close_to(75.0, 2.0);
}

#[test]
fn table_row_heights_of_the_wrong_count_draws_unweighted() {
    let weighted = row_centres(&with(two_rows(), "row-heights", json!([1, 2, 3])));
    let (first, second) = row_centres(&two_rows());
    assert_that!(weighted.0).is_close_to(first, 0.5);
    assert_that!(weighted.1).is_close_to(second, 0.5);
}
