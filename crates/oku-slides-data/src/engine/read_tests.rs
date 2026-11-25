use speculoos::prelude::*;

use super::{check_column_type, read_view};
use crate::table::key;
use crate::test_support::{based_table, fold, typed};
use crate::{ColumnType, DataCell, DataErrorKind, Table};

fn ids(of: &[&str]) -> Vec<String> {
    of.iter().map(|s| (*s).to_owned()).collect()
}

fn text(s: &str) -> DataCell {
    DataCell::Text(s.to_owned())
}

/// The data rows of a read, header dropped.
fn body(table: &Table, file: &tempfile::NamedTempFile, columns: &[&str]) -> Vec<Vec<DataCell>> {
    let read = read_view(table, file.reopen().unwrap(), &ids(columns), 0..100, 1000).unwrap();
    read.dataset.source[1..].to_vec()
}

#[test]
fn read_view_projects_columns_and_orders_rows() {
    let (table, file) = based_table(4, 3);

    let read = read_view(&table, file.reopen().unwrap(), &ids(&["c2", "c0"]), 1..3, 1000).unwrap();

    assert_that!(read.dataset.columns).is_equal_to(ids(&["c2", "c0"]));
    assert_that!(read.dataset.ids).is_equal_to(ids(&["", "r1", "r2"]));
    assert_that!(read.dataset.source).is_equal_to(vec![
        vec![text("C2"), text("C0")],
        vec![DataCell::Number(1002.0), DataCell::Number(1000.0)],
        vec![DataCell::Number(2002.0), DataCell::Number(2000.0)],
    ]);
}

#[test]
fn read_view_applies_overlay_over_base() {
    let (mut table, file) = based_table(2, 2);
    table.cells.insert(key("r1", "c0"), DataCell::Number(7.0));

    assert_that!(body(&table, &file, &["c0", "c1"])).is_equal_to(vec![
        vec![DataCell::Number(0.0), DataCell::Number(1.0)],
        vec![DataCell::Number(7.0), DataCell::Number(1001.0)],
    ]);
}

#[test]
fn read_view_hides_removed_rows_and_columns() {
    let (mut table, file) = based_table(3, 2);
    table.rows.remove(1);
    table.columns.retain(|c| c.id != "c0");

    assert_that!(body(&table, &file, &["c1"]))
        .is_equal_to(vec![vec![DataCell::Number(1.0)], vec![DataCell::Number(2001.0)]]);
    let error = read_view(&table, file.reopen().unwrap(), &ids(&["c0"]), 0..10, 1000).unwrap_err();
    assert_that!(error.kind()).is_equal_to(DataErrorKind::NoColumn);
}

#[test]
fn cleared_cell_hides_base_value() {
    let (mut table, file) = based_table(1, 1);
    table.cells.insert(key("r0", "c0"), DataCell::Null);

    assert_that!(body(&table, &file, &["c0"])).is_equal_to(vec![vec![DataCell::Null]]);
}

#[test]
fn retyped_column_converts_base_on_read() {
    let (mut table, file) = based_table(2, 1);
    table.set_column_type("c0", ColumnType::Text).unwrap();

    let read = read_view(&table, file.reopen().unwrap(), &ids(&["c0"]), 0..10, 1000).unwrap();

    assert_that!(read.dataset.source[1..].to_vec()).is_equal_to(vec![vec![text("0")], vec![text("1000")]]);
    assert_that!(read.misfit.is_empty()).is_true();
}

#[test]
fn unconvertible_base_cell_reads_as_text_and_reports() {
    let (mut table, file) = fold(&typed());
    table.set_column_type("t", ColumnType::Number).unwrap();

    let read = read_view(&table, file.reopen().unwrap(), &ids(&["t"]), 0..10, 1000).unwrap();

    assert_that!(read.dataset.source[1][0].clone()).is_equal_to(text("héllo"));
    assert_that!(read.misfit.contains("t")).is_true();
}

#[test]
fn check_column_type_names_base_rows_that_do_not_convert() {
    let (mut table, file) = fold(&typed());
    table.cells.insert(key("r0", "t"), text("12"));

    let ok = check_column_type(&table, file.reopen().unwrap(), "i", ColumnType::Number);
    let refused = check_column_type(&table, file.reopen().unwrap(), "d", ColumnType::Number);

    assert_that!(ok).is_ok();
    assert_that!(refused.unwrap_err().kind()).is_equal_to(DataErrorKind::ColumnType);
}

#[test]
fn column_absent_from_base_reads_overlay_or_null() {
    let (mut table, file) = based_table(2, 1);
    table.columns.push(crate::Column { id: "new".into(), name: "New".into(), ty: ColumnType::Text });
    table.cells.insert(key("r0", "new"), text("a"));

    assert_that!(body(&table, &file, &["new"])).is_equal_to(vec![vec![text("a")], vec![DataCell::Null]]);
}

#[test]
fn orphan_base_column_is_ignored() {
    let (mut table, file) = based_table(1, 2);
    table.columns.retain(|c| c.id != "c1");

    let read = read_view(&table, file.reopen().unwrap(), &ids(&["c0"]), 0..10, 1000).unwrap();

    assert_that!(read.dataset.source[1..].to_vec()).is_equal_to(vec![vec![DataCell::Number(0.0)]]);
}
