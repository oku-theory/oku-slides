use std::collections::BTreeMap;

use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::limits::{MAX_COLUMNS, MAX_OVERLAY, MAX_ROWS};
use crate::table::CellKey;
use crate::test_support::{base, counter, sized, small};
use crate::{CellRead, DataErrorKind, NewTable};

fn text(s: &str) -> DataCell {
    DataCell::Text(s.to_owned())
}

fn set(row: &str, column: &str, value: DataCell) -> TableOp {
    TableOp::SetCell { row: row.to_owned(), column: column.to_owned(), value }
}

fn insert_row(id: Option<&str>, after: Option<&str>, cells: &[(&str, DataCell)]) -> TableOp {
    TableOp::InsertRow {
        id: id.map(str::to_owned),
        after: after.map(str::to_owned),
        cells: cells.iter().map(|(c, v)| ((*c).to_owned(), v.clone())).collect(),
    }
}

fn insert_column(id: Option<&str>, after: Option<&str>, cells: &[(&str, DataCell)]) -> TableOp {
    TableOp::InsertColumn {
        id: id.map(str::to_owned),
        after: after.map(str::to_owned),
        name: "New".to_owned(),
        cells: cells.iter().map(|(r, v)| ((*r).to_owned(), v.clone())).collect(),
    }
}

/// The small table's contents as a `Replace`, keeping its ids, with row `r1` left out.
fn replace_without_r1() -> TableOp {
    let mut table = NewTable::from(&small());
    table.rows.retain(|row| row.id.as_deref() != Some("r1"));
    table.overlay = table.overlay.into_iter().filter(|(key, _)| key.row != "r1").collect();
    table.generation = Some("g2".to_owned());
    TableOp::Replace { table }
}

/// Applies `op` to the small table, unwrapping the result.
fn applied(op: &TableOp) -> Table {
    let mut table = small();
    table.apply(op).unwrap();
    table
}

/// The kind of the error `op` gets on `table`.
fn refused(table: &mut Table, op: &TableOp) -> DataErrorKind {
    table.apply(op).unwrap_err().kind()
}

#[test]
fn apply_set_cell_wrong_type_is_refused() {
    let mut table = small();
    let error = table.apply(&set("r0", "n", text("secret-value"))).unwrap_err();
    let fr = error.localize(&Localizer::new(Some("fr")));
    assert_that!(error.kind()).is_equal_to(DataErrorKind::CellType);
    assert_that!(fr).contains("Name n");
    assert_that!(fr.contains("secret-value")).is_false();
}

#[rstest]
#[case::set_cell(set("r0", "n", DataCell::Number(5.0)), "", "")]
#[case::rename_column(TableOp::RenameColumn { column: "a".to_owned(), name: "B".to_owned() }, "", "")]
#[case::insert_row(insert_row(Some("r9"), Some("r0"), &[]), "", "")]
#[case::remove_row(TableOp::RemoveRow { row: "r0".to_owned() }, "r0", "")]
#[case::insert_column(insert_column(Some("z"), None, &[]), "", "")]
#[case::remove_column(TableOp::RemoveColumn { column: "a".to_owned() }, "", "a")]
#[case::replace(replace_without_r1(), "r1", "")]
fn apply_each_table_op_keeps_other_ids(#[case] op: TableOp, #[case] removed_row: &str, #[case] removed_column: &str) {
    let before = small();
    let after = applied(&op);
    for row in before.rows.iter().filter(|r| *r != removed_row) {
        assert_that!(after.rows.contains(row)).named(row).is_true();
    }
    for column in before.columns.iter().filter(|c| c.id != removed_column) {
        assert_that!(after.columns.iter().any(|c| c.id == column.id)).named(&column.id).is_true();
    }
}

#[rstest]
#[case::set_cell(set("r0", "zz", DataCell::Number(5.0)))]
#[case::rename_column(TableOp::RenameColumn { column: "zz".to_owned(), name: "B".to_owned() })]
#[case::insert_row(insert_row(Some("r9"), None, &[("n", text("x"))]))]
#[case::remove_row(TableOp::RemoveRow { row: "zz".to_owned() })]
#[case::insert_column(insert_column(Some("z"), None, &[("zz", DataCell::Number(1.0))]))]
#[case::remove_column(TableOp::RemoveColumn { column: "zz".to_owned() })]
#[case::replace(TableOp::Replace { table: NewTable { generation: None, ..NewTable::from(&small()) } })]
fn apply_failure_leaves_table_unchanged(#[case] op: TableOp) {
    let mut table = small();
    assert_that!(table.apply(&op)).is_err();
    assert_that!(table).is_equal_to(small());
}

#[test]
fn set_cell_on_clone_leaves_original() {
    // Enough rows that `rows` leaves imbl's inline storage, which never shares.
    let original = sized(64, 2);
    let mut copy = original.clone();
    copy.apply(&set("r0", "c0", DataCell::Number(9.0))).unwrap();
    assert_that!(original).is_equal_to(sized(64, 2));
    assert_that!(copy.rows.ptr_eq(&original.rows)).is_true();
    assert_that!(copy.cell("r0", "c0")).is_equal_to(CellRead::Overlay(&DataCell::Number(9.0)));
}

#[test]
fn apply_unknown_row_is_refused() {
    assert_that!(refused(&mut small(), &set("zz", "a", text("x")))).is_equal_to(DataErrorKind::NoRow);
}

#[test]
fn apply_taken_id_is_refused() {
    assert_that!(refused(&mut small(), &insert_row(Some("r0"), None, &[]))).is_equal_to(DataErrorKind::IdTaken);
    assert_that!(refused(&mut small(), &insert_column(Some("a"), None, &[]))).is_equal_to(DataErrorKind::IdTaken);
}

#[test]
fn apply_invalid_id_with_slash_is_refused() {
    assert_that!(refused(&mut small(), &insert_row(Some("a/b"), None, &[]))).is_equal_to(DataErrorKind::IdInvalid);
}

#[test]
fn apply_missing_id_is_refused() {
    assert_that!(refused(&mut small(), &insert_row(None, None, &[]))).is_equal_to(DataErrorKind::IdMissing);
    assert_that!(refused(&mut small(), &insert_column(None, None, &[]))).is_equal_to(DataErrorKind::IdMissing);
}

#[test]
fn apply_set_cell_null_without_base_removes_overlay_entry() {
    let table = applied(&set("r0", "a", DataCell::Null));
    assert_that!(table.overlay_len()).is_equal_to(3);
    assert_that!(table.cell("r0", "a")).is_equal_to(CellRead::Null);
}

#[test]
fn apply_set_cell_null_with_base_stores_cleared() {
    let mut table = small();
    table.base = Some(base());
    table.apply(&set("r0", "a", DataCell::Null)).unwrap();
    assert_that!(table.overlay_len()).is_equal_to(4);
    assert_that!(table.cell("r0", "a")).is_equal_to(CellRead::Cleared);
}

#[test]
fn apply_set_cell_nan_is_refused() {
    let mut table = small();
    assert_that!(refused(&mut table, &set("r0", "n", DataCell::Number(f64::NAN))))
        .is_equal_to(DataErrorKind::NonFinite);
    assert_that!(table).is_equal_to(small());
}

#[test]
fn apply_set_cell_text_over_bound_is_too_large() {
    let mut table = small();
    let long = text(&"x".repeat(crate::limits::MAX_TEXT_BYTES + 1));
    assert_that!(refused(&mut table, &set("r0", "a", long))).is_equal_to(DataErrorKind::TooLarge);
    assert_that!(table).is_equal_to(small());
}

#[test]
fn rename_column_to_its_own_name_is_noop() {
    let table = applied(&TableOp::RenameColumn { column: "a".to_owned(), name: "Name a".to_owned() });
    assert_that!(table).is_equal_to(small());
}

#[test]
fn insert_row_after_unknown_row_is_refused() {
    assert_that!(refused(&mut small(), &insert_row(Some("r9"), Some("zz"), &[]))).is_equal_to(DataErrorKind::NoRow);
}

#[test]
fn insert_row_after_none_goes_first() {
    let table = applied(&insert_row(Some("r9"), None, &[("a", text("z")), ("n", DataCell::Null)]));
    assert_that!(table.rows.get(0)).is_equal_to(Some("r9"));
    assert_that!(table.cell("r9", "a")).is_equal_to(CellRead::Overlay(&text("z")));
    assert_that!(table.cell("r9", "n")).is_equal_to(CellRead::Null);
}

#[test]
fn insert_column_is_typed_from_its_cells_after_the_given_column() {
    let table = applied(&insert_column(Some("z"), Some("a"), &[("r0", DataCell::Bool(true))]));
    assert_that!(table.columns[1].id.as_str()).is_equal_to("z");
    assert_that!(table.columns[1].ty).is_equal_to(ColumnType::Bool);
}

#[test]
fn insert_column_with_cells_for_unknown_row_is_refused() {
    let op = insert_column(Some("z"), None, &[("zz", DataCell::Number(1.0))]);
    assert_that!(refused(&mut small(), &op)).is_equal_to(DataErrorKind::NoRow);
}

#[test]
fn remove_row_drops_its_overlay_entries() {
    let table = applied(&TableOp::RemoveRow { row: "r0".to_owned() });
    assert_that!(table.cells.keys().all(|key| key.row != "r0")).is_true();
    assert_that!(table.overlay_len()).is_equal_to(2);
}

#[test]
fn remove_row_keeps_entries_of_rows_sharing_its_prefix() {
    let mut table = sized(12, 1);
    for row in ["r1", "r10", "r11"] {
        table.cells.insert(key(row, "c0"), DataCell::Number(1.0));
    }
    table.apply(&TableOp::RemoveRow { row: "r1".to_owned() }).unwrap();
    assert_that!(table.cells.keys().map(|k| k.row.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["r10", "r11"]);
}

#[test]
fn edits_on_a_full_table_stay_logarithmic() {
    // A full table: 65,536 rows, an overlay of 200,000 entries. With linear row lookups and
    // overlay scans this batch costs about 10^9 steps (tens of seconds in debug).
    let columns = MAX_OVERLAY.div_ceil(MAX_ROWS);
    let mut table = sized(MAX_ROWS, columns);
    table.cells = (0..MAX_OVERLAY)
        .map(|i| (key(&format!("r{}", i % MAX_ROWS), &format!("c{}", i / MAX_ROWS)), DataCell::Number(1.0)))
        .collect();
    assert_that!(table.overlay_len()).is_equal_to(MAX_OVERLAY);
    let started = std::time::Instant::now();
    for i in 0..1_000 {
        table.apply(&TableOp::RemoveRow { row: format!("r{}", MAX_ROWS - 1 - i * 7) }).unwrap();
    }
    for i in 0..10_000 {
        table.apply(&set(&format!("r{}", (i * 13) % 50_000), "c0", DataCell::Number(2.0))).unwrap();
    }
    assert_that!(table.rows.len()).is_equal_to(MAX_ROWS - 1_000);
    assert_that!(started.elapsed()).is_less_than(std::time::Duration::from_secs(5));
}

#[test]
fn remove_column_drops_its_overlay_entries() {
    let table = applied(&TableOp::RemoveColumn { column: "a".to_owned() });
    assert_that!(table.cells.keys().all(|key| key.column != "a")).is_true();
    assert_that!(table.overlay_len()).is_equal_to(2);
}

/// A table of `rows` rows whose overlay holds `entries` cells of one number column.
fn with_overlay(entries: usize) -> Table {
    let mut table = sized(entries + 1, 1);
    table.cells = (0..entries).map(|i| (crate::table::key(&format!("r{i}"), "c0"), DataCell::Number(1.0))).collect();
    table
}

#[rstest]
#[case::rows_at_bound(sized(MAX_ROWS - 1, 1), insert_row(Some("new"), None, &[]), None)]
#[case::rows_over_bound(sized(MAX_ROWS, 1), insert_row(Some("new"), None, &[]), Some(DataErrorKind::TooLarge))]
#[case::columns_at_bound(sized(1, MAX_COLUMNS - 1), insert_column(Some("new"), None, &[]), None)]
#[case::columns_over_bound(sized(1, MAX_COLUMNS), insert_column(Some("new"), None, &[]), Some(DataErrorKind::TooLarge))]
#[case::overlay_at_bound(with_overlay(MAX_OVERLAY - 1), set(&format!("r{}", MAX_OVERLAY - 1), "c0", DataCell::Number(2.0)), None)]
#[case::overlay_over_bound(with_overlay(MAX_OVERLAY), set(&format!("r{MAX_OVERLAY}"), "c0", DataCell::Number(2.0)), Some(DataErrorKind::TooLarge))]
fn apply_growing_op_is_refused_past_a_bound(
    #[case] mut table: Table,
    #[case] op: TableOp,
    #[case] expected: Option<DataErrorKind>,
) {
    assert_that!(table.apply(&op).err().map(|e| e.kind())).is_equal_to(expected);
}

#[test]
fn apply_set_cell_over_existing_entry_at_overlay_bound_succeeds() {
    let mut table = with_overlay(MAX_OVERLAY);
    assert_that!(table.apply(&set("r0", "c0", DataCell::Number(3.0)))).is_ok();
}

#[test]
fn apply_remove_row_succeeds_over_bound() {
    let mut table = sized(MAX_ROWS + 1, 1);
    table.apply(&TableOp::RemoveRow { row: "r0".to_owned() }).unwrap();
    assert_that!(table.rows.len()).is_equal_to(MAX_ROWS);
}

#[test]
fn replace_sets_new_generation_and_drops_rows_absent_from_new_table() {
    let table = applied(&replace_without_r1());
    assert_that!(table.generation.as_str()).is_equal_to("g2");
    assert_that!(table.rows.iter().collect::<Vec<_>>()).is_equal_to(vec!["r0"]);
    assert_that!(table.cells.keys().all(|key: &CellKey| key.row == "r0")).is_true();
    assert_that!(table.base).is_none();
}

#[test]
fn replace_with_non_finite_cell_is_refused() {
    let mut new = NewTable::from(&small());
    new.generation = Some("g2".to_owned());
    new.rows[0].cells = vec![text("x"), DataCell::Number(f64::INFINITY)];
    assert_that!(refused(&mut small(), &TableOp::Replace { table: new })).is_equal_to(DataErrorKind::NonFinite);
}

#[test]
fn table_op_fill_ids_draws_unused_ids_and_keeps_given_ones() {
    let table = small();
    let mut ids = counter();
    let mut op = insert_row(None, None, &[]);
    op.fill_ids(&table, &GivenIds::default(), &mut ids);
    let mut given = insert_row(Some("mine"), None, &[]);
    given.fill_ids(&table, &GivenIds::default(), &mut ids);
    let mut replace = TableOp::Replace { table: NewTable::from(&table) };
    replace.fill_ids(&table, &GivenIds::default(), &mut ids);
    assert_that!(op).is_equal_to(insert_row(Some("id1"), None, &[]));
    assert_that!(given).is_equal_to(insert_row(Some("mine"), None, &[]));
    let TableOp::Replace { table: new } = replace else { panic!("not a replace") };
    assert_that!(new.generation).is_equal_to(Some("id2".to_owned()));
}

#[test]
fn table_op_cell_count_counts_written_cells() {
    let cells: BTreeMap<String, DataCell> = [("a".to_owned(), DataCell::Null)].into();
    assert_that!(set("r0", "a", DataCell::Null).cell_count()).is_equal_to(1);
    assert_that!(TableOp::InsertRow { id: None, after: None, cells }.cell_count()).is_equal_to(1);
    assert_that!(TableOp::RemoveRow { row: "r0".to_owned() }.cell_count()).is_equal_to(0);
    assert_that!(TableOp::Replace { table: NewTable::from(&small()) }.cell_count()).is_equal_to(4);
}

#[test]
fn table_op_json_uses_snake_case_tags() {
    let op: TableOp = serde_json::from_str(r#"{"op":"set_cell","row":"r0","column":"a","value":"x"}"#).unwrap();
    assert_that!(op).is_equal_to(set("r0", "a", text("x")));
    assert_that!(serde_json::from_str::<TableOp>(r#"{"op":"remove_row","row":"r0","extra":1}"#)).is_err();
}

#[test]
fn table_op_fill_ids_skips_ids_given_elsewhere_in_the_batch() {
    // `small` uses none of the counter's first ids for its rows and columns.
    let table = small();
    let column_op = |id: Option<&str>| TableOp::InsertColumn {
        id: id.map(str::to_owned),
        after: None,
        name: "x".into(),
        cells: BTreeMap::new(),
    };
    let mut given = GivenIds::default();
    given.record(&insert_row(Some("id1"), None, &[]));
    given.record(&column_op(Some("id1")));
    let mut row = insert_row(None, None, &[]);
    row.fill_ids(&table, &given, &mut counter());
    let mut column = column_op(None);
    column.fill_ids(&table, &given, &mut counter());
    assert_that!(row).is_equal_to(insert_row(Some("id2"), None, &[]));
    assert_that!(column).is_equal_to(column_op(Some("id2")));
}
