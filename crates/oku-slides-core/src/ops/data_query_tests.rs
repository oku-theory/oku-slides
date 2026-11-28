//! Manual edits of a data derived from a query: refused, its rows being a result.

use oku_slides_data::limits::MAX_DECK_TEXT_BYTES;
use oku_slides_data::{Data, NewTable, Source};

use crate::data_bounds::DeckLoad;
use crate::error::DataFault;

use super::*;

/// `deck_with_data` with `d1` turned into a query data.
fn deck_with_query() -> Deck {
    let mut deck = deck_with_data();
    let query =
        Data { name: "sales".into(), label: None, inline: false, source: Source::Query { sql: "SELECT 1".into() } };
    deck.data.insert(did("d1"), Arc::new(query));
    deck
}

#[rstest]
#[case::edit(Op::EditData { data: did("d1"), change: TableOp::RemoveRow { row: "r1".into() } })]
#[case::replace(Op::EditData { data: did("d1"), change: TableOp::Replace { table: months() } })]
#[case::retype(retype("d1", "c2", ColumnType::Text))]
fn manual_edit_of_query_data_is_refused_as_derived(#[case] op: Op) {
    let mut deck = deck_with_query();
    let before = deck.clone();
    let err = refused(&mut deck, vec![op]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Derived));
    assert_that!(err.op_error().localize(&fr())).is_equal_to(fr().tr_with("op-data-derived", &[("data", q("d1"))]));
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn rename_of_query_data_is_kept() {
    let mut deck = deck_with_query();
    run(&mut deck, vec![rename("d1", "ventes")]).unwrap();
    assert_that!(deck.data[&did("d1")].name.as_str()).is_equal_to("ventes");
    assert_that!(deck.data[&did("d1")].sql()).is_equal_to(Some("SELECT 1"));
}

#[test]
fn projection_over_query_data_is_refused_as_derived_not_as_missing_column() {
    let mut deck = deck_with_query();
    let mut option = chart_ref("d1");
    option["columns"] = json!(["c1"]);
    let err = refused(&mut deck, vec![add_chart("g", &option)]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Derived));
    assert_that!(err.op_error().localize(&fr())).is_equal_to(fr().tr_with("op-data-derived", &[("data", q("d1"))]));
}

fn add_query(id: &str, name: &str, sql: &str) -> Op {
    Op::AddData { id: Some(did(id)), name: name.to_owned(), label: None, table: None, sql: Some(sql.to_owned()) }
}

fn set_query(data: &str, sql: &str) -> Op {
    Op::SetQuery { data: did(data), sql: sql.to_owned() }
}

/// `deck_with_data` and the query `d3` (top) reading `sales`.
fn deck_with_reader() -> Deck {
    let mut deck = deck_with_data();
    run(&mut deck, vec![add_query("d3", "top", "SELECT Month FROM sales WHERE sales > 10")]).unwrap();
    deck
}

#[test]
fn add_query_data_reading_existing_data_applies() {
    let deck = deck_with_reader();
    let data = &deck.data[&did("d3")];
    assert_that!(data.sql()).is_equal_to(Some("SELECT Month FROM sales WHERE sales > 10"));
    assert_that!(deck.data_readers(&did("d1"))).is_equal_to(vec![did("d3")]);
    assert_that!(deck.query_order().order).is_equal_to(vec![did("d3")]);
}

#[rstest]
#[case::both(Some(sales()), Some("SELECT * FROM sales"))]
#[case::neither(None, None)]
fn add_data_with_table_and_sql_is_refused(#[case] table: Option<NewTable>, #[case] sql: Option<&str>) {
    let mut deck = deck_with_data();
    let op = Op::AddData { id: Some(did("d3")), name: "x".into(), label: None, table, sql: sql.map(str::to_owned) };
    let err = refused(&mut deck, vec![op]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Source));
    assert_that!(err.op_error().localize(&fr())).is_equal_to(fr().tr("op-data-source"));
}

#[test]
fn add_query_with_unknown_input_is_refused_fr() {
    let mut deck = deck_with_data();
    let err = refused(&mut deck, vec![add_query("d3", "top", "SELECT * FROM Costs")]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::UnknownInput));
    let expected = fr().tr_with("op-data-unknown-input", &[("name", q("costs"))]);
    assert_that!(err.op_error().localize(&fr())).is_equal_to(expected);
}

#[test]
fn add_query_with_invalid_sql_is_refused() {
    let mut deck = deck_with_data();
    let err = refused(&mut deck, vec![add_query("d3", "top", "DELETE FROM sales")]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::Data(DataErrorKind::QueryConstruct));
    let reason = fr().tr_with("data-query-construct", &[("construct", q("DELETE"))]);
    assert_that!(err.op_error().localize(&fr())).contains(reason.as_str());
}

#[rstest]
#[case::through_other(vec![add_query("d4", "mid", "SELECT * FROM top")], "d3", "SELECT * FROM mid", "top")]
#[case::itself(vec![], "d3", "SELECT * FROM top", "top")]
fn set_query_making_cycle_is_refused_and_names_data(
    #[case] setup: Vec<Op>,
    #[case] data: &str,
    #[case] sql: &str,
    #[case] name: &str,
) {
    let mut deck = deck_with_reader();
    run(&mut deck, setup).unwrap();
    let err = refused(&mut deck, vec![set_query(data, sql)]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Cycle));
    assert_that!(err.op_error().localize(&fr())).is_equal_to(fr().tr_with("op-data-cycle", &[("data", q(name))]));
}

#[test]
fn set_query_replaces_the_sql() {
    let mut deck = deck_with_reader();
    run(&mut deck, vec![set_query("d3", "SELECT * FROM months")]).unwrap();
    assert_that!(deck.data[&did("d3")].sql()).is_equal_to(Some("SELECT * FROM months"));
}

#[test]
fn set_query_on_manual_data_is_refused() {
    let mut deck = deck_with_data();
    let err = refused(&mut deck, vec![set_query("d1", "SELECT * FROM months")]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::NotQuery));
    assert_that!(err.op_error().localize(&fr())).is_equal_to(fr().tr_with("op-data-not-query", &[("data", q("d1"))]));
}

#[test]
fn set_query_on_missing_data_is_refused() {
    let mut deck = deck_with_data();
    let err = refused(&mut deck, vec![set_query("d9", "SELECT * FROM months")]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Missing));
    assert_that!(err.op_error().localize(&fr())).is_equal_to(fr().tr_with("op-data-missing", &[("data", q("d9"))]));
}

#[test]
fn edit_data_on_query_data_is_refused() {
    let mut deck = deck_with_reader();
    let op = Op::EditData { data: did("d3"), change: TableOp::RemoveRow { row: "r1".into() } };
    let err = refused(&mut deck, vec![op]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Derived));
    assert_that!(err.op_error().localize(&fr())).is_equal_to(fr().tr_with("op-data-derived", &[("data", q("d3"))]));
}

#[test]
fn rename_data_rewrites_reader_sql() {
    let mut deck = deck_with_reader();
    run(&mut deck, vec![add_query("d4", "again", "SELECT * FROM top JOIN sales ON top.Month = sales.Month")]).unwrap();
    run(&mut deck, vec![rename("d1", "ventes")]).unwrap();
    let reads = |id: &str| oku_slides_data::sql::parse_query(deck.data[&did(id)].sql().unwrap()).unwrap().inputs();
    assert_that!(reads("d3").into_iter().collect::<Vec<_>>()).is_equal_to(vec!["ventes".to_owned()]);
    assert_that!(reads("d4").into_iter().collect::<Vec<_>>()).is_equal_to(vec!["top".to_owned(), "ventes".to_owned()]);
    assert_that!(deck.data_readers(&did("d1"))).is_equal_to(vec![did("d3"), did("d4")]);
}

#[test]
fn rename_data_closing_a_merged_cycle_is_refused() {
    // As a merge may leave it: a query reading a name no data has yet.
    let mut deck = deck_with_reader();
    let looping = Data {
        name: "loop".into(),
        label: None,
        inline: false,
        source: Source::Query { sql: "SELECT * FROM top".into() },
    };
    deck.data.insert(did("d4"), Arc::new(looping));
    let top = Data { source: Source::Query { sql: "SELECT * FROM later".into() }, ..(*deck.data[&did("d3")]).clone() };
    deck.data.insert(did("d3"), Arc::new(top));
    let err = refused(&mut deck, vec![rename("d4", "later")]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::Cycle));
    assert_that!(err.op_error().localize(&fr())).is_equal_to(fr().tr_with("op-data-cycle", &[("data", q("later"))]));
}

#[test]
fn remove_data_read_by_query_is_refused_and_lists_it() {
    let mut deck = deck_with_reader();
    run(&mut deck, vec![add_chart("g1", &chart_ref("d1"))]).unwrap();
    let err = refused(&mut deck, vec![remove("d1")]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::InUse));
    let args = [("queries", q("top")), ("shown", 1.into()), ("views", q("g1")), ("more", 0.into())];
    assert_that!(err.op_error().localize(&fr())).is_equal_to(fr().tr_with("op-data-read-by-queries", &args));
}

#[test]
fn remove_data_read_only_by_query_names_no_view() {
    let mut deck = deck_with_reader();
    let err = refused(&mut deck, vec![remove("d1")]);
    let fault = err.op_error().data_fault();
    assert_that!(fault).is_equal_to(Some(&DataFault::InUse { views: vec![], queries: vec!["top".into()], more: 0 }));
    let args = [("queries", q("top")), ("shown", 0.into()), ("views", q("")), ("more", 0.into())];
    assert_that!(err.op_error().localize(&fr())).is_equal_to(fr().tr_with("op-data-read-by-queries", &args));
    assert_that!(err.op_error().to_string()).is_equal_to("data is read by top".to_owned());
}

#[test]
fn remove_query_data_reading_itself_is_allowed() {
    let mut deck = deck_with_reader();
    deck.data.insert(
        did("d4"),
        Arc::new(Data {
            name: "me".into(),
            label: None,
            inline: false,
            source: Source::Query { sql: "SELECT * FROM me".into() },
        }),
    );
    run(&mut deck, vec![remove("d4")]).unwrap();
    assert_that!(deck.data.contains_key(&did("d4"))).is_false();
}

/// `deck` with a filler data whose label leaves `left` bytes of the deck text bound.
fn filled(mut deck: Deck, left: usize) -> Deck {
    let pad = Data { name: "pad".into(), label: None, ..(*deck.data[&did("d2")]).clone() };
    deck.data.insert(did("pad"), Arc::new(pad.clone()));
    let used = DeckLoad::of_deck(&deck).text;
    let pad = Data { label: Some("x".repeat(MAX_DECK_TEXT_BYTES - used - left)), ..pad };
    deck.data.insert(did("pad"), Arc::new(pad));
    deck
}

/// A query's SQL counts against the deck text bound, added or set.
#[rstest]
#[case::add_query(add_query("d4", "big", "SELECT Month FROM sales"), "big".len() + "SELECT Month FROM sales".len())]
#[case::set_query(
    set_query("d3", "SELECT Month FROM sales WHERE sales > 100"),
    "SELECT Month FROM sales WHERE sales > 100".len() - "SELECT Month FROM sales WHERE sales > 10".len()
)]
fn query_sql_counts_against_the_deck_text_bound(#[case] op: Op, #[case] grows: usize) {
    let mut fits = filled(deck_with_reader(), grows);
    run(&mut fits, vec![op.clone()]).unwrap();
    let mut over = filled(deck_with_reader(), grows - 1);
    let err = refused(&mut over, vec![op]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataDeckFull);
}

/// Two data named `sales`, as a merge may leave them.
fn deck_with_two_sales(reader: bool) -> Deck {
    let mut deck = if reader { deck_with_reader() } else { deck_with_data() };
    let twin = Data { name: "Sales".into(), ..(*deck.data[&did("d2")]).clone() };
    deck.data.insert(did("d5"), Arc::new(twin));
    deck
}

#[test]
fn rename_data_of_a_name_two_data_hold_and_queries_read_is_refused_fr() {
    let mut deck = deck_with_two_sales(true);
    let err = refused(&mut deck, vec![rename("d1", "ventes")]);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::NameAmbiguous));
    let expected = fr().tr_with("op-data-name-ambiguous", &[("name", q("sales"))]);
    assert_that!(err.op_error().localize(&fr())).is_equal_to(expected);
}

#[test]
fn rename_data_of_a_name_two_data_hold_and_no_query_reads_applies() {
    let mut deck = deck_with_two_sales(false);
    run(&mut deck, vec![rename("d5", "ventes")]).unwrap();
    assert_that!(deck.data[&did("d5")].name.as_str()).is_equal_to("ventes");
}
