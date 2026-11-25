use std::collections::BTreeSet;
use std::sync::Arc;

use loro::{Container, ContainerTrait, LoroMap, LoroMovableList, LoroValue, ValueOrContainer};
use oku_slides_data::limits::{
    MAX_COLUMNS, MAX_DATA, MAX_DECK_OVERLAY, MAX_DECK_ROW_IDS, MAX_DECK_TEXT_BYTES, MAX_OVERLAY, MAX_ROWS,
    MAX_TEXT_BYTES,
};
use oku_slides_data::{
    BaseId, BaseRef, CellKey, CellRead, Column, ColumnType, Data, DataCell, DataId, DataKind, Rows, Table,
};
use rstest::rstest;
use speculoos::prelude::*;

use super::super::{DeckReplica, ROOT};
use super::*;
use crate::validate::Issue;
use crate::widget::WidgetRegistry;
use crate::{Deck, validate};

fn registry() -> Arc<WidgetRegistry> {
    crate::test_registry::shared()
}

fn did(id: &str) -> DataId {
    DataId::try_from(id.to_owned()).unwrap()
}

fn key(row: &str, column: &str) -> CellKey {
    CellKey { row: row.to_owned(), column: column.to_owned() }
}

fn column(id: &str, name: &str, ty: ColumnType) -> Column {
    Column { id: id.to_owned(), name: name.to_owned(), ty }
}

/// Columns `c1` (Country, text) and `c2` (Sales, number), rows `r1`-`r3`, a few cells.
fn table() -> Table {
    let cells = [
        (key("r1", "c1"), DataCell::Text("FR".to_owned())),
        (key("r1", "c2"), DataCell::Number(12.5)),
        (key("r2", "c1"), DataCell::Text("DE".to_owned())),
        (key("r3", "c2"), DataCell::Number(-3.0)),
    ];
    Table {
        generation: "g1".to_owned(),
        epoch: String::new(),
        columns: vec![column("c1", "Country", ColumnType::Text), column("c2", "Sales", ColumnType::Number)],
        rows: Rows::repaired(["r1", "r2", "r3"].map(str::to_owned)),
        base: None,
        cells: cells.into_iter().collect(),
    }
}

fn data(name: &str, table: Table) -> Arc<Data> {
    Arc::new(Data {
        name: name.to_owned(),
        label: Some("Sales by country".to_owned()),
        inline: false,
        kind: DataKind::Manual,
        table,
    })
}

/// A deck holding `data`, by id.
fn deck_of(data: Vec<(&str, Arc<Data>)>) -> Deck {
    Deck { data: data.into_iter().map(|(id, d)| (did(id), d)).collect(), ..Deck::default() }
}

/// A deck with one data `d1`, named `sales`.
fn sample() -> Deck {
    deck_of(vec![("d1", data("sales", table()))])
}

/// `deck` with `edit` applied to the data `id`.
fn edited(deck: &Deck, id: &str, edit: impl FnOnce(&mut Data)) -> Deck {
    let mut deck = deck.clone();
    edit(Arc::make_mut(deck.data.get_mut(&did(id)).unwrap()));
    deck
}

fn set_cell(t: &mut Table, row: &str, col: &str, cell: DataCell) {
    t.cells.insert(key(row, col), cell);
}

fn base() -> BaseRef {
    BaseRef {
        id: BaseId::try_from(format!("sha256:{}", "a".repeat(64))).unwrap(),
        bytes: 10,
        rows: 3,
        columns: Vec::new(),
    }
}

/// Two peers opened from one snapshot of `deck`.
fn peers(deck: &Deck) -> (DeckReplica, DeckReplica) {
    let snap = DeckReplica::new(deck, registry()).unwrap().snapshot().unwrap();
    let open = || DeckReplica::from_snapshot(&snap, 1 << 26, registry()).unwrap().0;
    (open(), open())
}

/// `a` and `b` after each applied its deck and both merged the other.
fn merged(deck: &Deck, da: &Deck, db: &Deck) -> (DeckReplica, DeckReplica) {
    let (mut a, mut b) = peers(deck);
    a.apply(da, "A", None).unwrap();
    b.apply(db, "B", None).unwrap();
    a.merge(&b).unwrap();
    b.merge(&a).unwrap();
    (a, b)
}

fn keys(issues: &[Issue]) -> Vec<&'static str> {
    issues.iter().map(|i| i.message.id()).collect()
}

fn data_map(r: &DeckReplica) -> LoroMap {
    match r.doc.get_map(ROOT).get(DATA) {
        Some(ValueOrContainer::Container(Container::Map(m))) => m,
        other => panic!("data map expected: {other:?}"),
    }
}

fn child_map(map: &LoroMap, key: &str) -> LoroMap {
    match map.get(key) {
        Some(ValueOrContainer::Container(Container::Map(m))) => m,
        other => panic!("map {key} expected: {other:?}"),
    }
}

fn child_list(map: &LoroMap, key: &str) -> LoroMovableList {
    match map.get(key) {
        Some(ValueOrContainer::Container(Container::MovableList(l))) => l,
        other => panic!("list {key} expected: {other:?}"),
    }
}

/// The current table container of data key `id`.
fn table_map(r: &DeckReplica, id: &str) -> LoroMap {
    child_map(&child_map(&data_map(r), id), TABLE)
}

fn cells_map(r: &DeckReplica, id: &str) -> LoroMap {
    child_map(&child_map(&table_map(r, id), LAYER), CELLS)
}

/// The deck read back from a fresh peer opened on `r`'s snapshot.
fn reopened(r: &DeckReplica) -> (Deck, Vec<Issue>) {
    DeckReplica::from_snapshot(&r.snapshot().unwrap(), 1 << 28, registry()).unwrap().0.deck()
}

fn table_of<'a>(deck: &'a Deck, id: &str) -> &'a Table {
    &deck.data[&did(id)].table
}

// --- Writing and reading -------------------------------------------------------------------

#[test]
fn data_round_trips_through_replica() {
    let deck = sample();
    let r = DeckReplica::new(&deck, registry()).unwrap();
    let (got, issues) = reopened(&r);
    assert_that!(issues).is_empty();
    assert_that!(got.data).is_equal_to(deck.data);
    assert_that!(got.truncated.is_empty()).is_true();
}

#[test]
fn unchanged_apply_writes_nothing() {
    let deck = sample();
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let before = r.doc.oplog_vv();
    r.apply(&deck.clone(), "same", None).unwrap();
    assert_that!(r.doc.oplog_vv()).is_equal_to(before);
}

#[test]
fn set_cell_writes_one_loro_change() {
    let deck = sample();
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let ops = r.len_ops();
    let changed = edited(&deck, "d1", |d| set_cell(&mut d.table, "r2", "c2", DataCell::Number(4.0)));
    r.apply(&changed, "cell", None).unwrap();
    assert_that!(r.len_ops()).is_equal_to(ops + 1);
    assert_that!(r.deck().0.data).is_equal_to(changed.data);
}

#[test]
fn set_cell_after_merge_diffs_against_the_doc() {
    let deck = sample();
    let (mut a, b) = peers(&deck);
    a.merge(&b).unwrap();
    let changed = edited(&deck, "d1", |d| set_cell(&mut d.table, "r2", "c2", DataCell::Number(4.0)));
    a.apply(&changed, "cell", None).unwrap();
    assert_that!(reopened(&a).0.data).is_equal_to(changed.data);
}

#[rstest]
#[case::with_base(true)]
#[case::without_base(false)]
fn null_marker_round_trips_through_replica(#[case] with_base: bool) {
    let deck = edited(&sample(), "d1", |d| {
        d.table.base = with_base.then(base);
        set_cell(&mut d.table, "r2", "c2", DataCell::Null);
    });
    let r = DeckReplica::new(&deck, registry()).unwrap();
    let (got, _) = reopened(&r);
    let cell = table_of(&got, "d1").cells.get(&key("r2", "c2")).cloned();
    assert_that!(cell).is_equal_to(with_base.then_some(DataCell::Null));
    assert_that!(table_of(&got, "d1").base.clone()).is_equal_to(with_base.then(base));
}

#[test]
fn removing_data_marks_its_map_and_hides_it() {
    let deck = sample();
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    r.apply(&Deck::default(), "remove", None).unwrap();
    let marked = child_map(&data_map(&r), "d1");
    assert_that!(super::super::tree::is_marked(&marked)).is_true();
    assert_that!(r.deck().0.data.is_empty()).is_true();
    assert_that!(reopened(&r).0.data.is_empty()).is_true();
}

#[test]
fn adding_a_removed_id_again_starts_a_fresh_map() {
    let deck = sample();
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let old = child_map(&data_map(&r), "d1").id();
    r.apply(&Deck::default(), "remove", None).unwrap();
    r.apply(&deck, "add again", None).unwrap();
    let fresh = child_map(&data_map(&r), "d1");
    assert_that!(fresh.id()).is_not_equal_to(old);
    assert_that!(super::super::tree::is_marked(&fresh)).is_false();
    assert_that!(reopened(&r).0.data).is_equal_to(deck.data);
}

#[test]
fn replace_starts_a_new_table_container() {
    let deck = sample();
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let old = table_map(&r, "d1").id();
    let replaced = edited(&deck, "d1", |d| d.table.generation = "g2".to_owned());
    r.apply(&replaced, "replace", None).unwrap();
    assert_that!(table_map(&r, "d1").id()).is_not_equal_to(old);
    assert_that!(reopened(&r).0.data).is_equal_to(replaced.data);
}

// --- Concurrency ---------------------------------------------------------------------------

#[rstest]
#[case::different_cells(("r1", "c1"), ("r2", "c2"))]
#[case::same_row(("r1", "c1"), ("r1", "c2"))]
#[case::same_column(("r1", "c2"), ("r2", "c2"))]
fn concurrent_cell_edits_on_shared_data_both_kept(#[case] ka: (&str, &str), #[case] kb: (&str, &str)) {
    let deck = sample();
    let da = edited(&deck, "d1", |d| set_cell(&mut d.table, ka.0, ka.1, DataCell::Text("A".to_owned())));
    let db = edited(&deck, "d1", |d| set_cell(&mut d.table, kb.0, kb.1, DataCell::Text("B".to_owned())));
    let (a, b) = merged(&deck, &da, &db);
    let got = a.deck().0;
    let cells = &table_of(&got, "d1").cells;
    assert_that!(cells.get(&key(ka.0, ka.1)).cloned()).is_equal_to(Some(DataCell::Text("A".to_owned())));
    assert_that!(cells.get(&key(kb.0, kb.1)).cloned()).is_equal_to(Some(DataCell::Text("B".to_owned())));
    assert_that!(b.deck().0).is_equal_to(got);
}

#[test]
fn concurrent_same_cell_edits_converge() {
    let deck = sample();
    let da = edited(&deck, "d1", |d| set_cell(&mut d.table, "r1", "c2", DataCell::Number(1.0)));
    let db = edited(&deck, "d1", |d| set_cell(&mut d.table, "r1", "c2", DataCell::Number(2.0)));
    let (a, b) = merged(&deck, &da, &db);
    let got = a.deck().0;
    let cell = table_of(&got, "d1").cells.get(&key("r1", "c2")).cloned();
    assert_that!(cell == Some(DataCell::Number(1.0)) || cell == Some(DataCell::Number(2.0))).is_true();
    assert_that!(b.deck().0).is_equal_to(got);
}

/// `table` with row `id` inserted after `after`.
fn insert_row(t: &mut Table, id: &str, after: &str) {
    let at = t.rows.index_of(after).unwrap() + 1;
    let mut ids: Vec<String> = t.rows.iter().map(str::to_owned).collect();
    ids.insert(at, id.to_owned());
    t.rows = Rows::repaired(ids);
}

#[test]
fn concurrent_row_inserts_after_same_row_both_kept() {
    let deck = sample();
    let da = edited(&deck, "d1", |d| insert_row(&mut d.table, "ra", "r1"));
    let db = edited(&deck, "d1", |d| insert_row(&mut d.table, "rb", "r1"));
    let (a, b) = merged(&deck, &da, &db);
    let got = a.deck().0;
    let rows: Vec<&str> = table_of(&got, "d1").rows.iter().collect();
    assert_that!(rows.len()).is_equal_to(5);
    assert_that!(rows[0]).is_equal_to("r1");
    assert_that!(rows[1..3].iter().copied().collect::<BTreeSet<_>>()).is_equal_to(BTreeSet::from(["ra", "rb"]));
    assert_that!(b.deck().0).is_equal_to(got);
}

#[test]
fn concurrent_row_insert_and_cell_edit_merge() {
    let deck = sample();
    let da = edited(&deck, "d1", |d| insert_row(&mut d.table, "ra", "r2"));
    let db = edited(&deck, "d1", |d| set_cell(&mut d.table, "r3", "c1", DataCell::Text("IT".to_owned())));
    let (a, b) = merged(&deck, &da, &db);
    let got = a.deck().0;
    let t = table_of(&got, "d1");
    assert_that!(t.rows.iter().collect::<Vec<_>>()).is_equal_to(vec!["r1", "r2", "ra", "r3"]);
    assert_that!(t.cells.get(&key("r3", "c1")).cloned()).is_equal_to(Some(DataCell::Text("IT".to_owned())));
    assert_that!(b.deck().0).is_equal_to(got);
}

#[test]
fn concurrent_column_remove_and_cell_edit_drops_orphan_cell() {
    let deck = sample();
    let da = edited(&deck, "d1", |d| {
        d.table.columns.retain(|c| c.id != "c2");
        d.table.canonicalize();
    });
    let db = edited(&deck, "d1", |d| set_cell(&mut d.table, "r2", "c2", DataCell::Number(7.0)));
    let (a, b) = merged(&deck, &da, &db);
    let got = a.deck().0;
    let t = table_of(&got, "d1");
    assert_that!(t.columns.iter().map(|c| c.id.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["c1"]);
    assert_that!(t.cells.keys().any(|k| k.column == "c2")).is_false();
    assert_that!(b.deck().0).is_equal_to(got);
}

#[test]
fn concurrent_rename_to_same_name_reports_name_taken() {
    let deck = deck_of(vec![("d1", data("sales", table())), ("d2", data("costs", table()))]);
    let da = edited(&deck, "d1", |d| d.name = "total".to_owned());
    let db = edited(&deck, "d2", |d| d.name = "Total".to_owned());
    let (a, b) = merged(&deck, &da, &db);
    let got = a.deck().0;
    let issues = validate(&got, &registry(), None);
    assert_that!(keys(&issues)).contains("issue-data-name-taken");
    assert_that!(b.deck().0).is_equal_to(got);
}

#[test]
fn concurrent_rename_column_to_different_names_converges() {
    let deck = sample();
    let da = edited(&deck, "d1", |d| d.table.columns[0].name = "Land".to_owned());
    let db = edited(&deck, "d1", |d| d.table.columns[0].name = "Pays".to_owned());
    let (a, b) = merged(&deck, &da, &db);
    let got = a.deck().0;
    let name = table_of(&got, "d1").columns[0].name.clone();
    assert_that!(name == "Land" || name == "Pays").is_true();
    assert_that!(b.deck().0).is_equal_to(got);
}

#[test]
fn concurrent_remove_data_and_edit_data_converge_without_data() {
    let deck = sample();
    let db = edited(&deck, "d1", |d| set_cell(&mut d.table, "r2", "c2", DataCell::Number(9.0)));
    let (a, b) = merged(&deck, &Deck::default(), &db);
    assert_that!(a.deck().0.data.is_empty()).is_true();
    assert_that!(b.deck().0.data.is_empty()).is_true();
}

#[test]
fn concurrent_add_data_same_name_reports_name_taken() {
    let deck = Deck::default();
    let da = deck_of(vec![("d1", data("sales", table()))]);
    let db = deck_of(vec![("d2", data("sales", table()))]);
    let (a, b) = merged(&deck, &da, &db);
    let got = a.deck().0;
    assert_that!(got.data.len()).is_equal_to(2);
    assert_that!(keys(&validate(&got, &registry(), None))).contains("issue-data-name-taken");
    assert_that!(b.deck().0).is_equal_to(got);
}

/// `t` replaced by a new generation `generation` with rows `rows` and no cells.
fn replace(t: &mut Table, generation: &str, rows: &[&str]) {
    t.generation = generation.to_owned();
    t.rows = Rows::repaired(rows.iter().map(|r| (*r).to_owned()));
    t.cells = imbl::OrdMap::new();
}

#[test]
fn concurrent_replace_is_last_writer_wins_without_duplicate_rows() {
    let deck = sample();
    let da = edited(&deck, "d1", |d| replace(&mut d.table, "ga", &["x1", "x2"]));
    let db = edited(&deck, "d1", |d| replace(&mut d.table, "gb", &["y1", "y2"]));
    let (a, b) = merged(&deck, &da, &db);
    let got = a.deck().0;
    let t = table_of(&got, "d1");
    let rows: Vec<&str> = t.rows.iter().collect();
    assert_that!(rows == ["x1", "x2"] || rows == ["y1", "y2"]).is_true();
    assert_that!(b.deck().0).is_equal_to(got);
}

#[test]
fn edit_on_replaced_generation_is_lost_reported_on_both_peers_and_docs_equal() {
    let deck = sample();
    let da = edited(&deck, "d1", |d| replace(&mut d.table, "g2", &["x1"]));
    let db = edited(&deck, "d1", |d| set_cell(&mut d.table, "r2", "c2", DataCell::Number(5.0)));
    let (mut a, mut b) = peers(&deck);
    a.apply(&da, "A", None).unwrap();
    b.apply(&db, "B", None).unwrap();
    let ra = a.merge(&b).unwrap();
    let rb = b.merge(&a).unwrap();
    assert_that!(ra.lost).is_equal_to(vec![did("d1")]);
    assert_that!(rb.lost).is_equal_to(vec![did("d1")]);
    assert_that!(a.deck().0).is_equal_to(b.deck().0);
    assert_that!(table_of(&a.deck().0, "d1").rows.iter().collect::<Vec<_>>()).is_equal_to(vec!["x1"]);
}

#[test]
fn concurrent_edits_of_one_generation_lose_nothing() {
    let deck = sample();
    let da = edited(&deck, "d1", |d| set_cell(&mut d.table, "r1", "c1", DataCell::Text("A".to_owned())));
    let db = edited(&deck, "d1", |d| set_cell(&mut d.table, "r2", "c2", DataCell::Number(5.0)));
    let (mut a, mut b) = peers(&deck);
    a.apply(&da, "A", None).unwrap();
    b.apply(&db, "B", None).unwrap();
    assert_that!(a.merge(&b).unwrap().lost).is_empty();
    assert_that!(b.merge(&a).unwrap().lost).is_empty();
}

#[test]
fn peer_update_over_replica_cap_is_refused() {
    let deck = sample();
    let snap = DeckReplica::new(&deck, registry()).unwrap().snapshot().unwrap();
    let (mut a, _) = DeckReplica::from_snapshot(&snap, snap.len() + 64, registry()).unwrap();
    let (mut b, _) = DeckReplica::from_snapshot(&snap, 1 << 26, registry()).unwrap();
    let big = edited(&deck, "d1", |d| set_cell(&mut d.table, "r1", "c1", DataCell::Text("x".repeat(4_096))));
    b.apply(&big, "B", None).unwrap();
    let err = a.merge(&b).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::ReplicaErrorKind::TooLarge);
    assert_that!(a.deck().0.data).is_equal_to(deck.data);
}

// --- Forged replicas -----------------------------------------------------------------------

/// A replica of `deck`, the doc edited by `forge`, then committed.
fn forged(deck: &Deck, forge: impl FnOnce(&DeckReplica)) -> DeckReplica {
    let r = DeckReplica::new(deck, registry()).unwrap();
    forge(&r);
    r.doc.commit();
    r
}

/// Ported from `host_tests::forged_duplicate_row_ids_are_repaired_on_read`.
#[test]
fn forged_data_duplicate_row_id_is_repaired_on_read() {
    let r = forged(&sample(), |r| child_list(&table_map(r, "d1"), ROWS).insert(2, "r1").map(drop).unwrap());
    let (got, issues) = reopened(&r);
    assert_that!(issues).is_empty();
    assert_that!(table_of(&got, "d1").rows.iter().collect::<Vec<_>>()).is_equal_to(vec!["r1", "r2", "r1_2", "r3"]);
}

/// Ported from `host_tests::forged_many_rows_with_one_id_and_distinct_cells_are_kept_with_repaired_ids`.
#[test]
fn forged_data_2000_rows_with_one_id_read_with_distinct_ids() {
    let r = forged(&sample(), |r| {
        let rows = child_list(&table_map(r, "d1"), ROWS);
        for _ in 0..2_000 {
            rows.push("r1").unwrap();
        }
    });
    let started = std::time::Instant::now();
    let (got, issues) = reopened(&r);
    assert_that!(started.elapsed() < std::time::Duration::from_secs(5)).is_true();
    assert_that!(issues).is_empty();
    let rows: BTreeSet<&str> = table_of(&got, "d1").rows.iter().collect();
    assert_that!(rows.len()).is_equal_to(2_003);
}

/// Ported from `host_tests::forged_duplicate_column_id_is_dropped_on_read`: now repaired.
#[test]
fn forged_data_duplicate_column_id_is_repaired_on_read() {
    let r = forged(&sample(), |r| {
        let m = child_list(&table_map(r, "d1"), COLUMNS).push_container(LoroMap::new()).unwrap();
        m.insert("id", "c1").unwrap();
        m.insert("name", "Twin").unwrap();
        m.insert("type", "bool").unwrap();
    });
    let (got, issues) = reopened(&r);
    assert_that!(issues).is_empty();
    let ids: Vec<String> = table_of(&got, "d1").columns.iter().map(|c| c.id.clone()).collect();
    assert_that!(ids).is_equal_to(vec!["c1".to_owned(), "c2".to_owned(), "c1_2".to_owned()]);
}

/// Ported from `host_tests::forged_cell_container_reads_as_null`: now an empty cell.
#[test]
fn forged_data_cell_container_reads_as_empty() {
    let r = forged(&sample(), |r| cells_map(r, "d1").insert_container("r1/c2", LoroMap::new()).map(drop).unwrap());
    let (got, issues) = reopened(&r);
    assert_that!(issues).is_empty();
    assert_that!(table_of(&got, "d1").cells.get(&key("r1", "c2"))).is_none();
}

#[test]
fn forged_duplicate_ids_repair_identically_on_two_peers() {
    let (mut a, b) = peers(&sample());
    let rows = child_list(&table_map(&a, "d1"), ROWS);
    rows.push("r2").unwrap();
    child_list(&table_map(&a, "d1"), COLUMNS).push_container(LoroMap::new()).unwrap().insert("id", "c2").unwrap();
    a.doc.commit();
    let mut b = b;
    b.merge(&a).unwrap();
    a.merge(&b).unwrap();
    let (got_a, got_b) = (a.deck().0, b.deck().0);
    assert_that!(got_a).is_equal_to(&got_b);
    assert_that!(table_of(&got_a, "d1").rows.iter().collect::<Vec<_>>()).is_equal_to(vec!["r1", "r2", "r3", "r2_2"]);
}

#[test]
fn forged_invalid_id_is_repaired_from_x() {
    let r = forged(&Deck::default(), |r| {
        let m = data_map(r).insert_container("bad id!", LoroMap::new()).unwrap();
        m.insert("name", "forged").unwrap();
    });
    let (got, _) = reopened(&r);
    assert_that!(got.data.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec![did("x")]);
    assert_that!(got.data[&did("x")].name.as_str()).is_equal_to("forged");
}

#[test]
fn forged_data_without_table_is_reported_truncated_and_never_written() {
    let mut r = forged(&sample(), |r| child_map(&data_map(r), "d1").delete(TABLE).unwrap());
    let (read, issues) = r.deck();
    assert_that!(keys(&issues)).is_equal_to(vec!["issue-data-truncated"]);
    assert_that!(read.truncated.contains(&did("d1"))).is_true();
    r.apply(&read, "again", None).unwrap();
    assert_that!(child_map(&data_map(&r), "d1").get(TABLE)).is_none();
}

#[test]
fn forged_overlay_with_unknown_row_is_dropped() {
    let r = forged(&sample(), |r| cells_map(r, "d1").insert("zz/c1", "ghost").map(drop).unwrap());
    let (got, issues) = reopened(&r);
    assert_that!(issues).is_empty();
    assert_that!(got.data).is_equal_to(sample().data);
}

#[test]
fn forged_unknown_column_type_reads_as_text_and_reports() {
    let r = forged(&sample(), |r| {
        let col = match child_list(&table_map(r, "d1"), COLUMNS).get(1) {
            Some(ValueOrContainer::Container(Container::Map(m))) => m,
            other => panic!("column expected: {other:?}"),
        };
        col.insert("type", "money").unwrap();
    });
    let (got, issues) = reopened(&r);
    assert_that!(table_of(&got, "d1").columns[1].ty).is_equal_to(ColumnType::Text);
    assert_that!(keys(&issues)).is_equal_to(vec!["issue-data-cell-type"]);
    let args = format!("{:?}", issues[0].message);
    assert_that!(args).contains("money");
    assert_that!(args).contains("sales");
}

#[test]
fn forged_label_and_column_name_with_bidi_are_cleaned_on_read() {
    let r = forged(&sample(), |r| {
        child_map(&data_map(r), "d1").insert("label", "a\u{202E}b").unwrap();
        let col = match child_list(&table_map(r, "d1"), COLUMNS).get(0) {
            Some(ValueOrContainer::Container(Container::Map(m))) => m,
            other => panic!("column expected: {other:?}"),
        };
        col.insert("name", "c\u{2066}d").unwrap();
    });
    let (got, _) = reopened(&r);
    let d = &got.data[&did("d1")];
    assert_that!(d.label.as_deref()).is_equal_to(Some("ab"));
    assert_that!(d.table.columns[0].name.as_str()).is_equal_to("cd");
}

#[test]
fn unparsable_base_reads_as_base_missing_and_read_only() {
    let r = forged(&sample(), |r| {
        child_map(&table_map(r, "d1"), LAYER).insert(BASE, "forged").map(drop).unwrap();
    });
    let (got, issues) = reopened(&r);
    assert_that!(keys(&issues)).is_equal_to(vec!["issue-data-base-missing"]);
    assert_that!(got.truncated.contains(&did("d1"))).is_true();
    assert_that!(table_of(&got, "d1").base.clone()).is_none();
}

#[test]
fn forged_snapshot_with_300_data_keeps_256_in_id_order_and_names_dropped() {
    let r = forged(&Deck::default(), |r| {
        let map = data_map(r);
        for i in 0..300 {
            map.insert_container(&format!("d{i:03}"), LoroMap::new()).unwrap().insert("name", format!("n{i}")).unwrap();
        }
    });
    let (got, issues) = reopened(&r);
    let kept: Vec<String> = got.data.keys().map(|k| k.as_str().to_owned()).collect();
    assert_that!(kept.len()).is_equal_to(MAX_DATA);
    assert_that!(kept.last().cloned()).is_equal_to(Some("d255".to_owned()));
    // The kept ones have no table: each is reported truncated, before the count of the rest.
    let mut expected = vec!["issue-data-truncated"; MAX_DATA];
    expected.push("issue-data-too-many");
    assert_that!(keys(&issues)).is_equal_to(expected);
    let args = format!("{:?}", issues[MAX_DATA].message);
    assert_that!(args).contains("d256");
    assert_that!(got.truncated.len()).is_equal_to(300);
}

#[test]
fn forged_view_referencing_280th_data_does_not_change_kept_set() {
    let r = forged(&Deck::default(), |r| {
        let map = data_map(r);
        for i in 0..300 {
            map.insert_container(&format!("d{i:03}"), LoroMap::new()).unwrap().insert("name", format!("n{i}")).unwrap();
        }
    });
    let plain = reopened(&r).0;
    // No view kind exists before 4b: the deck names `d280` the only way it can, a data edit.
    let deck = edited(&plain, "d000", |d| d.label = Some("d280".to_owned()));
    let mut r = r;
    r.apply(&deck, "view", None).unwrap();
    let (got, _) = reopened(&r);
    assert_that!(got.data.keys().collect::<Vec<_>>()).is_equal_to(plain.data.keys().collect::<Vec<_>>());
    assert_that!(data_map(&r).len()).is_equal_to(300);
}

#[test]
fn truncated_read_then_apply_does_not_write_truncation() {
    let deck =
        edited(&sample(), "d1", |d| set_cell(&mut d.table, "r2", "c1", DataCell::Text("é".repeat(MAX_TEXT_BYTES))));
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let (read, issues) = r.deck();
    assert_that!(keys(&issues)).is_equal_to(vec!["issue-data-truncated"]);
    r.apply(&read, "again", None).unwrap();
    let stored = match cells_map(&r, "d1").get("r2/c1") {
        Some(ValueOrContainer::Value(LoroValue::String(s))) => s.len(),
        other => panic!("text expected: {other:?}"),
    };
    assert_that!(stored).is_equal_to(2 * MAX_TEXT_BYTES);
}

/// A data `name` with `rows` rows, `columns` columns (`c0`…), and `overlay` cells of `text`
/// over the first column then the next, row by row.
fn big(name: &str, rows: usize, columns: usize, overlay: usize, text: &str) -> Arc<Data> {
    let cols: Vec<Column> = (0..columns).map(|j| column(&format!("c{j}"), "", ColumnType::Text)).collect();
    let cells = (0..overlay)
        .map(|n| (key(&format!("r{}", n / columns), &format!("c{}", n % columns)), DataCell::Text(text.to_owned())))
        .collect();
    Arc::new(Data {
        name: name.to_owned(),
        label: None,
        inline: false,
        kind: DataKind::Manual,
        table: Table {
            generation: "g".to_owned(),
            epoch: String::new(),
            columns: cols,
            rows: Rows::repaired((0..rows).map(|r| format!("r{r}"))),
            base: None,
            cells,
        },
    })
}

/// A deck past one bound, the data cut (its id) and what it keeps (rows, columns, overlay, text bytes).
fn over_bound(case: &str) -> (Deck, &'static str, (usize, usize, usize, usize)) {
    let many = |n: usize, d: Arc<Data>| -> Vec<(String, Arc<Data>)> {
        (0..n).map(|i| (format!("a{i:02}"), d.clone())).collect()
    };
    let (data, cut, kept): (Vec<(String, Arc<Data>)>, _, _) = match case {
        "rows" => (vec![("z".to_owned(), big("z", MAX_ROWS + 1, 1, 0, ""))], "z", (MAX_ROWS, 1, 0, 1)),
        "columns" => (vec![("z".to_owned(), big("z", 1, MAX_COLUMNS + 1, 0, ""))], "z", (1, MAX_COLUMNS, 0, 1)),
        "overlay" => (
            vec![("z".to_owned(), big("z", MAX_OVERLAY / 4 + 1, 4, MAX_OVERLAY + 1, ""))],
            "z",
            (MAX_OVERLAY / 4 + 1, 4, MAX_OVERLAY, 1),
        ),
        "deck-rows" => {
            let full = MAX_DECK_ROW_IDS / MAX_ROWS;
            let mut v = many(full, big("a", MAX_ROWS, 1, 0, ""));
            v.push(("z".to_owned(), big("z", 1, 1, 0, "")));
            (v, "z", (0, 1, 0, 1))
        }
        "deck-overlay" => {
            let half = MAX_DECK_OVERLAY / 2;
            let mut v = many(2, big("a", half / 3, 3, half, ""));
            v.push(("z".to_owned(), big("z", 1, 1, 1, "")));
            (v, "z", (1, 1, 0, 1))
        }
        "deck-text" => {
            let cells = MAX_DECK_TEXT_BYTES / MAX_TEXT_BYTES;
            let text = "x".repeat(MAX_TEXT_BYTES);
            let mut v = vec![("a00".to_owned(), big("", cells, 1, cells, &text))];
            v.push(("z".to_owned(), big("z", 1, 1, 0, "")));
            (v, "z", (1, 1, 0, 1))
        }
        "cell" => {
            let text = "x".repeat(MAX_TEXT_BYTES + 1);
            (vec![("z".to_owned(), big("z", 1, 1, 1, &text))], "z", (1, 1, 1, MAX_TEXT_BYTES + 1))
        }
        other => panic!("unknown case {other}"),
    };
    let deck = Deck { data: data.into_iter().map(|(id, d)| (did(&id), d)).collect(), ..Deck::default() };
    (deck, cut, kept)
}

#[rstest]
#[case::rows("rows")]
#[case::columns("columns")]
#[case::overlay("overlay")]
#[case::deck_rows("deck-rows")]
#[case::deck_overlay("deck-overlay")]
#[case::deck_text("deck-text")]
#[case::cell_text("cell")]
fn forged_snapshot_over_bound_truncates_and_reports(#[case] case: &str) {
    let (deck, cut, (rows, columns, overlay, text)) = over_bound(case);
    let r = DeckReplica::new(&deck, registry()).unwrap();
    let (got, issues) = reopened(&r);
    let t = table_of(&got, cut);
    assert_that!(t.rows.len()).is_equal_to(rows);
    assert_that!(t.columns.len()).is_equal_to(columns);
    assert_that!(t.overlay_len()).is_equal_to(overlay);
    assert_that!(t.text_bytes() + got.data[&did(cut)].name.len()).is_equal_to(text);
    assert_that!(keys(&issues)).is_equal_to(vec!["issue-data-truncated"]);
    assert_that!(format!("{:?}", issues[0].message)).contains(format!("\"{cut}\""));
    assert_that!(got.truncated.iter().cloned().collect::<Vec<_>>()).is_equal_to(vec![did(cut)]);
    let prefix = |t: &Table| t.rows.iter().map(str::to_owned).collect::<Vec<_>>();
    let wanted = prefix(table_of(&deck, cut));
    assert_that!(wanted.starts_with(&prefix(t))).is_true();
}

#[test]
fn format_6_snapshot_is_refused() {
    let r = DeckReplica::new(&sample(), registry()).unwrap();
    r.doc.get_map(super::super::META).insert(super::super::FORMAT, 6).unwrap();
    r.doc.commit();
    let err = DeckReplica::from_snapshot(&r.snapshot().unwrap(), 1 << 26, registry()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::ReplicaErrorKind::UnsupportedVersion);
}

#[test]
fn truncated_data_replaced_then_written_reads_back_whole() {
    let mut r = forged(&sample(), |r| child_map(&data_map(r), "d1").delete(TABLE).unwrap());
    let (mut read, _) = r.deck();
    assert_that!(read.truncated.contains(&did("d1"))).is_true();
    let table: oku_slides_data::NewTable = serde_json::from_value(serde_json::json!({
        "generation": "g2",
        "columns": [{ "id": "k", "name": "Kind" }],
        "rows": [{ "id": "q1", "cells": ["new"] }],
    }))
    .unwrap();
    let replace = crate::ops::Op::EditData { data: did("d1"), change: oku_slides_data::TableOp::Replace { table } };
    crate::ops::apply_all(&mut read, vec![replace], &registry()).unwrap();
    assert_that!(read.truncated.contains(&did("d1"))).is_false();
    r.apply(&read, "replace", None).unwrap();
    let (got, issues) = reopened(&r);
    assert_that!(issues).is_empty();
    assert_that!(got.truncated.contains(&did("d1"))).is_false();
    let written = &got.data[&did("d1")].table;
    assert_that!(written.generation.as_str()).is_equal_to("g2");
    assert_that!(written.cell("q1", "k")).is_equal_to(CellRead::Overlay(&DataCell::Text("new".to_owned())));
}

// --- Repairing a duplicate id on write -----------------------------------------------------

/// After `a` and `b` each wrote their deck read back from the merge of `da` and `db` (a cell
/// set on another row, so each write repairs the duplicate on its own), merged in the order
/// `a_first` says, then the other way.
fn repaired_by_both(da: &Deck, db: &Deck, a_first: bool) -> (DeckReplica, DeckReplica) {
    let (mut a, mut b) = merged(&sample(), da, db);
    let text = |s: &str| DataCell::Text(s.to_owned());
    let wa = edited(&a.deck().0, "d1", |d| set_cell(&mut d.table, "r1", "c1", text("A")));
    let wb = edited(&b.deck().0, "d1", |d| set_cell(&mut d.table, "r3", "c1", text("B")));
    a.apply(&wa, "A", None).unwrap();
    b.apply(&wb, "B", None).unwrap();
    if a_first {
        b.merge(&a).unwrap();
        a.merge(&b).unwrap();
    } else {
        a.merge(&b).unwrap();
        b.merge(&a).unwrap();
    }
    (a, b)
}

/// The raw ids of list `key` (`rows` or `columns`) of data `d1` in `r`'s doc.
fn raw_ids(r: &DeckReplica, key: &str) -> Vec<String> {
    let list = child_list(&table_map(r, "d1"), key);
    (0..list.len())
        .map(|i| match list.get(i) {
            Some(ValueOrContainer::Value(LoroValue::String(s))) => s.to_string(),
            _ => item_id(&list, i),
        })
        .collect()
}

#[rstest]
#[case::a_then_b(true)]
#[case::b_then_a(false)]
fn both_peers_repairing_one_duplicate_row_converge(#[case] a_first: bool) {
    let deck = sample();
    // Both insert row `rx`: the merged list holds it twice, read as `rx` and `rx_2`.
    let da = edited(&deck, "d1", |d| insert_row(&mut d.table, "rx", "r1"));
    let db = edited(&deck, "d1", |d| insert_row(&mut d.table, "rx", "r2"));
    let (a, b) = repaired_by_both(&da, &db, a_first);
    let rows: Vec<String> = table_of(&a.deck().0, "d1").rows.iter().map(str::to_owned).collect();
    let expected: Vec<String> = ["r1", "rx", "r2", "rx_2", "r3"].map(str::to_owned).to_vec();
    assert_that!((rows, raw_ids(&a, ROWS))).is_equal_to((expected.clone(), expected));
    assert_that!((a.deck().0, raw_ids(&a, ROWS))).is_equal_to((b.deck().0, raw_ids(&b, ROWS)));
}

#[rstest]
#[case::a_then_b(true)]
#[case::b_then_a(false)]
fn both_peers_repairing_one_duplicate_column_converge(#[case] a_first: bool) {
    let deck = sample();
    // Both add column `cx`, under two names: the merged list holds it twice.
    let da = edited(&deck, "d1", |d| d.table.columns.push(column("cx", "XA", ColumnType::Text)));
    let db = edited(&deck, "d1", |d| d.table.columns.push(column("cx", "XB", ColumnType::Text)));
    let (a, b) = repaired_by_both(&da, &db, a_first);
    let ids: Vec<String> = table_of(&a.deck().0, "d1").columns.iter().map(|c| c.id.clone()).collect();
    assert_that!((ids.len(), raw_ids(&a, COLUMNS))).is_equal_to((4, ids));
    assert_that!((a.deck().0, raw_ids(&a, COLUMNS))).is_equal_to((b.deck().0, raw_ids(&b, COLUMNS)));
}

#[path = "data_epoch_tests.rs"]
mod data_epoch_tests;

#[path = "data_undo_tests.rs"]
mod data_undo_tests;
