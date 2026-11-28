use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Read;
use std::sync::Arc;

use oku_slides_data::{BaseId, BaseRef, CellKey, Column, ColumnType, Data, DataCell, DataId, Rows, Source, Table};
use sha2::{Digest, Sha256};
use speculoos::prelude::*;

use super::BaseFiles;
use crate::error::SourcesErrorKind;
use crate::{Deck, SourceBundle, SourceLimits};

const MAX: SourceLimits = SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 26, compact_after_secs: 0 };

/// An in-memory store that checks hashes and bounds as the platform's does.
#[derive(Default)]
pub(crate) struct Store(pub(crate) RefCell<BTreeMap<BaseId, Vec<u8>>>);

impl BaseFiles for Store {
    fn open(&self, id: &BaseId) -> Result<Box<dyn Read + '_>, super::BaseFilesError> {
        let bytes = self.0.borrow().get(id).cloned().ok_or("missing")?;
        Ok(Box::new(std::io::Cursor::new(bytes)))
    }

    fn put(&self, r: &mut dyn Read, id: &BaseId, max: u64) -> Result<(), super::BaseFilesError> {
        let mut bytes = Vec::new();
        r.take(max + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > max {
            return Err("too large".into());
        }
        if id_of(&bytes) != *id {
            return Err("hash mismatch".into());
        }
        self.0.borrow_mut().insert(id.clone(), bytes);
        Ok(())
    }
}

pub(crate) fn id_of(bytes: &[u8]) -> BaseId {
    BaseId::from_sha256(&Sha256::digest(bytes).into())
}

fn did(id: &str) -> DataId {
    DataId::try_from(id.to_owned()).unwrap()
}

fn base(bytes: &[u8]) -> BaseRef {
    BaseRef { id: id_of(bytes), bytes: bytes.len() as u64, rows: 1, columns: Vec::new() }
}

fn data() -> Arc<Data> {
    let table = Table {
        generation: "g".to_owned(),
        epoch: String::new(),
        columns: vec![Column { id: "c1".to_owned(), name: "n".to_owned(), ty: ColumnType::Number }],
        rows: Rows::repaired(vec!["r1".to_owned()]),
        base: None,
        cells: [(CellKey { row: "r1".to_owned(), column: "c1".to_owned() }, DataCell::Number(1.0))]
            .into_iter()
            .collect(),
    };
    Arc::new(Data { name: "d".to_owned(), label: None, inline: false, source: Source::Manual(table) })
}

/// A bundle whose data `d1` was compacted over the base `bytes`.
fn based(bytes: &[u8]) -> SourceBundle {
    let deck = Deck { data: [(did("d1"), data())].into_iter().collect(), ..Deck::default() };
    let mut b = SourceBundle::new(deck, crate::test_registry::shared()).unwrap();
    compact(&mut b, bytes, "e1");
    b
}

fn compact(b: &mut SourceBundle, bytes: &[u8], epoch: &str) {
    let from = b.deck().data[&did("d1")].table().unwrap().clone();
    assert_that!(b.compact_data(&did("d1"), &from, epoch.to_owned(), base(bytes)).unwrap()).is_true();
}

fn keys(issues: &[crate::Issue]) -> Vec<&'static str> {
    issues.iter().map(|i| i.message.id()).collect()
}

#[test]
fn compacted_base_is_known() {
    let b = based(b"one");
    assert_that!(b.bases.contains(&id_of(b"one"))).is_true();
}

#[test]
fn based_bundle_round_trips_its_base_through_the_store() {
    let from = Store::default();
    from.0.borrow_mut().insert(id_of(b"one"), b"one".to_vec());
    let zip = based(b"one").to_zip_bases(true, Some(&from)).unwrap();
    let to = Store::default();
    let (back, issues) = SourceBundle::from_zip_bases(&zip, MAX, crate::test_registry::shared(), Some(&to)).unwrap();
    assert_that!(keys(&issues)).is_empty();
    assert_that!(back.bases.contains(&id_of(b"one"))).is_true();
    assert_that!(to.0.borrow().get(&id_of(b"one")).cloned()).is_equal_to(Some(b"one".to_vec()));
}

#[test]
fn save_writes_only_current_bases() {
    let store = Store::default();
    for bytes in [&b"old"[..], b"new"] {
        store.0.borrow_mut().insert(id_of(bytes), bytes.to_vec());
    }
    let mut b = based(b"old");
    compact(&mut b, b"new", "e2");
    let zip = b.to_zip_bases(true, Some(&store)).unwrap();
    let to = Store::default();
    SourceBundle::from_zip_bases(&zip, MAX, crate::test_registry::shared(), Some(&to)).unwrap();
    let stored: Vec<BaseId> = to.0.borrow().keys().cloned().collect();
    assert_that!(stored).is_equal_to(vec![id_of(b"new")]);
}

#[test]
fn based_bundle_without_store_is_not_saved() {
    let err = based(b"one").to_zip_bases(true, None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::Base);
}

#[test]
fn opened_base_with_wrong_hash_is_refused() {
    let lying = Store::default();
    lying.0.borrow_mut().insert(id_of(b"one"), b"two".to_vec());
    let zip = based(b"one").to_zip_bases(true, Some(&lying)).unwrap();
    let err =
        SourceBundle::from_zip_bases(&zip, MAX, crate::test_registry::shared(), Some(&Store::default())).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::Base);
}

#[test]
fn base_entry_over_the_sources_budget_is_too_large() {
    let big = vec![7u8; 1 << 16];
    let store = Store::default();
    store.0.borrow_mut().insert(id_of(&big), big.clone());
    let zip = based(&big).to_zip_bases(true, Some(&store)).unwrap();
    // Room for everything but the base.
    let limits = SourceLimits { total_bytes: 1 << 15, ..MAX };
    let err = SourceBundle::from_zip_bases(&zip, limits, crate::test_registry::shared(), Some(&Store::default()))
        .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::TooLarge);
}

#[test]
fn base_not_received_reads_as_base_missing() {
    let store = Store::default();
    store.0.borrow_mut().insert(id_of(b"one"), b"one".to_vec());
    let zip = based(b"one").to_zip_bases(true, Some(&store)).unwrap();
    let (back, issues) = SourceBundle::from_zip(&zip, MAX, crate::test_registry::shared()).unwrap();
    assert_that!(keys(&issues)).is_equal_to(vec!["issue-data-base-missing"]);
    assert_that!(back.bases.is_empty()).is_true();
}

#[test]
fn merge_learns_the_other_copy_bases() {
    let store = Store::default();
    store.0.borrow_mut().insert(id_of(b"one"), b"one".to_vec());
    let mut a = based(b"one");
    let zip = a.to_zip_bases(true, Some(&store)).unwrap();
    let (mut b, _) = SourceBundle::from_zip_bases(&zip, MAX, crate::test_registry::shared(), Some(&store)).unwrap();
    compact(&mut b, b"two", "e2");
    let (_, issues) = a.merge(b).unwrap();
    assert_that!(keys(&issues)).is_empty();
    assert_that!(a.bases.contains(&id_of(b"two"))).is_true();
}
