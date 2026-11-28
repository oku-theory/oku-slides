use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use oku_slides_core::CachedResult;
use oku_slides_data::ColumnType;
use speculoos::prelude::*;

use super::super::QueryState;
use super::super::derived_tests::{TOP, apply, id, page_hash, ready, rows, set_b, with_query};
use crate::Session;
use crate::test_support::{fresh_dir, session_on};

/// Saves `s` as `name` in `dir`; the file's path.
fn save(s: &mut Session, dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(format!("{name}.oku-slides.pdf"));
    s.save(Some(&path)).unwrap_or_else(|e| panic!("save: {}", e.one_line()));
    path
}

/// The sources zip of the file `pdf`.
fn sources(pdf: &Path) -> Vec<u8> {
    let bytes = std::fs::read(pdf).unwrap_or_else(|e| panic!("{e}"));
    oku_slides_pdf::extract_sources(&bytes)
        .unwrap_or_else(|e| panic!("{e}"))
        .unwrap_or_else(|| panic!("no sources"))
        .zip
}

/// The entries of `zip`, by name.
fn entries(zip: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut za = zip::ZipArchive::new(Cursor::new(zip)).unwrap_or_else(|e| panic!("{e}"));
    (0..za.len())
        .map(|i| {
            let mut f = za.by_index(i).unwrap_or_else(|e| panic!("{e}"));
            let name = f.name().unwrap_or_else(|e| panic!("{e}")).into_owned();
            let mut data = Vec::new();
            f.read_to_end(&mut data).unwrap_or_else(|e| panic!("{e}"));
            (name, data)
        })
        .collect()
}

/// The cache entries of the file `pdf`.
fn cached(pdf: &Path) -> Vec<String> {
    entries(&sources(pdf)).into_iter().map(|(name, _)| name).filter(|n| n.starts_with("data/")).collect()
}

/// `pdf` with its sources zip made of `entries`.
fn rewrite(pdf: &Path, entries: Vec<(String, Vec<u8>)>) {
    let mut zip = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(Cursor::new(&mut zip));
        for (name, data) in entries {
            zw.start_file(name, zip::write::SimpleFileOptions::default()).unwrap_or_else(|e| panic!("{e}"));
            zw.write_all(&data).unwrap_or_else(|e| panic!("{e}"));
        }
        zw.finish().unwrap_or_else(|e| panic!("{e}"));
    }
    let bytes = std::fs::read(pdf).unwrap_or_else(|e| panic!("{e}"));
    let out = oku_slides_pdf::embed_sources(&bytes, &zip).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(pdf, out).unwrap_or_else(|e| panic!("{e}"));
}

/// A session on a fresh data folder, which has none of the saved bases, opening `pdf`.
fn reopened(pdf: &Path) -> (Session, tempfile::TempDir) {
    let data = fresh_dir("oku-slides-derived-cache-");
    let mut s = session_on(data.path());
    s.open(pdf).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    (s, data)
}

fn pending(s: &Session, data: &str) -> bool {
    !matches!(s.query_state(&id(data)), Some(QueryState::Ready { .. }))
}

#[test]
fn save_then_open_renders_query_view_without_recompute() {
    let (mut s, dir) = with_query(TOP);
    s.settle_queries();
    let (key, out) = ready(&s, "q");
    let drawn = page_hash(&s);
    let pdf = save(&mut s, dir.path(), "cached");

    let (mut back, _data) = reopened(&pdf);
    let (back_key, back_out) = ready(&back, "q");
    assert_that!(back_key).is_equal_to(key);
    assert_that!(*back_out).is_equal_to(&*out);
    assert_that!(back.query_jobs().is_empty()).is_true();
    assert_that!(page_hash(&back)).is_equal_to(drawn);
}

#[test]
fn stale_cache_entry_is_recomputed() {
    let (mut s, dir) = with_query(TOP);
    s.settle_queries();
    let old = entries(&sources(&save(&mut s, dir.path(), "old")));
    apply(&mut s, vec![set_b(1, 5.0)]);
    let pdf = save(&mut s, dir.path(), "new");
    // The new file carries the old result: its key names the old inputs.
    let mut zip = entries(&sources(&pdf));
    let have: Vec<String> = zip.iter().map(|(n, _)| n.clone()).collect();
    zip.extend(old.into_iter().filter(|(n, _)| n.starts_with("data/") || !have.contains(n)));
    rewrite(&pdf, zip);
    assert_that!(cached(&pdf)).has_length(1);

    let (mut back, _data) = reopened(&pdf);
    assert_that!(pending(&back, "q")).is_true();
    back.settle_queries();
    assert_that!(rows(&back, "q")).is_equal_to(2);
}

#[test]
fn tampered_cache_entry_is_recomputed() {
    let (mut s, dir) = with_query(TOP);
    s.settle_queries();
    let pdf = save(&mut s, dir.path(), "tampered");
    let zip = entries(&sources(&pdf))
        .into_iter()
        .map(|(name, data)| {
            if !name.starts_with("data/") {
                return (name, data);
            }
            let mut entry: CachedResult = serde_json::from_slice(&data).unwrap_or_else(|e| panic!("{e}"));
            entry.columns[1].ty = ColumnType::Text;
            (name, serde_json::to_vec(&entry).unwrap_or_else(|e| panic!("{e}")))
        })
        .collect();
    rewrite(&pdf, zip);

    let (mut back, _data) = reopened(&pdf);
    assert_that!(pending(&back, "q")).is_true();
    assert_that!(back.query_jobs()).has_length(1);
}

#[test]
fn save_writes_only_current_results() {
    let (mut s, dir) = with_query(TOP);
    s.settle_queries();
    let key = hex::encode(ready(&s, "q").0.0);
    assert_that!(cached(&save(&mut s, dir.path(), "ready"))).is_equal_to(vec![format!("data/{key}.json")]);

    apply(&mut s, vec![set_b(1, 5.0)]);
    assert_that!(cached(&save(&mut s, dir.path(), "pending"))).is_empty();

    s.settle_queries();
    let key = hex::encode(ready(&s, "q").0.0);
    assert_that!(cached(&save(&mut s, dir.path(), "again"))).is_equal_to(vec![format!("data/{key}.json")]);
}
