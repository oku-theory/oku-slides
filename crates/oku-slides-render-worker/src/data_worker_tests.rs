use oku_slides_data::test_support::{based_table, set_cell, sized};
use oku_slides_data::{Data, DataCell, DataErrorKind, Source, Table};
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::bases::BaseStore;
use oku_slides_platform::config::RenderSettings;
use oku_slides_platform::storage::Storage;
use speculoos::prelude::*;
use tempfile::TempDir;

use super::DataWorker;

fn settings() -> RenderSettings {
    RenderSettings {
        timeout_ms: 10_000,
        probe_budget_ms: 10_000,
        memory_mib: 4096,
        max_frame_mib: 64,
        max_widget_kib: 256,
        max_widgets: 64,
        max_guide_chars: 4000,
        data_timeout_ms: 120_000,
        data_memory_mib: 4096,
        query_timeout_ms: 30_000,
        query_memory_mib: 4096,
    }
}

fn data(table: Table) -> Data {
    Data { name: "sales".into(), label: None, inline: false, source: Source::Manual(table) }
}

/// A worker over a fresh store holding a based table of `rows` × 2 (`c0`, `c1`).
fn setup(rows: usize) -> (TempDir, DataWorker, BaseStore, Data) {
    let dir = tempfile::tempdir().unwrap();
    let store = BaseStore::new(Storage::local().unwrap(), dir.path(), 1 << 30).unwrap();
    let (table, file) = based_table(rows, 2);
    let id = store.adopt(file.path(), 1 << 30).unwrap();
    assert_that!(Some(&id)).is_equal_to(table.base.as_ref().map(|b| &b.id));
    let worker = DataWorker::new(RenderLaunch::InProcess, &settings(), store.clone());
    (dir, worker, store, data(table))
}

fn number(cell: &DataCell) -> f64 {
    match cell {
        DataCell::Number(n) => *n,
        other => panic!("not a number: {other:?}"),
    }
}

#[test]
fn data_worker_read_based_rows_returns_them() {
    let (_dir, worker, _store, data) = setup(5);
    let read = worker.read(&data, None, 1..3, 100).unwrap().unwrap();
    // Header, then rows 1 and 2.
    assert_that!(read.dataset.source).has_length(3);
    assert_that!(number(&read.dataset.source[1][1])).is_equal_to(1001.0);
    assert_that!(read.misfit).is_empty();
}

#[test]
fn data_worker_read_without_base_is_refused() {
    let (_dir, worker, _store, _) = setup(1);
    let refused = worker.read(&data(sized(2, 2)), None, 0..2, 100).unwrap();
    assert_that!(refused.map_err(|e| e.kind())).is_err_containing(DataErrorKind::BaseInvalid);
}

#[test]
fn data_worker_read_of_base_missing_from_store_is_refused() {
    let (_dir, worker, store, data) = setup(2);
    std::fs::remove_file(store.path(&data.table().unwrap().base.as_ref().unwrap().id)).unwrap();
    let refused = worker.read(&data, None, 0..2, 100).unwrap();
    assert_that!(refused.map_err(|e| e.kind())).is_err_containing(DataErrorKind::BaseInvalid);
}

#[test]
fn data_worker_compact_folds_overlay_into_stored_base() {
    let (_dir, worker, store, mut data) = setup(4);
    set_cell(data.table_mut().unwrap(), "r2", "c0", DataCell::Number(99.0));
    let base = worker.compact(&data, 1 << 30).unwrap().unwrap();
    assert_that!(store.contains(&base.id)).is_ok_containing(true);
    assert_that!(base.rows).is_equal_to(4);
    let mut folded = data.clone();
    folded.table_mut().unwrap().base = Some(base);
    folded.table_mut().unwrap().cells.clear();
    let read = worker.read(&folded, None, 2..3, 100).unwrap().unwrap();
    assert_that!(number(&read.dataset.source[1][0])).is_equal_to(99.0);
}

/// The names of the files directly in `dir`.
fn files_in(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> =
        std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

/// → a refused compaction leaves no output file behind (t-1690).
#[test]
fn data_worker_compact_refused_leaves_no_output_file() {
    let (_dir, worker, store, data) = setup(2);
    std::fs::remove_file(store.path(&data.table().unwrap().base.as_ref().unwrap().id)).unwrap();
    let before = files_in(store.dir());
    let refused = worker.compact(&data, 1 << 30).unwrap();
    assert_that!(refused.map_err(|e| e.kind())).is_err_containing(DataErrorKind::BaseInvalid);
    assert_that!(files_in(store.dir())).is_equal_to(before);
}

/// → a compaction leaves only the adopted base behind, not its output file.
#[test]
fn data_worker_compact_leaves_only_the_adopted_base() {
    let (_dir, worker, store, mut data) = setup(2);
    set_cell(data.table_mut().unwrap(), "r1", "c0", DataCell::Number(5.0));
    let mut before = files_in(store.dir());
    let base = worker.compact(&data, 1 << 30).unwrap().unwrap();
    before.push(format!("{}.parquet", base.id.hex()));
    before.sort();
    assert_that!(files_in(store.dir())).is_equal_to(before);
}

#[test]
fn run_query_request_writes_base_in_store() {
    let (_dir, worker, store, data) = setup(3);
    let query = oku_slides_data::sql::parse_query("SELECT C0 + C1 AS s FROM sales ORDER BY s").unwrap();
    let inputs = vec![("sales".to_owned(), std::sync::Arc::new(data))];

    let out = worker.run_query(&query.for_engine().unwrap(), &inputs).unwrap().unwrap();

    assert_that!(store.contains(&out.base.id)).is_ok_containing(true);
    assert_that!(out.base.rows).is_equal_to(3);
    assert_that!(out.columns.iter().map(|c| c.id.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["s"]);
}

#[test]
fn run_query_refused_by_the_engine_is_a_data_refusal() {
    let (_dir, worker, _store, data) = setup(1);
    let query = oku_slides_data::sql::parse_query("SELECT nope FROM sales").unwrap();
    let inputs = vec![("sales".to_owned(), std::sync::Arc::new(data))];

    let refused = worker.run_query(&query.for_engine().unwrap(), &inputs).unwrap();

    assert_that!(refused.map(|_| ()).map_err(|e| e.kind())).is_err_containing(DataErrorKind::QueryRun);
}

#[test]
fn engine_version_names_polars_protocol_and_whitelist() {
    assert_that!(super::engine_version()).is_equal_to(format!(
        "polars-0.53.0/protocol-{}/whitelist-{}",
        crate::protocol::PROTOCOL_VERSION,
        oku_slides_data::sql::WHITELIST_VERSION
    ));
}
