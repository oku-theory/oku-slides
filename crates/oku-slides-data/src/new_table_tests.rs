use std::collections::BTreeSet;

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::limits::{MAX_COLUMNS, MAX_OVERLAY, MAX_ROWS};
use crate::test_support::{base, counter, fill, new_table, small};
use crate::{CellRead, DataErrorKind};

fn text(s: &str) -> DataCell {
    DataCell::Text(s.to_owned())
}

fn num(n: f64) -> DataCell {
    DataCell::Number(n)
}

fn dataset(source: Vec<Vec<DataCell>>) -> Dataset {
    Dataset { source, ids: Vec::new(), columns: Vec::new() }
}

/// The stored table of a dataset, ids drawn by [`counter`].
fn stored(ds: &Dataset) -> Table {
    let mut table = NewTable::from_dataset(ds).table;
    fill(&mut table);
    table.into_table().unwrap().0
}

#[rstest]
#[case::empty(vec![], 0, 0, 0)]
#[case::header_only(vec![vec![text("A"), text("B")]], 2, 0, 0)]
#[case::short_row_padded(vec![vec![text("A"), text("B")], vec![num(1.0)]], 2, 1, 0)]
#[case::long_row_cut(vec![vec![text("A")], vec![num(1.0), num(2.0)]], 1, 1, 1)]
#[case::duplicate_headers(vec![vec![text("A"), text("A")], vec![num(1.0), num(2.0)]], 2, 1, 0)]
fn from_dataset_reads_header_and_rows(
    #[case] source: Vec<Vec<DataCell>>,
    #[case] columns: usize,
    #[case] rows: usize,
    #[case] cut_rows: usize,
) {
    let read = NewTable::from_dataset(&dataset(source.clone()));
    assert_that!(read.cut_rows).is_equal_to(cut_rows);
    assert_that!(read.table.columns.len()).is_equal_to(columns);
    assert_that!(read.table.rows.iter().all(|row| row.cells.len() == columns)).is_true();
    let table = stored(&dataset(source));
    assert_that!(table.rows.len()).is_equal_to(rows);
    let ids: BTreeSet<&str> = table.columns.iter().map(|c| c.id.as_str()).collect();
    assert_that!(ids.len()).is_equal_to(columns);
}

#[test]
fn mixed_column_becomes_text_in_display_form() {
    let table = stored(&dataset(vec![vec![text("A")], vec![num(1.5)], vec![text("a")], vec![DataCell::Bool(true)]]));
    assert_that!(table.columns[0].ty).is_equal_to(ColumnType::Text);
    let cells: Vec<DataCell> = table.rows.iter().map(|r| table.cells[&key(r, &table.columns[0].id)].clone()).collect();
    assert_that!(cells).is_equal_to(vec![text("1.5"), text("a"), text("true")]);
}

#[test]
fn from_dataset_nan_becomes_null_and_is_counted() {
    let read = NewTable::from_dataset(&dataset(vec![
        vec![text("A"), text("B")],
        vec![num(f64::NAN), num(1.0)],
        vec![num(2.0), num(f64::NEG_INFINITY)],
    ]));
    assert_that!(read.non_finite).is_equal_to(2);
    assert_that!(read.table.rows[0].cells[0].clone()).is_equal_to(DataCell::Null);
    assert_that!(read.table.columns[0].ty).is_equal_to(Some(ColumnType::Number));
}

#[test]
fn from_dataset_ignores_dataset_ids() {
    let mut ds = dataset(vec![vec![text("A")], vec![num(1.0)]]);
    ds.ids = vec![String::new(), "kept".to_owned()];
    ds.columns = vec!["col".to_owned()];
    let read = NewTable::from_dataset(&ds);
    assert_that!(read.table.rows[0].id).is_none();
    assert_that!(read.table.columns[0].id).is_none();
    assert_that!(read.table.generation).is_none();
}

#[test]
fn from_dataset_cleans_header_names() {
    let read = NewTable::from_dataset(&dataset(vec![vec![text("A\u{202E}B"), num(2.0)]]));
    let names: Vec<&str> = read.table.columns.iter().map(|c| c.name.as_str()).collect();
    assert_that!(names).is_equal_to(vec!["AB", "2"]);
}

#[test]
fn new_table_without_ids_parses_then_fill_ids_draws_unique_ids_and_generation() {
    let json = r#"{"columns":[{"name":"A"},{"name":"B","type":"text"}],"rows":[{"cells":[1,"x"]},{"cells":[2]}]}"#;
    let mut table: NewTable = serde_json::from_str(json).unwrap();
    fill(&mut table);
    let (table, non_finite) = table.into_table().unwrap();
    let ids: BTreeSet<&str> = table
        .columns
        .iter()
        .map(|c| c.id.as_str())
        .chain(table.rows.iter())
        .chain([table.generation.as_str()])
        .collect();
    assert_that!(ids.len()).is_equal_to(5);
    assert_that!(non_finite).is_equal_to(0);
    assert_that!(table.columns[0].ty).is_equal_to(ColumnType::Number);
}

#[rstest]
#[case::generation(|t: &mut NewTable| t.generation = None)]
#[case::column(|t: &mut NewTable| t.columns[0].id = None)]
#[case::row(|t: &mut NewTable| t.rows[0].id = None)]
fn into_table_with_missing_id_is_refused(#[case] change: fn(&mut NewTable)) {
    let mut table = new_table(&[("a", None)], vec![vec![num(1.0)]]);
    change(&mut table);
    assert_that!(table.into_table().map(drop).map_err(|e| e.kind())).is_err_containing(DataErrorKind::IdMissing);
}

#[test]
fn into_table_with_repeated_or_invalid_id_is_refused() {
    let mut table = new_table(&[("a", None), ("b", None)], vec![]);
    table.columns[1].id = Some("a".to_owned());
    assert_that!(table.into_table().map(drop).map_err(|e| e.kind())).is_err_containing(DataErrorKind::IdTaken);
    let mut table = new_table(&[("a", None)], vec![vec![], vec![]]);
    table.rows[1].id = Some("a/b".to_owned());
    assert_that!(table.into_table().map(drop).map_err(|e| e.kind())).is_err_containing(DataErrorKind::IdInvalid);
}

#[test]
fn into_table_with_misfit_cell_is_refused() {
    let table = new_table(&[("a", Some(ColumnType::Number))], vec![vec![text("x")]]);
    assert_that!(table.into_table().map(drop).map_err(|e| e.kind())).is_err_containing(DataErrorKind::CellType);
}

#[test]
fn fill_ids_with_keep_reuses_column_ids_by_name_and_row_ids_by_position() {
    let keep = small();
    let mut table = NewTable::from_dataset(&dataset(vec![
        vec![text("Name n"), text("Other"), text("Name a")],
        vec![num(1.0), num(2.0), text("x")],
        vec![num(1.0), num(2.0), text("x")],
        vec![num(1.0), num(2.0), text("x")],
    ]))
    .table;
    let mut ids = counter();
    table.fill_ids(Some(&keep), &GivenIds::default(), &mut ids);
    let columns: Vec<Option<String>> = table.columns.iter().map(|c| c.id.clone()).collect();
    let rows: Vec<Option<String>> = table.rows.iter().map(|r| r.id.clone()).collect();
    assert_that!(columns).is_equal_to(vec![Some("n".to_owned()), Some("id2".to_owned()), Some("a".to_owned())]);
    assert_that!(rows).is_equal_to(vec![Some("r0".to_owned()), Some("r1".to_owned()), Some("id3".to_owned())]);
    assert_that!(table.generation).is_equal_to(Some("id1".to_owned()));
}

#[test]
fn fill_ids_does_not_reuse_ids_drawn_earlier_in_the_call() {
    let mut table = NewTable::from_dataset(&dataset(vec![vec![text("A"), text("B")], vec![num(1.0), num(2.0)]])).table;
    let mut proposals = 0_u32;
    let mut ids = |taken: &dyn Fn(&str) -> bool| {
        for candidate in ["same", "other", "third", "fourth", "fifth"] {
            proposals += 1;
            if !taken(candidate) {
                return candidate.to_owned();
            }
        }
        panic!("ran out of ids");
    };
    table.fill_ids(None, &GivenIds::default(), &mut ids);
    let columns: BTreeSet<Option<String>> = table.columns.iter().map(|c| c.id.clone()).collect();
    assert_that!(columns.len()).is_equal_to(2);
    assert_that!(proposals).is_greater_than(3);
}

#[test]
fn new_row_longer_than_columns_is_refused() {
    let table = new_table(&[("a", None)], vec![vec![num(1.0), num(2.0)]]);
    assert_that!(table.into_table().map(drop).map_err(|e| e.kind())).is_err_containing(DataErrorKind::RowWidth);
}

#[test]
fn from_table_copies_ids_base_and_overlay() {
    let mut table = small();
    table.base = Some(base());
    table.cells.insert(key("r0", "a"), DataCell::Null);
    let new = NewTable::from(&table);
    assert_that!(new.generation).is_none();
    assert_that!(new.columns.iter().map(|c| c.id.clone()).collect::<Vec<_>>())
        .is_equal_to(vec![Some("a".to_owned()), Some("n".to_owned())]);
    assert_that!(new.rows.iter().map(|r| r.id.clone()).collect::<Vec<_>>())
        .is_equal_to(vec![Some("r0".to_owned()), Some("r1".to_owned())]);
    assert_that!(new.base.clone()).is_equal_to(table.base.clone());
    assert_that!(new.overlay.clone()).is_equal_to(table.cells.clone());
    let (back, _) = NewTable { generation: Some("g1".to_owned()), ..new }.into_table().unwrap();
    assert_that!(back.cell("r0", "a")).is_equal_to(CellRead::Cleared);
    assert_that!(back).is_equal_to(table);
}

/// A table of `rows` rows, `columns` columns and `cells` row cells, ids drawn.
fn shaped(rows: usize, columns: usize, cells: usize) -> NewTable {
    NewTable {
        generation: None,
        columns: (0..columns).map(|_| NewColumn { id: None, name: "C".to_owned(), ty: None }).collect(),
        rows: (0..rows).map(|_| NewRow { id: None, cells: Vec::new() }).collect(),
        base: None,
        overlay: imbl::OrdMap::new(),
    }
    .with_cells(cells)
}

impl NewTable {
    /// Spreads `cells` empty cells over the first row (the bound counts cells, not their width).
    fn with_cells(mut self, cells: usize) -> NewTable {
        if let Some(row) = self.rows.first_mut() {
            row.cells = vec![DataCell::Null; cells];
        }
        self
    }
}

#[rstest]
#[case::rows_at_bound(shaped(MAX_ROWS, 1, 0), None)]
#[case::rows_over_bound(shaped(MAX_ROWS + 1, 1, 0), Some(DataErrorKind::TooLarge))]
#[case::columns_at_bound(shaped(1, MAX_COLUMNS, 0), None)]
#[case::columns_over_bound(shaped(1, MAX_COLUMNS + 1, 0), Some(DataErrorKind::TooLarge))]
#[case::cells_at_bound(shaped(1, 1, MAX_OVERLAY), None)]
#[case::cells_over_bound(shaped(1, 1, MAX_OVERLAY + 1), Some(DataErrorKind::TooLarge))]
fn check_bounds_refuses_past_a_bound(#[case] table: NewTable, #[case] expected: Option<DataErrorKind>) {
    assert_that!(table.check_bounds().err().map(|e| e.kind())).is_equal_to(expected);
}

#[test]
fn into_table_with_base_stores_a_non_finite_overlay_cell_as_cleared() {
    let mut table = small();
    table.base = Some(base());
    let mut new = NewTable { generation: Some("g2".to_owned()), ..NewTable::from(&table) };
    new.overlay.insert(key("r0", "n"), DataCell::Number(f64::NAN));
    let (back, non_finite) = new.into_table().unwrap();
    assert_that!(non_finite).is_equal_to(1);
    assert_that!(back.cell("r0", "n")).is_equal_to(CellRead::Cleared);
}

#[test]
fn into_table_without_base_drops_a_non_finite_overlay_cell() {
    let mut new = NewTable { generation: Some("g2".to_owned()), ..NewTable::from(&small()) };
    new.overlay.insert(key("r0", "n"), DataCell::Number(f64::INFINITY));
    let (back, non_finite) = new.into_table().unwrap();
    assert_that!(non_finite).is_equal_to(1);
    assert_that!(back.cell("r0", "n")).is_equal_to(CellRead::Null);
}

#[test]
fn into_table_with_base_and_row_cells_is_refused() {
    let mut table = small();
    table.base = Some(base());
    let mut new = NewTable { generation: Some("g2".to_owned()), ..NewTable::from(&table) };
    new.rows[0].cells = vec![DataCell::Number(1.0)];
    assert_that!(new.into_table().unwrap_err().kind()).is_equal_to(DataErrorKind::BaseRowCells);
}

/// Each row's weight is what it stores: its non-empty cells, and their text bytes, numbers and
/// booleans of a Text column counted in display form → the sums equal the stored table's.
#[test]
fn row_loads_sum_to_the_stored_overlay_and_text() {
    let ds = dataset(vec![
        vec![text("Nom"), text("Valeur"), text("Note")],
        vec![text("été"), num(1.5), num(12.0)],
        vec![DataCell::Null, num(2.0), text("bon")],
        vec![text("b"), DataCell::Null, DataCell::Bool(true)],
    ]);
    let table = NewTable::from_dataset(&ds).table;
    let loads = table.row_loads();
    assert_that!(loads).is_equal_to(vec![
        RowLoad { cells: 3, text: 7 },
        RowLoad { cells: 2, text: 3 },
        RowLoad { cells: 2, text: 5 },
    ]);
    let stored = stored(&ds);
    let names: usize = stored.columns.iter().map(|c| c.name.len()).sum();
    assert_that!(loads.iter().map(|l| l.cells).sum::<usize>()).is_equal_to(stored.overlay_len());
    assert_that!(loads.iter().map(|l| l.text).sum::<usize>() + names).is_equal_to(stored.text_bytes());
}

/// A column left untyped counts as Text, the heaviest form its cells can take.
#[test]
fn row_loads_count_an_untyped_column_as_text() {
    let mut table = NewTable::from_dataset(&dataset(vec![vec![text("n")], vec![num(125.0)]])).table;
    table.columns[0].ty = None;
    assert_that!(table.row_loads()).is_equal_to(vec![RowLoad { cells: 1, text: 3 }]);
}
