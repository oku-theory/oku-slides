//! t-1690: the merge with main first opened a format-8 (v12) deck as format 9. Its replica read
//! dropped each chart's inline `dataset`, and the first edit deleted it from the doc: every chart
//! and table lost its data. A v12 deck is now refused on open, never opened stripped.

use std::io::Write as _;

use loro::{Container, LoroDoc, LoroMap, LoroMovableList, ValueOrContainer};
use oku_slides_core::{
    DeckReplica, Op, ReplicaErrorKind, SlotRef, SourceBundle, SourceLimits, SourcesErrorKind, Target,
};
use serde_json::json;
use speculoos::prelude::*;

use crate::common;

/// A deck with chart `g`, rewritten as main's v12 writer stored it: format 8, the rows inline.
fn v12_snapshot() -> Vec<u8> {
    let mut deck = common::data_deck();
    let element = serde_json::from_value(json!({ "id": "g", "kind": "base/chart", "params": {
        "option": { "data": "d", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } }))
    .unwrap_or_else(|e| panic!("{e}"));
    let slot = SlotRef { owner: Target { slide: "s".into(), element: None }, param: "blocks".into() };
    Op::AddElement { slot, index: None, defaults: false, element }
        .apply(&mut deck, &common::registry())
        .unwrap_or_else(|e| panic!("{e}"));
    let replica = DeckReplica::new(&deck, common::shared()).unwrap_or_else(|e| panic!("{e:?}"));
    let doc = LoroDoc::new();
    doc.import(&replica.snapshot().unwrap_or_else(|e| panic!("{e:?}"))).unwrap_or_else(|e| panic!("{e}"));
    let option = chart_option(&doc);
    option.delete("data").unwrap_or_else(|e| panic!("{e}"));
    let dataset = option.insert_container("dataset", LoroMap::new()).unwrap_or_else(|e| panic!("{e}"));
    let columns = dataset.insert_container("columns", LoroMovableList::new()).unwrap_or_else(|e| panic!("{e}"));
    for name in ["Month", "Sales"] {
        columns.push(name).unwrap_or_else(|e| panic!("{e}"));
    }
    let rows = dataset.insert_container("rows", LoroMovableList::new()).unwrap_or_else(|e| panic!("{e}"));
    let row = rows.push_container(LoroMap::new()).unwrap_or_else(|e| panic!("{e}"));
    row.insert("id", "r1").unwrap_or_else(|e| panic!("{e}"));
    doc.get_map("meta").insert("format", 8).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    doc.export(loro::ExportMode::Snapshot).unwrap_or_else(|e| panic!("{e}"))
}

/// The `option` map of chart `g` in the deck tree.
fn chart_option(doc: &LoroDoc) -> LoroMap {
    let Some(ValueOrContainer::Container(Container::Tree(tree))) = doc.get_map("deck").get("slides") else {
        panic!("no deck tree")
    };
    let map = |v: Option<ValueOrContainer>| match v {
        Some(ValueOrContainer::Container(Container::Map(m))) => m,
        other => panic!("not a map: {other:?}"),
    };
    let node = tree
        .nodes()
        .into_iter()
        .find(|&n| {
            let meta = tree.get_meta(n).unwrap_or_else(|e| panic!("{e}"));
            matches!(meta.get("id"), Some(ValueOrContainer::Value(v)) if v.as_string().is_some_and(|s| &**s == "g"))
        })
        .unwrap_or_else(|| panic!("no chart g"));
    map(map(tree.get_meta(node).unwrap_or_else(|e| panic!("{e}")).get("params")).get("option"))
}

#[test]
fn v12_replica_with_inline_chart_data_is_refused_on_open() {
    let err = DeckReplica::from_snapshot(&v12_snapshot(), 1 << 24, common::shared()).map(|_| ()).map_err(|e| e.kind());
    assert_that!(err).is_equal_to(Err(ReplicaErrorKind::UnsupportedVersion));
}

/// A sources zip holding `snapshot` under manifest `version`.
fn sources_zip(version: u32, snapshot: Vec<u8>) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
        let options = zip::write::SimpleFileOptions::default();
        let manifest = format!(r#"{{"format":"oku-slides-sources","version":{version}}}"#);
        for (name, data) in [("manifest.json", manifest.into_bytes()), ("deck.loro", snapshot)] {
            zip.start_file(name, options).unwrap_or_else(|e| panic!("{e}"));
            zip.write_all(&data).unwrap_or_else(|e| panic!("{e}"));
        }
        zip.finish().unwrap_or_else(|e| panic!("{e}"));
    }
    bytes
}

/// A v12 file (sources 14) is refused by its manifest; a format-8 replica under the current
/// manifest is refused by the replica check.
#[rstest::rstest]
#[case::v12_file(14, SourcesErrorKind::NotSources)]
#[case::relabelled(16, SourcesErrorKind::Replica(ReplicaErrorKind::UnsupportedVersion))]
fn v12_sources_zip_with_inline_chart_data_is_refused_on_open(#[case] version: u32, #[case] kind: SourcesErrorKind) {
    let limits = SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 26, compact_after_secs: 0 };
    let zip = sources_zip(version, v12_snapshot());
    let err = SourceBundle::from_zip(&zip, limits, common::shared()).map(|_| ()).map_err(|e| e.kind());
    assert_that!(err).is_equal_to(Err(kind));
}
