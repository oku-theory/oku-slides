use std::io::Read;
use std::path::{Path, PathBuf};

use oku_slides_core::SourceBundle;
use oku_slides_data::{BaseId, BaseRef, DataId};
use oku_slides_i18n::Localizer;
use oku_slides_platform::bases::BaseStore;
use serde_json::json;
use speculoos::prelude::*;

use crate::resume::{DECK_DIR, DocSnapshot, ReadBudget, Resume};
use crate::test_support::{fresh_dir, session_on, session_with, settings_on};
use crate::{DocDefaults, Localize, Session};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");

fn did() -> DataId {
    data_id("d9")
}

fn data_id(id: &str) -> DataId {
    DataId::try_from(id.to_owned()).unwrap_or_else(|e| panic!("{e}"))
}

/// The roadmap deck with the data `d9`, shown by no view.
fn opened(data: &Path) -> Session {
    let mut s = session_on(data);
    s.open(Path::new(FIXTURE)).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    let table = json!({
        "generation": "g1",
        "columns": [{ "id": "c1", "name": "Mois" }, { "id": "c2", "name": "ventes" }],
        "rows": [{ "id": "r1", "cells": ["Jan", 10] }, { "id": "r2", "cells": ["Fév", 12] }]
    });
    let add = json!({ "op": "add_data", "id": "d9", "name": "ventes", "table": table });
    let op = serde_json::from_value(add).unwrap_or_else(|e| panic!("{e}"));
    s.apply("données", vec![op], None).unwrap_or_else(|e| panic!("apply: {}", e.one_line()));
    s
}

/// `bytes` put in `store`, unleased (as a base a closed tab left).
pub(crate) fn put_base(store: &BaseStore, bytes: &[u8]) -> BaseRef {
    let dir = fresh_dir("oku-slides-base-src-");
    let src = dir.path().join("b.parquet");
    std::fs::write(&src, bytes).unwrap_or_else(|e| panic!("{e}"));
    let id = store.adopt(&src, 1 << 24).unwrap_or_else(|e| panic!("adopt: {e}"));
    store.release([&id]).unwrap_or_else(|e| panic!("release: {e}"));
    BaseRef { id, bytes: bytes.len() as u64, rows: 2, columns: Vec::new() }
}

/// Compacts `d9` of `s` onto the base `bytes`, as a compaction would; its id.
fn compact_onto(s: &mut Session, bytes: &[u8], epoch: &str) -> BaseId {
    compact_data_onto(s, "d9", bytes, epoch)
}

/// Compacts the data `data` of `s` onto the base `bytes`, as a compaction would; its id.
pub(crate) fn compact_data_onto(s: &mut Session, data: &str, bytes: &[u8], epoch: &str) -> BaseId {
    let base = put_base(s.bases.store(), bytes);
    let id = base.id.clone();
    let data = data_id(data);
    let from = s.deck().data[&data].table.clone();
    let done = s.doc.sources.compact_data(&data, &from, epoch.to_owned(), base);
    assert_that!(done.unwrap_or_else(|e| panic!("compact: {e}"))).is_true();
    s.sync_bases();
    id
}

fn save(s: &mut Session, dir: &Path) -> PathBuf {
    let path = dir.join("bases.oku-slides.pdf");
    s.save(Some(&path)).unwrap_or_else(|e| panic!("save: {}", e.one_line()));
    path
}

/// The names of the `bases/` entries of the sources zip in `pdf`.
fn zip_bases(pdf: &Path) -> Vec<String> {
    let bytes = std::fs::read(pdf).unwrap_or_else(|e| panic!("{e}"));
    let found = oku_slides_pdf::extract_sources(&bytes)
        .unwrap_or_else(|e| panic!("{e}"))
        .unwrap_or_else(|| panic!("no sources"));
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(found.zip)).unwrap_or_else(|e| panic!("{e}"));
    let mut names = Vec::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).unwrap_or_else(|e| panic!("{e}"));
        let name = entry.name().unwrap_or_else(|e| panic!("{e}")).into_owned();
        if name.starts_with("bases/") {
            names.push(name);
            entry.read_to_end(&mut Vec::new()).unwrap_or_else(|e| panic!("{e}"));
        }
    }
    names
}

fn entry(id: &BaseId) -> String {
    format!("bases/{}.parquet", id.hex())
}

#[test]
fn save_and_reopen_deck_with_base() {
    let (one, two) = (fresh_dir("oku-slides-bases-a-"), fresh_dir("oku-slides-bases-b-"));
    let mut s = opened(one.path());
    let id = compact_onto(&mut s, b"base des ventes", "e1");
    let path = save(&mut s, one.path());
    let mut t = session_on(two.path());
    t.open(&path).unwrap_or_else(|e| panic!("reopen: {}", e.one_line()));
    assert_that!(t.doc.sources.bases.contains(&id)).is_true();
    assert_that!(t.bases.store().contains(&id).unwrap_or_else(|e| panic!("{e}"))).is_true();
    let base = t.deck().data[&did()].table.base.as_ref().map(|b| b.id.clone());
    assert_that!(base).is_equal_to(Some(id));
}

#[test]
fn opened_base_with_wrong_hash_is_refused_fr() {
    let (one, two) = (fresh_dir("oku-slides-bases-a-"), fresh_dir("oku-slides-bases-b-"));
    let mut s = opened(one.path());
    let id = compact_onto(&mut s, b"base des ventes", "e1");
    // The store's file no longer hashes to its name: the saved entry is forged.
    std::fs::write(s.bases.store().path(&id), b"autre contenu").unwrap_or_else(|e| panic!("{e}"));
    let path = save(&mut s, one.path());
    let mut t = session_on(two.path());
    let e = t.open(&path).err().unwrap_or_else(|| panic!("a forged base opens"));
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).contains(fr.tr("sources-base-refused").as_str());
    assert_that!(t.bases.store().contains(&id).unwrap_or_else(|e| panic!("{e}"))).is_false();
}

#[test]
fn save_writes_only_current_bases() {
    let dir = fresh_dir("oku-slides-bases-");
    let mut s = opened(dir.path());
    let old = compact_onto(&mut s, b"base d'hier", "e1");
    let new = compact_onto(&mut s, b"base du jour", "e2");
    assert_that!(s.doc.sources.bases.contains(&old)).is_true();
    let path = save(&mut s, dir.path());
    assert_that!(zip_bases(&path)).is_equal_to(vec![entry(&new)]);
}

#[test]
fn base_missing_at_resume_reports_base_missing() {
    let dir = fresh_dir("oku-slides-bases-");
    let mut s = opened(dir.path());
    let id = compact_onto(&mut s, b"base des ventes", "e1");
    let resume = Resume::new(
        oku_slides_platform::storage::Storage::local().unwrap_or_else(|e| panic!("{e}")),
        dir.path().join("tab"),
    );
    let snapshot = DocSnapshot::of(&s.doc, &mut Vec::new()).unwrap_or_else(|e| panic!("{e}"));
    resume.write_snapshot(DECK_DIR, &snapshot).unwrap_or_else(|e| panic!("{e}"));
    std::fs::remove_file(s.bases.store().path(&id)).unwrap_or_else(|e| panic!("{e}"));
    let defaults = DocDefaults { undo_limit: 10, author: None, store_usage: true };
    let mut budget = ReadBudget::new(s.source_limits);
    let (doc, issues) = resume
        .read_doc_issues(DECK_DIR, &defaults, &mut budget, &s.registry, s.bases.store())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_that!(doc.sources.bases.contains(&id)).is_false();
    let keys: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(keys.contains(&"issue-data-base-missing")).is_true();
}

#[test]
fn sources_over_1024_mib_refused_at_open() {
    let defaults = crate::Settings::defaults().unwrap_or_else(|e| panic!("{e}"));
    assert_that!(defaults.sources.max_sources_mib).is_equal_to(1024);
    let (one, two) = (fresh_dir("oku-slides-bases-a-"), fresh_dir("oku-slides-bases-b-"));
    let mut s = opened(one.path());
    // A base past the cap: 1 MiB stands for 1024 here, the rule is the same.
    let big = vec![7_u8; (1 << 20) + 1];
    let id = compact_onto(&mut s, &big, "e1");
    let path = save(&mut s, one.path());
    let mut settings = settings_on(two.path());
    settings.sources.max_sources_mib = 1;
    let mut t = session_with(settings);
    let e = t.open(&path).err().unwrap_or_else(|| panic!("sources over the cap open"));
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).contains(fr.tr("sources-too-large").as_str());
    assert_that!(t.bases.store().contains(&id).unwrap_or_else(|e| panic!("{e}"))).is_false();
    let _: &SourceBundle = &t.doc.sources;
}

/// → a deck whose base nears the 512 MiB deck bound saves and reopens with it (slow group).
#[test]
fn save_and_reopen_near_512_mib_of_bases() {
    use oku_slides_data::limits::MAX_DECK_BASE_BYTES;
    let (one, two) = (fresh_dir("oku-slides-bases-a-"), fresh_dir("oku-slides-bases-b-"));
    let mut s = opened(one.path());
    // Incompressible bytes, as Parquet is: a zip of them is as large as the base.
    let len = usize::try_from(MAX_DECK_BASE_BYTES).unwrap_or_else(|e| panic!("{e}")) - (1 << 20);
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let bytes: Vec<u8> = (0..len / 8)
        .flat_map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state.to_le_bytes()
        })
        .collect();
    let src = fresh_dir("oku-slides-base-src-");
    std::fs::write(src.path().join("b.parquet"), &bytes).unwrap_or_else(|e| panic!("{e}"));
    let store = s.bases.store().clone();
    let id = store.adopt(&src.path().join("b.parquet"), MAX_DECK_BASE_BYTES).unwrap_or_else(|e| panic!("adopt: {e}"));
    store.release([&id]).unwrap_or_else(|e| panic!("release: {e}"));
    drop(bytes);
    let base = BaseRef { id: id.clone(), bytes: len as u64, rows: 2, columns: Vec::new() };
    let from = s.deck().data[&did()].table.clone();
    let done = s.doc.sources.compact_data(&did(), &from, "e1".to_owned(), base);
    assert_that!(done.unwrap_or_else(|e| panic!("compact: {e}"))).is_true();
    s.sync_bases();

    let path = save(&mut s, one.path());
    let mut t = session_on(two.path());
    t.open(&path).unwrap_or_else(|e| panic!("reopen: {}", e.one_line()));

    assert_that!(t.doc.sources.bases.contains(&id)).is_true();
    let stored = std::fs::metadata(t.bases.store().path(&id)).unwrap_or_else(|e| panic!("{e}")).len();
    assert_that!(stored).is_equal_to(len as u64);
}
