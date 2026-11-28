use speculoos::prelude::*;

use super::CellPatch;
use crate::test_support::{set_cell, sized};
use crate::{Data, DataCell, Source, Table};

fn data(table: Table) -> Data {
    Data { name: "sales".into(), label: None, inline: false, source: Source::Manual(table) }
}

fn key(row: &str, column: &str) -> crate::CellKey {
    crate::table::key(row, column)
}

/// → only the edited cell travels; applying it to the older version gives the newer one.
#[test]
fn cell_patch_between_versions_holds_only_the_edited_cell() {
    let mut table = sized(1000, 10);
    set_cell(&mut table, "r1", "c1", DataCell::Number(1.0));
    set_cell(&mut table, "r2", "c2", DataCell::Number(2.0));
    let older = data(table);
    let mut newer = older.clone();
    set_cell(newer.table_mut().unwrap(), "r3", "c3", DataCell::Text("x".into()));
    newer.table_mut().unwrap().cells.remove(&key("r1", "c1"));

    let patch = CellPatch::between(&older, &newer).unwrap();

    assert_that!(patch.set).is_equal_to(vec![(key("r3", "c3"), DataCell::Text("x".into()))]);
    assert_that!(patch.removed).is_equal_to(vec![key("r1", "c1")]);
    assert_that!(patch.applied(&older)).is_equal_to(Some(newer));
}

#[test]
fn cell_patch_between_versions_of_other_rows_is_none() {
    let older = data(sized(3, 2));
    let newer = data(sized(4, 2));
    assert_that!(CellPatch::between(&older, &newer)).is_none();
}

#[test]
fn cell_patch_between_renamed_versions_is_none() {
    let older = data(sized(3, 2));
    let mut newer = older.clone();
    newer.name = "other".into();
    assert_that!(CellPatch::between(&older, &newer)).is_none();
}

/// → a query data has no cells: it goes whole, and no patch applies to it.
#[test]
fn cell_patch_of_a_query_data_is_none() {
    let query =
        Data { name: "top".into(), label: None, inline: false, source: Source::Query { sql: "SELECT 1".into() } };
    assert_that!(CellPatch::between(&query, &query)).is_none();
    let patch = CellPatch { set: Vec::new(), removed: vec![key("r1", "c1")] };
    assert_that!(patch.applied(&query)).is_none();
}
