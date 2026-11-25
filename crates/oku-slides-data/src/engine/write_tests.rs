use speculoos::prelude::*;
use tempfile::NamedTempFile;

use super::write_base;
use crate::engine::read_view;
use crate::table::key;
use crate::test_support::{fold, typed};
use crate::{ColumnType, DataCell, DataErrorKind, Table};

fn all(table: &Table) -> Vec<String> {
    table.columns.iter().map(|c| c.id.clone()).collect()
}

#[test]
fn write_then_read_round_trips_every_type() {
    let table = typed();
    let (based, file) = fold(&table);

    let read = read_view(&based, file.reopen().unwrap(), &all(&table), 0..10, 1000).unwrap();

    assert_that!(read.dataset).is_equal_to(table.to_dataset(None, 0..10, 1000).unwrap());
    assert_that!(read.misfit.is_empty()).is_true();
}

#[test]
fn write_records_rows_columns_and_hash() {
    let table = typed();
    let (based, file) = fold(&table);
    let base = based.base.unwrap();
    let bytes = std::fs::read(file.path()).unwrap();

    assert_that!(base.rows).is_equal_to(3);
    assert_that!(base.bytes).is_equal_to(bytes.len() as u64);
    let hash: [u8; 32] = sha2::Digest::finalize(<sha2::Sha256 as sha2::Digest>::new_with_prefix(&bytes)).into();
    assert_that!(base.id).is_equal_to(crate::BaseId::from_sha256(&hash));
    let columns: Vec<(&str, ColumnType, u64)> =
        base.columns.iter().map(|c| (c.id.as_str(), c.ty, c.text_bytes)).collect();
    assert_that!(columns).is_equal_to(vec![
        ("t", ColumnType::Text, 6),
        ("n", ColumnType::Number, 0),
        ("i", ColumnType::Integer, 0),
        ("b", ColumnType::Bool, 0),
        ("d", ColumnType::Date, 20),
    ]);
}

#[test]
fn second_write_folds_overlay_and_drops_hidden_rows_and_columns() {
    let (mut based, file) = fold(&typed());
    based.cells.insert(key("r0", "t"), DataCell::Text("new".into()));
    based.cells.insert(key("r1", "n"), DataCell::Null);
    based.rows.remove(2);
    based.columns.retain(|c| c.id != "b");
    let expected = {
        let read = read_view(&based, file.reopen().unwrap(), &all(&based), 0..10, 1000).unwrap();
        read.dataset
    };

    let mut out = NamedTempFile::new().unwrap();
    let base = write_base(&based, Some(file.reopen().unwrap()), out.as_file_mut()).unwrap();
    let mut refolded = based.clone();
    refolded.base = Some(base.clone());
    refolded.cells = imbl::OrdMap::new();
    let read = read_view(&refolded, out.reopen().unwrap(), &all(&refolded), 0..10, 1000).unwrap();

    assert_that!(read.dataset).is_equal_to(expected);
    assert_that!(base.rows).is_equal_to(2);
    assert_that!(base.columns.iter().any(|c| c.id == "b")).is_false();
}

#[test]
fn write_without_the_recorded_base_file_is_refused() {
    let (based, _file) = fold(&typed());
    let mut out = NamedTempFile::new().unwrap();

    let error = write_base(&based, None, out.as_file_mut()).unwrap_err();

    assert_that!(error.kind()).is_equal_to(DataErrorKind::BaseInvalid);
}

#[test]
fn write_of_an_overlay_cell_that_does_not_fit_is_refused() {
    let mut table = typed();
    table.cells.insert(key("r0", "n"), DataCell::Text("x".into()));
    let mut out = NamedTempFile::new().unwrap();

    let error = write_base(&table, None, out.as_file_mut()).unwrap_err();

    assert_that!(error.kind()).is_equal_to(DataErrorKind::ColumnType);
}
