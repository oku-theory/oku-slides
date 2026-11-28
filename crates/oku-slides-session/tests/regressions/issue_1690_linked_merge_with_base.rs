//! t-1690: merging a file into a deck with a linked deck-pack tab copied both decks through a
//! sources zip without the base store, so a file whose data had a base failed to merge. The
//! trial copy now goes through the store.

use std::sync::mpsc::channel;
use std::time::Duration;

use oku_slides_core::{Namespace, SourceBundle, SourceLimits};
use oku_slides_data::{BaseRef, DataId};
use oku_slides_platform::bases::BaseStore;
use oku_slides_platform::storage::Storage;
use oku_slides_session::StoreFiles;
use serde_json::json;
use speculoos::prelude::*;

use crate::common::{Data, data_dir, registry};
use crate::workspace_support::{import_hifi, on, op, open_pack, workspace};

const LIMITS: SourceLimits = SourceLimits { replica_bytes: 1 << 26, total_bytes: 1 << 28, compact_after_secs: 0 };

/// `pdf` with its sources' data `d9` folded onto a new base, written to `to`.
fn with_base(data: &Data, pdf: &std::path::Path, to: &std::path::Path) {
    let bytes = std::fs::read(pdf).unwrap_or_else(|e| panic!("{e}"));
    let zip = oku_slides_pdf::extract_sources(&bytes)
        .unwrap_or_else(|e| panic!("{e}"))
        .unwrap_or_else(|| panic!("no sources"))
        .zip;
    let store = BaseStore::new(Storage::local().unwrap_or_else(|e| panic!("{e}")), data.as_ref(), 1 << 30)
        .unwrap_or_else(|e| panic!("{e}"));
    let files = StoreFiles::new(&store);
    let (mut theirs, _) = SourceBundle::from_zip_bases(&zip, LIMITS, registry(), Some(&files))
        .unwrap_or_else(|e| panic!("their copy: {e}"));
    let src = data.as_ref().join("base-source");
    std::fs::write(&src, b"their base").unwrap_or_else(|e| panic!("{e}"));
    let id = store.adopt(&src, 1 << 20).unwrap_or_else(|e| panic!("adopt: {e}"));
    let d9 = DataId::try_from("d9".to_owned()).unwrap_or_else(|e| panic!("{e}"));
    let from = theirs.deck().data[&d9].table().unwrap_or_else(|| panic!("a manual data")).clone();
    let rows = u32::try_from(from.rows.len()).unwrap_or_else(|e| panic!("{e}"));
    let base = BaseRef { id, bytes: 10, rows, columns: Vec::new() };
    let done = theirs.compact_data(&d9, &from, "e2".to_owned(), base).unwrap_or_else(|e| panic!("compact: {e}"));
    assert_that!(done).is_true();
    let zip = theirs.to_zip_bases(true, Some(&files)).unwrap_or_else(|e| panic!("their zip: {e}"));
    let written = oku_slides_pdf::embed_sources(&bytes, &zip).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(to, written).unwrap_or_else(|e| panic!("{e}"));
}

fn scenario(data: &Data) {
    let ws = workspace(data);
    let deck = import_hifi(&ws, data);
    let pack = on(&ws, &deck, |s| s.deck().layouts.get(&Namespace::deck_layouts()).map(|p| p.id))
        .unwrap_or_else(|| panic!("the import's deck pack"));
    open_pack(&ws, &deck, pack);
    let table = json!({ "columns": [{ "id": "c1", "name": "Mois", "type": "text" }],
        "rows": [{ "id": "r1", "cells": ["Jan"] }] });
    let add = op(json!({ "op": "add_data", "id": "d9", "name": "ventes", "table": table }));
    on(&ws, &deck, |s| s.apply("data", vec![add], None)).unwrap_or_else(|e| panic!("apply: {}", e.one_line()));
    let ours = data.as_ref().join("ours.oku-slides.pdf");
    on(&ws, &deck, |s| s.save(Some(&ours))).unwrap_or_else(|e| panic!("save: {}", e.one_line()));
    let theirs = data.as_ref().join("theirs.oku-slides.pdf");
    with_base(data, &ours, &theirs);

    let merged = on(&ws, &deck, |s| s.merge_file(&theirs));

    assert_that!(merged.map_err(|e| e.one_line())).is_ok();
}

#[test]
fn issue_1690_linked_merge_with_base() {
    let (done, finished) = channel();
    std::thread::spawn(move || {
        scenario(&data_dir());
        done.send(()).unwrap_or_else(|e| panic!("{e}"));
    });
    finished.recv_timeout(Duration::from_secs(120)).unwrap_or_else(|e| panic!("timed out: {e}"));
}
