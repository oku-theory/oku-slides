use std::sync::Arc;

use oku_slides_data::{Data, DataId, NewTable, Source};
use serde_json::json;
use speculoos::prelude::*;

use super::*;

fn did(id: &str) -> DataId {
    DataId::try_from(id.to_owned()).unwrap()
}

fn manual(name: &str) -> Data {
    let table: NewTable =
        serde_json::from_value(json!({ "generation": "g1", "columns": [{ "id": "c1", "name": "x" }], "rows": [] }))
            .unwrap();
    Data { name: name.into(), label: None, inline: false, source: Source::Manual(table.into_table().unwrap().0) }
}

fn query(name: &str, sql: &str) -> Data {
    Data { name: name.into(), label: None, inline: false, source: Source::Query { sql: sql.into() } }
}

/// A deck holding `data`, keyed `d1`, `d2`… in order.
fn deck(data: Vec<Data>) -> Deck {
    let mut deck = Deck::from_json(r#"{"slides":[]}"#).unwrap();
    for (i, d) in data.into_iter().enumerate() {
        deck.data.insert(did(&format!("d{}", i + 1)), Arc::new(d));
    }
    deck
}

fn ids(list: &[&str]) -> Vec<DataId> {
    list.iter().map(|id| did(id)).collect()
}

#[test]
fn query_order_chain_puts_inputs_first() {
    let deck = deck(vec![
        query("top", "SELECT * FROM mid"),
        query("mid", "SELECT * FROM base"),
        manual("base"),
        query("other", "SELECT * FROM base"),
    ]);
    let order = deck.query_order();
    assert_that!(order.order).is_equal_to(ids(&["d2", "d1", "d4"]));
    assert_that!(order.broken.is_empty()).is_true();
}

#[test]
fn query_order_cycle_marks_each_data_on_it_not_its_reader() {
    let deck = deck(vec![
        query("a", "SELECT * FROM b"),
        query("b", "SELECT * FROM a"),
        query("c", "SELECT * FROM a"),
        query("me", "SELECT * FROM me"),
    ]);
    let order = deck.query_order();
    let broken: Vec<_> = order.broken.keys().cloned().collect();
    assert_that!(broken).is_equal_to(ids(&["d1", "d2", "d4"]));
    assert_that!(order.broken.values().all(|f| matches!(f, QueryFault::Cycle))).is_true();
    assert_that!(order.order).is_equal_to(ids(&["d3"]));
}

#[test]
fn query_order_missing_input_and_invalid_are_broken() {
    let deck = deck(vec![query("a", "SELECT * FROM Gone"), query("b", "DELETE FROM a"), manual("base")]);
    let order = deck.query_order();
    assert_that!(matches!(&order.broken[&did("d1")], QueryFault::MissingInput(n) if n == "gone")).is_true();
    let QueryFault::Invalid(reason) = &order.broken[&did("d2")] else { panic!("not invalid") };
    assert_that!(reason.id()).is_equal_to("data-query-construct");
    assert_that!(order.order.is_empty()).is_true();
    assert_that!(order.clone()).is_equal_to(order);
}

#[test]
fn query_order_cycle_overrides_missing_input() {
    let deck = deck(vec![query("a", "SELECT * FROM a JOIN gone ON a.x = gone.x")]);
    assert_that!(matches!(deck.query_order().broken[&did("d1")], QueryFault::Cycle)).is_true();
}

#[test]
fn data_readers_lists_queries_reading_the_name_case_ignored() {
    let deck = deck(vec![
        manual("Sales"),
        query("a", "SELECT * FROM sales"),
        query("b", "SELECT * FROM SALES"),
        query("c", "SELECT * FROM other"),
        query("d", "not sql"),
    ]);
    assert_that!(deck.data_readers(&did("d1"))).is_equal_to(ids(&["d2", "d3"]));
    assert_that!(deck.data_readers(&did("d9")).is_empty()).is_true();
}

#[test]
fn reads_on_cycle_follows_queries_only() {
    let deck = deck(vec![query("a", "SELECT * FROM b"), query("b", "SELECT * FROM a"), manual("c")]);
    let reads = Reads::new(&deck);
    assert_that!(reads.on_cycle(&did("d1"))).is_true();
    assert_that!(reads.on_cycle(&did("d3"))).is_false();
}
