use std::io::{Cursor, Read, Write};

use oku_slides_data::{BaseColumn, BaseRef, Column, ColumnType};
use speculoos::prelude::*;

use super::{CachedResult, MAX_RESULT_ENTRY};
use crate::error::SourcesErrorKind;
use crate::sources_bases::sources_bases_tests::{Store, id_of};
use crate::{Deck, SourceBundle, SourceLimits};

const MAX: SourceLimits = SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 26, compact_after_secs: 0 };

fn hex(c: char) -> String {
    c.to_string().repeat(64)
}

/// A result keyed `key` over the base `bytes`, with one number column `a`.
fn result(key: &str, bytes: &[u8]) -> CachedResult {
    let base = BaseRef {
        id: id_of(bytes),
        bytes: bytes.len() as u64,
        rows: 1,
        columns: vec![BaseColumn { id: "a".to_owned(), ty: ColumnType::Number, text_bytes: 0 }],
    };
    let columns = vec![Column { id: "a".to_owned(), name: "a".to_owned(), ty: ColumnType::Number }];
    CachedResult { key: key.to_owned(), columns, base }
}

/// A store holding `bases`.
fn store(bases: &[&[u8]]) -> Store {
    let store = Store::default();
    for bytes in bases {
        store.0.borrow_mut().insert(id_of(bytes), bytes.to_vec());
    }
    store
}

/// An empty deck caching `results`, zipped from `from` with a base budget of `budget`.
fn zipped(results: Vec<CachedResult>, from: &Store, budget: u64) -> Vec<u8> {
    let mut b = SourceBundle::new(Deck::default(), crate::test_registry::shared()).unwrap();
    b.results = results.into_iter().map(|r| (r.key.clone(), r)).collect();
    b.write_zip_within(true, Some(from), budget).unwrap()
}

fn opened(zip: &[u8]) -> (SourceBundle, Store) {
    let to = Store::default();
    let (back, _) = SourceBundle::from_zip_bases(zip, MAX, crate::test_registry::shared(), Some(&to)).unwrap();
    (back, to)
}

fn cached_keys(b: &SourceBundle) -> Vec<String> {
    b.results.keys().cloned().collect()
}

/// `zip` with entry `name` replaced by `bytes` (added when absent).
fn rewritten(zip: &[u8], name: &str, bytes: &[u8]) -> Vec<u8> {
    let mut za = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
    let mut out = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(Cursor::new(&mut out));
        let options = zip::write::SimpleFileOptions::default();
        for i in 0..za.len() {
            let mut f = za.by_index(i).unwrap();
            let entry = f.name().unwrap().into_owned();
            if entry == name {
                continue;
            }
            let mut data = Vec::new();
            f.read_to_end(&mut data).unwrap();
            zw.start_file(entry, options).unwrap();
            zw.write_all(&data).unwrap();
        }
        zw.start_file(name, options).unwrap();
        zw.write_all(bytes).unwrap();
        zw.finish().unwrap();
    }
    out
}

#[test]
fn cached_result_round_trips_with_its_base() {
    let from = store(&[b"one"]);
    let (back, to) = opened(&zipped(vec![result(&hex('a'), b"one")], &from, u64::MAX));
    assert_that!(back.results.get(&hex('a'))).is_equal_to(Some(&result(&hex('a'), b"one")));
    assert_that!(to.0.borrow().contains_key(&id_of(b"one"))).is_true();
}

#[test]
fn cache_over_base_budget_is_skipped_and_recomputed() {
    let from = store(&[b"one", b"three"]);
    let results = vec![result(&hex('a'), b"one"), result(&hex('b'), b"three")];
    // Room for the first base only: the second result is not written, so it computes again.
    let (back, to) = opened(&zipped(results, &from, 4));
    assert_that!(cached_keys(&back)).is_equal_to(vec![hex('a')]);
    assert_that!(to.0.borrow().contains_key(&id_of(b"three"))).is_false();
}

#[test]
fn two_results_over_one_base_write_it_once() {
    let from = store(&[b"one"]);
    let results = vec![result(&hex('a'), b"one"), result(&hex('b'), b"one")];
    let (back, _) = opened(&zipped(results, &from, 3));
    assert_that!(cached_keys(&back)).is_equal_to(vec![hex('a'), hex('b')]);
}

#[test]
fn result_with_unreadable_base_is_not_cached() {
    let (back, _) = opened(&zipped(vec![result(&hex('a'), b"one")], &Store::default(), u64::MAX));
    assert_that!(back.results.is_empty()).is_true();
}

/// Entries that do not check: each is dropped, the deck still opens.
#[rstest::rstest]
#[case::key_unlike_name(|r: &mut CachedResult| r.key = hex('b'))]
#[case::size_unlike_base(|r: &mut CachedResult| r.base.bytes += 1)]
#[case::base_absent(|r: &mut CachedResult| r.base.id = id_of(b"other"))]
#[case::column_unlike_base(|r: &mut CachedResult| r.columns[0].ty = ColumnType::Text)]
#[case::column_name_unlike_id(|r: &mut CachedResult| r.columns[0].name = "b".to_owned())]
#[case::column_id_invalid(|r: &mut CachedResult| {
    r.columns[0].id = "a b".to_owned();
    r.base.columns[0].id = "a b".to_owned();
})]
#[case::rows_past_bound(|r: &mut CachedResult| r.base.rows = u32::MAX)]
#[case::column_missing(|r: &mut CachedResult| r.columns.clear())]
fn tampered_entry_is_dropped(#[case] tamper: fn(&mut CachedResult)) {
    let from = store(&[b"one"]);
    let zip = zipped(vec![result(&hex('a'), b"one")], &from, u64::MAX);
    let mut bad = result(&hex('a'), b"one");
    tamper(&mut bad);
    let zip = rewritten(&zip, &format!("data/{}.json", hex('a')), &serde_json::to_vec(&bad).unwrap());
    let (back, _) = opened(&zip);
    assert_that!(back.results.is_empty()).is_true();
}

#[rstest::rstest]
#[case::bad_name("data/xyz.json", b"{}".to_vec())]
#[case::not_json("data/aaaa.json", b"not json".to_vec())]
#[case::unknown_field(
    "data/aaaa.json",
    br#"{"key":"x","columns":[],"base":{"id":"x","bytes":0,"rows":0,"columns":[]},"extra":1}"#.to_vec()
)]
#[case::too_large("data/aaaa.json", vec![b' '; MAX_RESULT_ENTRY + 1])]
fn malformed_entry_is_dropped(#[case] name: &str, #[case] bytes: Vec<u8>) {
    let from = store(&[b"one"]);
    let zip = zipped(vec![result(&hex('c'), b"one")], &from, u64::MAX);
    let name = name.replace("aaaa", &hex('a'));
    let (back, _) = opened(&rewritten(&zip, &name, &bytes));
    assert_that!(cached_keys(&back)).is_equal_to(vec![hex('c')]);
}

#[test]
fn cache_without_store_is_dropped() {
    let from = store(&[b"one"]);
    let zip = zipped(vec![result(&hex('a'), b"one")], &from, u64::MAX);
    let (back, _) = SourceBundle::from_zip(&zip, MAX, crate::test_registry::shared()).unwrap();
    assert_that!(back.results.is_empty()).is_true();
}

#[test]
fn old_sources_version_15_is_refused() {
    let zip = zipped(Vec::new(), &Store::default(), u64::MAX);
    let manifest = br#"{"format":"oku-slides-sources","version":15}"#;
    let err = SourceBundle::from_zip(&rewritten(&zip, "manifest.json", manifest), MAX, crate::test_registry::shared())
        .map(|_| ())
        .map_err(|e| e.kind());
    assert_that!(err).is_equal_to(Err(SourcesErrorKind::NotSources));
}
