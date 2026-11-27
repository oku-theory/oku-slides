use std::sync::Arc;

use oku_slides_data::{BaseId, BaseRef, CellKey, Column, ColumnType, Data, DataCell, DataId, DataKind, Rows, Table};
use speculoos::prelude::*;

use crate::{Deck, DeckReplica};

fn did() -> DataId {
    DataId::try_from("d1".to_owned()).unwrap()
}

fn base() -> BaseRef {
    BaseRef {
        id: BaseId::try_from(format!("sha256:{}", "b".repeat(64))).unwrap(),
        bytes: 1,
        rows: 1,
        columns: Vec::new(),
    }
}

fn deck() -> Deck {
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
    let data = Data { name: "d".to_owned(), label: None, inline: false, kind: DataKind::Manual, table };
    Deck { data: [(did(), Arc::new(data))].into_iter().collect(), ..Deck::default() }
}

fn replica() -> DeckReplica {
    DeckReplica::new(&deck(), crate::test_registry::shared()).unwrap()
}

#[test]
fn compaction_writes_new_epoch_over_base_with_empty_overlay() {
    let mut r = replica();
    let from = r.deck().0.data[&did()].table.clone();
    assert_that!(r.compact_data(&did(), &from, "e2".to_owned(), base()).unwrap()).is_true();
    let table = r.deck().0.data[&did()].table.clone();
    assert_that!(table.epoch.as_str()).is_equal_to("e2");
    assert_that!(table.base).is_equal_to(Some(base()));
    assert_that!(table.cells.len()).is_equal_to(0);
}

#[test]
fn compaction_is_not_an_undo_step() {
    let mut r = replica();
    let from = r.deck().0.data[&did()].table.clone();
    r.compact_data(&did(), &from, "e2".to_owned(), base()).unwrap();
    assert_that!(r.can_undo()).is_false();
}

#[test]
fn compaction_of_a_changed_table_writes_nothing() {
    let mut r = replica();
    let mut from = r.deck().0.data[&did()].table.clone();
    assert_that!(from.cells.len()).is_equal_to(1);
    from.cells.clear();
    assert_that!(r.compact_data(&did(), &from, "e2".to_owned(), base()).unwrap()).is_false();
    assert_that!(r.deck().0.data[&did()].table.epoch.as_str()).is_equal_to("");
}

#[test]
fn compaction_of_absent_data_writes_nothing() {
    let mut r = DeckReplica::new(&Deck::default(), crate::test_registry::shared()).unwrap();
    let from = deck().data[&did()].table.clone();
    assert_that!(r.compact_data(&did(), &from, "e2".to_owned(), base()).unwrap()).is_false();
}

#[test]
fn compacted_base_reads_back_editable_without_issue() {
    let mut r = replica();
    let from = r.deck().0.data[&did()].table.clone();
    r.compact_data(&did(), &from, "e2".to_owned(), base()).unwrap();
    let back =
        DeckReplica::from_snapshot(&r.snapshot().unwrap(), 1usize << 24, crate::test_registry::shared()).unwrap().1;
    assert_that!(back.1.is_empty()).is_true();
    assert_that!(back.0.truncated.contains(&did())).is_false();
    assert_that!(back.0.data[&did()].table.base.clone()).is_equal_to(Some(base()));
}

/// Compaction applies an emptied bundle with `KnowledgeSync::Unchanged`: the stored knowledge
/// must survive, which a switch to `Changed` would break.
#[test]
fn compaction_keeps_knowledge_bundle() {
    let mut start = deck();
    let file = crate::KnowledgeFile { path: "/a.md".to_owned(), text: "body".to_owned() };
    start.knowledge.insert(format!("{:032x}", 1), file);
    let mut r = DeckReplica::new(&start, crate::test_registry::shared()).unwrap();
    let from = r.deck().0.data[&did()].table.clone();
    r.compact_data(&did(), &from, "e2".to_owned(), base()).unwrap();
    assert_that!(r.deck().0.knowledge).is_equal_to(start.knowledge);
}
