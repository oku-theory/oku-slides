use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::DataErrorKind;
use crate::error::DataInner;
use crate::test_support::{base, new_table, sized, small};

/// A table of one column `c` of type `ty`, rows `r0`… holding `cells`.
fn one_column(ty: ColumnType, cells: Vec<DataCell>) -> Table {
    let rows = cells.into_iter().map(|cell| vec![cell]).collect();
    new_table(&[("c", Some(ty))], rows).into_table().unwrap().0
}

fn text(s: &str) -> DataCell {
    DataCell::Text(s.to_owned())
}

#[test]
fn cell_reads_overlay_cleared_base_and_null() {
    let mut table = small();
    assert_that!(table.cell("r0", "a")).is_equal_to(CellRead::Overlay(&text("x")));
    assert_that!(table.cell("r9", "a")).is_equal_to(CellRead::Null);
    table.base = Some(base());
    table.cells.insert(key("r1", "a"), DataCell::Null);
    assert_that!(table.cell("r1", "a")).is_equal_to(CellRead::Cleared);
    assert_that!(table.cell("r9", "a")).is_equal_to(CellRead::Base);
}

#[test]
fn overlay_len_and_text_bytes_count_the_overlay() {
    let table = small();
    assert_that!(table.overlay_len()).is_equal_to(4);
    // "x" + "y" + "Name a" + "Name n"
    assert_that!(table.text_bytes()).is_equal_to(14);
}

#[test]
fn stored_table_without_generation_fails_to_parse() {
    let parsed = serde_json::from_str::<Table>(r#"{"columns":[],"rows":[]}"#);
    assert_that!(parsed).is_err();
}

#[test]
fn stored_null_cell_without_base_is_dropped_at_parse() {
    let json =
        r#"{"generation":"g","columns":[{"id":"c","name":"C","type":"text"}],"rows":["r"],"cells":{"r/c":null}}"#;
    let table: Table = serde_json::from_str(json).unwrap();
    assert_that!(table.overlay_len()).is_equal_to(0);
    assert_that!(table.cell("r", "c")).is_equal_to(CellRead::Null);
}

#[test]
fn stored_null_cell_with_base_is_kept_as_cleared() {
    let mut json = serde_json::json!({
        "generation": "g",
        "columns": [{"id": "c", "name": "C", "type": "text"}],
        "rows": ["r"],
        "cells": {"r/c": null}
    });
    json["base"] = serde_json::to_value(base()).unwrap();
    let table: Table = serde_json::from_value(json).unwrap();
    assert_that!(table.cell("r", "c")).is_equal_to(CellRead::Cleared);
}

#[rstest]
#[case::without_base(false)]
#[case::with_base(true)]
fn null_marker_round_trips_through_json(#[case] with_base: bool) {
    let mut table = small();
    if with_base {
        table.base = Some(base());
        table.cells.insert(key("r0", "a"), DataCell::Null);
    }
    let back: Table = serde_json::from_str(&serde_json::to_string(&table).unwrap()).unwrap();
    assert_that!(back).is_equal_to(&table);
    let expected = if with_base { CellRead::Cleared } else { CellRead::Overlay(&text("x")) };
    assert_that!(back.cell("r0", "a")).is_equal_to(expected);
}

#[rstest]
#[case::slash_in_row(r#""a/b/c""#)]
#[case::empty_row(r#""/c""#)]
#[case::empty_column(r#""r/""#)]
#[case::no_slash(r#""rc""#)]
#[case::bad_char(r#""r/é""#)]
fn cell_key_with_invalid_part_fails_to_parse(#[case] key: &str) {
    assert_that!(serde_json::from_str::<CellKey>(key)).is_err();
}

#[test]
fn table_with_unknown_field_fails_to_parse() {
    let parsed = serde_json::from_str::<Table>(r#"{"generation":"g","columns":[],"rows":[],"extra":1}"#);
    assert_that!(parsed).is_err();
}

#[test]
fn to_dataset_projection_orders_columns() {
    let ds = small().to_dataset(Some(&["n".to_owned(), "a".to_owned()]), 0..10, 100).unwrap();
    assert_that!(ds.columns).is_equal_to(vec!["n".to_owned(), "a".to_owned()]);
    assert_that!(ds.source[0]).is_equal_to(vec![text("Name n"), text("Name a")]);
    assert_that!(ds.source[1]).is_equal_to(vec![DataCell::Number(1.0), text("x")]);
}

#[test]
fn to_dataset_unknown_column_is_refused() {
    let error = small().to_dataset(Some(&["zz".to_owned()]), 0..10, 100).unwrap_err();
    assert_that!(error.kind()).is_equal_to(DataErrorKind::NoColumn);
}

#[rstest]
#[case::empty(vec![])]
#[case::duplicate(vec!["a".to_owned(), "a".to_owned()])]
fn to_dataset_bad_projection_is_refused(#[case] columns: Vec<String>) {
    let error = small().to_dataset(Some(&columns), 0..10, 100).unwrap_err();
    assert_that!(error.kind()).is_equal_to(DataErrorKind::ColumnsInvalid);
}

#[rstest]
#[case::rows_98(98, 99)]
#[case::rows_99(99, 100)]
#[case::rows_100(100, 100)]
#[case::rows_150(150, 100)]
fn to_dataset_row_range_excludes_header(#[case] rows: usize, #[case] lines: usize) {
    let ds = sized(rows, 2).to_dataset(None, 0..99, 100).unwrap();
    assert_that!(ds.source.len()).is_equal_to(lines);
    assert_that!(ds.ids.len()).is_equal_to(lines);
}

#[test]
fn to_dataset_ids_zero_is_header_placeholder() {
    let ds = small().to_dataset(None, 1..2, 100).unwrap();
    assert_that!(ds.ids).is_equal_to(vec![String::new(), "r1".to_owned()]);
}

#[test]
fn to_dataset_cuts_text_cells() {
    let table = one_column(ColumnType::Text, vec![text(&"é".repeat(1_000)), text(&"é".repeat(1_001))]);
    let ds = table.to_dataset(None, 0..2, 1_000).unwrap();
    assert_that!(ds.source[1][0]).is_equal_to(text(&"é".repeat(1_000)));
    assert_that!(ds.source[2][0]).is_equal_to(text(&"é".repeat(1_000)));
}

#[test]
fn to_dataset_with_base_is_needs_base() {
    let mut table = small();
    table.base = Some(base());
    assert_that!(table.to_dataset(None, 0..2, 100).map(drop).map_err(|e| e.kind()))
        .is_err_containing(DataErrorKind::NeedsBase);
}

#[rstest]
#[case::point(ColumnType::Text, text("12.5"), ColumnType::Number, DataCell::Number(12.5))]
#[case::comma(ColumnType::Text, text("12,5"), ColumnType::Number, DataCell::Number(12.5))]
#[case::integer(ColumnType::Text, text("12"), ColumnType::Integer, DataCell::Number(12.0))]
#[case::bool(ColumnType::Text, text("true"), ColumnType::Bool, DataCell::Bool(true))]
#[case::date(ColumnType::Text, text("2024-03-01"), ColumnType::Date, text("2024-03-01"))]
#[case::number_to_integer(ColumnType::Number, DataCell::Number(3.0), ColumnType::Integer, DataCell::Number(3.0))]
#[case::integer_to_number(ColumnType::Integer, DataCell::Number(3.0), ColumnType::Number, DataCell::Number(3.0))]
#[case::number_to_text(ColumnType::Number, DataCell::Number(1.5), ColumnType::Text, text("1.5"))]
#[case::bool_to_text(ColumnType::Bool, DataCell::Bool(false), ColumnType::Text, text("false"))]
fn set_column_type_converts_without_loss(
    #[case] from: ColumnType,
    #[case] cell: DataCell,
    #[case] to: ColumnType,
    #[case] expected: DataCell,
) {
    let mut table = one_column(from, vec![cell]);
    table.set_column_type("c", to).unwrap();
    assert_that!(table.columns[0].ty).is_equal_to(to);
    assert_that!(table.cell("r0", "c")).is_equal_to(CellRead::Overlay(&expected));
}

#[rstest]
#[case::sixteen_digits(ColumnType::Text, text("9007199254740993"), ColumnType::Number)]
#[case::exponent(ColumnType::Text, text("1e21"), ColumnType::Number)]
#[case::grouping(ColumnType::Text, text("1,000"), ColumnType::Number)]
#[case::space_grouping(ColumnType::Text, text("1 000"), ColumnType::Number)]
#[case::upper_true(ColumnType::Text, text("TRUE"), ColumnType::Bool)]
#[case::fraction_to_integer(ColumnType::Number, DataCell::Number(1.5), ColumnType::Integer)]
#[case::date_to_number(ColumnType::Date, text("2024-03-01"), ColumnType::Number)]
fn set_column_type_refuses_a_loss_naming_rows(
    #[case] from: ColumnType,
    #[case] cell: DataCell,
    #[case] to: ColumnType,
) {
    let mut table = one_column(from, vec![DataCell::Null, cell]);
    let before = table.clone();
    let error = table.set_column_type("c", to).unwrap_err();
    assert_that!(error.kind()).is_equal_to(DataErrorKind::ColumnType);
    assert_that!(error.localize(&Localizer::new(Some("fr")))).contains("r1");
    assert_that!(table).is_equal_to(before);
}

#[test]
fn set_column_type_negative_zero_becomes_text_zero() {
    let mut table = one_column(ColumnType::Text, vec![text("-0")]);
    table.set_column_type("c", ColumnType::Number).unwrap();
    let DataCell::Number(zero) = table.cells[&key("r0", "c")] else { panic!("not a number") };
    assert_that!(zero.is_sign_negative()).is_true();
    table.set_column_type("c", ColumnType::Text).unwrap();
    assert_that!(table.cell("r0", "c")).is_equal_to(CellRead::Overlay(&text("0")));
}

#[test]
fn set_column_type_names_five_rows_and_counts_the_others() {
    let mut table = one_column(ColumnType::Text, vec![text("a"); 6]);
    let error = table.set_column_type("c", ColumnType::Number).unwrap_err();
    let DataInner::ColumnType { rows, more, .. } = &error.0 else { panic!("not a column type error") };
    assert_that!(rows.len()).is_equal_to(5);
    assert_that!(*more).is_equal_to(1);
}

#[test]
fn set_column_type_integer_number_round_trip_keeps_cells() {
    let mut table = one_column(ColumnType::Integer, vec![DataCell::Number(7.0), DataCell::Number(-2.0)]);
    let before = table.clone();
    table.set_column_type("c", ColumnType::Number).unwrap();
    table.set_column_type("c", ColumnType::Integer).unwrap();
    assert_that!(table).is_equal_to(before);
}

#[test]
fn set_column_type_leaves_absent_cells_absent() {
    let mut table = one_column(ColumnType::Text, vec![DataCell::Null, text("1")]);
    table.set_column_type("c", ColumnType::Number).unwrap();
    assert_that!(table.cell("r0", "c")).is_equal_to(CellRead::Null);
    assert_that!(table.overlay_len()).is_equal_to(1);
}

#[test]
fn set_column_type_unknown_column_is_refused() {
    let mut table = small();
    assert_that!(table.set_column_type("zz", ColumnType::Text).map_err(|e| e.kind()))
        .is_err_containing(DataErrorKind::NoColumn);
}

#[test]
fn set_column_type_with_base_checks_only_the_overlay() {
    // The base's cells are checked by the engine (`check_column_type`) before the op runs.
    let mut table = small();
    table.base = Some(base());
    assert_that!(table.set_column_type("a", ColumnType::Text)).is_ok();
}

#[test]
fn canonicalize_repairs_duplicate_column_ids_and_drops_orphan_cells() {
    let mut table = crate::test_support::sized(2, 2);
    table.columns[1].id = "c0".into();
    table.cells.insert(CellKey { row: "r0".into(), column: "c0".into() }, DataCell::Number(1.0));
    table.cells.insert(CellKey { row: "gone".into(), column: "c0".into() }, DataCell::Number(2.0));
    table.cells.insert(CellKey { row: "r1".into(), column: "gone".into() }, DataCell::Number(3.0));
    table.canonicalize();
    let ids: Vec<&str> = table.columns.iter().map(|c| c.id.as_str()).collect();
    assert_that!(ids).is_equal_to(vec!["c0", "c0_2"]);
    assert_that!(table.cells.len()).is_equal_to(1);
    let before = table.clone();
    table.canonicalize();
    assert_that!(table).is_equal_to(before);
}
