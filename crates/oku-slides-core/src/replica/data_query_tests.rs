//! Query data in the replica: a `kind` read back, a `sql` atom (last-writer-wins), no table.

use oku_slides_data::limits::MAX_SQL_BYTES;
use oku_slides_i18n::{Localize, Localizer};

use super::*;

/// A query data named `name`.
fn query(name: &str, sql: &str) -> Arc<Data> {
    Arc::new(Data { name: name.to_owned(), label: None, inline: false, source: Source::Query { sql: sql.to_owned() } })
}

/// A deck with the manual data `d1` and the query data `q1` over it.
fn with_query() -> Deck {
    deck_of(vec![("d1", data("sales", table())), ("q1", query("top", "SELECT * FROM sales"))])
}

/// `deck` with the SQL of `q1` set to `sql`.
fn set_sql(deck: &Deck, sql: &str) -> Deck {
    edited(deck, "q1", |d| d.source = Source::Query { sql: sql.to_owned() })
}

#[test]
fn query_data_round_trips_through_replica() {
    let deck = with_query();
    let r = DeckReplica::new(&deck, registry()).unwrap();
    let (got, issues) = reopened(&r);
    assert_that!(issues).is_empty();
    assert_that!(got.data).is_equal_to(deck.data);
    let q1 = child_map(&data_map(&r), "q1");
    assert_that!(q1.get(TABLE)).is_none();
    assert_that!(q1.get(KIND).map(|v| v.into_value().ok())).is_equal_to(Some(Some(LoroValue::from("query"))));
}

#[test]
fn concurrent_set_sql_resolves_last_writer() {
    let deck = with_query();
    let (a, b) = merged(&deck, &set_sql(&deck, "SELECT 1"), &set_sql(&deck, "SELECT 2"));
    let (got_a, got_b) = (a.deck(), b.deck());
    assert_that!(got_a.1).is_empty();
    assert_that!(got_a.0).is_equal_to(&got_b.0);
    let sql = got_a.0.data[&did("q1")].sql().map(str::to_owned);
    assert_that!(sql.as_deref() == Some("SELECT 1") || sql.as_deref() == Some("SELECT 2")).is_true();
}

#[test]
fn unknown_kind_is_dropped_with_issue() {
    let mut r = forged(&with_query(), |r| child_map(&data_map(r), "q1").insert(KIND, "linked").unwrap());
    let (read, issues) = r.deck();
    assert_that!(keys(&issues)).is_equal_to(vec!["issue-data-kind-unknown"]);
    assert_that!(format!("{:?}", issues[0].message)).contains("\"linked\"");
    assert_that!(read.data.keys().collect::<Vec<_>>()).is_equal_to(vec![&did("d1")]);
    // Never written back: a newer peer's data survives this one's writes.
    assert_that!(read.truncated.contains(&did("q1"))).is_true();
    r.apply(&read, "again", None).unwrap();
    assert_that!(child_map(&data_map(&r), "q1").get(KIND).map(|v| v.into_value().ok()))
        .is_equal_to(Some(Some(LoroValue::from("linked"))));
}

#[test]
fn missing_kind_is_dropped_with_issue() {
    let r = forged(&sample(), |r| child_map(&data_map(r), "d1").delete(KIND).unwrap());
    let (read, issues) = reopened(&r);
    assert_that!(keys(&issues)).is_equal_to(vec!["issue-unreadable-field"]);
    assert_that!(format!("{:?}", issues[0].message)).contains("\"data/d1/kind\"");
    assert_that!(read.data.is_empty()).is_true();
    assert_that!(read.truncated.contains(&did("d1"))).is_true();
}

#[test]
fn kind_switch_in_place_is_refused_before_any_write() {
    let deck = with_query();
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let before = r.doc.state_frontiers();
    let turned = edited(&deck, "d1", |d| d.source = Source::Query { sql: "SELECT 1".to_owned() });
    let err = r.apply(&turned, "query", None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::ReplicaErrorKind::Write);
    let fr = Localizer::new(Some("fr"));
    assert_that!(err.localize(&fr)).is_equal_to(fr.tr("replica-write"));
    assert_that!(r.doc.state_frontiers()).is_equal_to(before);
    assert_that!(child_map(&data_map(&r), "d1").get(TABLE)).is_some();
}

/// A peer switching a data to query while another edits its cells: the switch is refused, so
/// the cell edit is never lost silently.
#[test]
fn switch_to_query_concurrent_with_a_cell_edit_is_refused_and_keeps_the_edit() {
    let deck = with_query();
    let (mut a, mut b) = peers(&deck);
    let edit = edited(&deck, "d1", |d| {
        set_cell(d.table_mut().unwrap(), "r1", "c1", DataCell::Text("edited".to_owned()));
    });
    a.apply(&edit, "edit", None).unwrap();
    let turned = edited(&deck, "d1", |d| d.source = Source::Query { sql: "SELECT 1".to_owned() });
    assert_that!(b.apply(&turned, "query", None).map_err(|e| e.kind()))
        .is_equal_to(Err(crate::ReplicaErrorKind::Write));
    let report = b.merge(&a).unwrap();
    let (got, issues) = b.deck();
    assert_that!(issues).is_empty();
    assert_that!(report.lost.is_empty()).is_true();
    assert_that!(got.data).is_equal_to(edit.data);
}

#[test]
fn query_data_sql_over_max_bytes_is_cut_and_kept_read_only() {
    let long = "x".repeat(MAX_SQL_BYTES + 10);
    let r = forged(&with_query(), |r| child_map(&data_map(r), "q1").insert(SQL, long.as_str()).unwrap());
    let (read, issues) = reopened(&r);
    assert_that!(keys(&issues)).is_equal_to(vec!["issue-data-truncated"]);
    assert_that!(read.data[&did("q1")].sql().map(str::len)).is_equal_to(Some(MAX_SQL_BYTES));
    assert_that!(read.truncated.contains(&did("q1"))).is_true();
}

/// A format 9 snapshot, whose data `kind` was assumed manual, is refused, never upgraded.
#[test]
fn old_replica_format_9_is_refused() {
    let r = DeckReplica::new(&with_query(), registry()).unwrap();
    r.doc.get_map(super::super::super::META).insert(super::super::super::FORMAT, 9).unwrap();
    r.doc.commit();
    let err = DeckReplica::from_snapshot(&r.snapshot().unwrap(), 1 << 26, registry()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::ReplicaErrorKind::UnsupportedVersion);
}
