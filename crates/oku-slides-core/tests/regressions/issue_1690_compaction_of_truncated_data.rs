//! t-1690: compacting a truncated data returned `true` and wrote nothing, as the replica never
//! writes a truncated data back. The host then believed the base committed while the over-bound
//! overlay stayed, and the data stayed truncated on every read.

use std::sync::Arc;

use oku_slides_core::{Deck, DeckReplica};
use oku_slides_data::limits::MAX_OVERLAY;
use oku_slides_data::{BaseId, BaseRef, CellKey, Column, ColumnType, Data, DataCell, DataId, Rows, Source, Table};
use speculoos::prelude::*;

use crate::common;

/// Data `z` with one overlay entry past `MAX_OVERLAY`.
fn over_cap() -> Deck {
    let columns: Vec<Column> =
        (0..4).map(|j| Column { id: format!("c{j}"), name: format!("v{j}"), ty: ColumnType::Number }).collect();
    let rows = MAX_OVERLAY / 4 + 1;
    let cells = (0..=MAX_OVERLAY)
        .map(|n| (CellKey { row: format!("r{}", n / 4), column: format!("c{}", n % 4) }, DataCell::Number(1.0)))
        .collect();
    let table = Table {
        generation: "g".to_owned(),
        epoch: String::new(),
        columns,
        rows: Rows::repaired((0..rows).map(|r| format!("r{r}"))),
        base: None,
        cells,
    };
    let data = Data { name: "z".to_owned(), label: None, inline: false, source: Source::Manual(table) };
    let mut deck = Deck::default();
    deck.data.insert(z(), Arc::new(data));
    deck
}

fn z() -> DataId {
    DataId::try_from("z".to_owned()).unwrap_or_else(|e| panic!("id: {e:?}"))
}

#[test]
fn compaction_of_truncated_data_writes_the_new_layer() {
    let registry = common::shared();
    let mut r = DeckReplica::new(&over_cap(), registry.clone()).unwrap_or_else(|e| panic!("new: {e:?}"));
    let (read, _) = r.deck();
    assert_that!(read.truncated.contains(&z())).is_true();
    let base = BaseRef {
        id: BaseId::try_from(format!("sha256:{}", "a".repeat(64))).unwrap_or_else(|e| panic!("base: {e:?}")),
        bytes: 10,
        rows: 3,
        columns: Vec::new(),
    };
    let from = read.data[&z()].table().unwrap_or_else(|| panic!("a manual data")).clone();
    assert_that!(r.compact_data(&z(), &from, "e2".to_owned(), base.clone()).unwrap_or(false)).is_true();
    let snapshot = r.snapshot().unwrap_or_else(|e| panic!("snapshot: {e:?}"));
    let (back, _) = DeckReplica::from_snapshot(&snapshot, 1 << 28, registry).unwrap_or_else(|e| panic!("{e:?}"));
    let (deck, _) = back.deck();
    assert_that!(deck.truncated.contains(&z())).is_false();
    let table = deck.data[&z()].table().unwrap_or_else(|| panic!("a manual data"));
    assert_that!(table.base.as_ref()).is_equal_to(Some(&base));
    assert_that!(table.overlay_len()).is_equal_to(0);
}
