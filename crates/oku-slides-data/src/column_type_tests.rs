use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::DataCell;
use crate::limits::MAX_TEXT_BYTES;

const TWO_53: f64 = 9_007_199_254_740_992.0;

fn text(s: &str) -> DataCell {
    DataCell::Text(s.to_owned())
}

#[rstest]
#[case::leap_day(ColumnType::Date, text("2024-02-29"), true)]
#[case::first_day(ColumnType::Date, text("0001-01-01"), true)]
#[case::last_day(ColumnType::Date, text("9999-12-31"), true)]
#[case::no_such_day(ColumnType::Date, text("2026-02-30"), false)]
#[case::no_such_month(ColumnType::Date, text("2026-13-01"), false)]
#[case::unpadded(ColumnType::Date, text("2026-1-1"), false)]
#[case::year_zero(ColumnType::Date, text("0000-01-01"), false)]
#[case::empty_date(ColumnType::Date, text(""), false)]
#[case::number_as_date(ColumnType::Date, DataCell::Number(2024.0), false)]
#[case::integer_max(ColumnType::Integer, DataCell::Number(TWO_53), true)]
#[case::integer_min(ColumnType::Integer, DataCell::Number(-TWO_53), true)]
#[case::integer_fraction(ColumnType::Integer, DataCell::Number(1.5), false)]
#[case::integer_too_large(ColumnType::Integer, DataCell::Number(TWO_53 + 2.0), false)]
#[case::number_nan(ColumnType::Number, DataCell::Number(f64::NAN), false)]
#[case::number_infinite(ColumnType::Number, DataCell::Number(f64::INFINITY), false)]
#[case::number(ColumnType::Number, DataCell::Number(1.5), true)]
#[case::number_text(ColumnType::Number, text("1"), false)]
#[case::bool(ColumnType::Bool, DataCell::Bool(false), true)]
#[case::bool_text(ColumnType::Bool, text("true"), false)]
#[case::text_at_bound(ColumnType::Text, text(&"a".repeat(MAX_TEXT_BYTES)), true)]
#[case::text_over_bound(ColumnType::Text, text(&"a".repeat(MAX_TEXT_BYTES + 1)), false)]
#[case::text_number(ColumnType::Text, DataCell::Number(1.0), false)]
#[case::null_text(ColumnType::Text, DataCell::Null, true)]
#[case::null_integer(ColumnType::Integer, DataCell::Null, true)]
#[case::null_date(ColumnType::Date, DataCell::Null, true)]
fn fits_follows_the_column_type(#[case] ty: ColumnType, #[case] cell: DataCell, #[case] expected: bool) {
    assert_that!(ty.fits(&cell)).is_equal_to(expected);
}

#[rstest]
#[case::empty(vec![], ColumnType::Text)]
#[case::only_null(vec![DataCell::Null], ColumnType::Text)]
#[case::numbers(vec![DataCell::Number(1.0), DataCell::Null, DataCell::Number(2.5)], ColumnType::Number)]
#[case::bools(vec![DataCell::Bool(true), DataCell::Null], ColumnType::Bool)]
#[case::number_and_text(vec![DataCell::Number(1.0), text("a")], ColumnType::Text)]
#[case::number_and_bool(vec![DataCell::Number(1.0), DataCell::Bool(true)], ColumnType::Text)]
#[case::dates(vec![text("2024-01-01")], ColumnType::Date)]
fn infer_ignores_nulls(#[case] cells: Vec<DataCell>, #[case] expected: ColumnType) {
    assert_that!(ColumnType::infer(&cells)).is_equal_to(expected);
}

#[test]
fn column_type_serializes_lowercase() {
    let json = serde_json::to_string(&ColumnType::Integer).unwrap();
    assert_that!(json).is_equal_to("\"integer\"".to_owned());
}
