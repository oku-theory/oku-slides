//! t-1773: the query ops refuse a missing input or a cycle, but two peers can each make a
//! valid change that merges into one. The merged deck keeps both; its queries are then marked
//! broken, and validate reports each, rather than the merge failing or a query computing on
//! a loop.

use std::sync::Arc;

use oku_slides_core::{Deck, DeckReplica, Op, PlanOutcome, QueryFault, apply_all, plan_views, validate};
use oku_slides_data::DataId;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common;

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("invalid op: {e}"))
}

fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn did(id: &str) -> DataId {
    ok(DataId::try_from(id.to_owned()))
}

fn with_ops(deck: &Deck, ops: Vec<Op>) -> Deck {
    let mut next = deck.clone();
    ok(apply_all(&mut next, ops, &common::registry()));
    next
}

fn add_query(id: &str, name: &str, sql: &str) -> Op {
    op(json!({ "op": "add_data", "id": id, "name": name, "sql": sql }))
}

fn set_query(data: &str, sql: &str) -> Op {
    op(json!({ "op": "set_query", "data": data, "sql": sql }))
}

/// `deck` changed by `ops_a` on one peer and `ops_b` on another, both merged.
fn merged(deck: &Deck, ops_a: Vec<Op>, ops_b: Vec<Op>) -> Deck {
    let registry = Arc::new(common::registry());
    let snap = ok(ok(DeckReplica::new(deck, registry.clone())).snapshot());
    let open = || ok(DeckReplica::from_snapshot(&snap, 1 << 24, registry.clone())).0;
    let (mut a, mut b) = (open(), open());
    ok(a.apply(&with_ops(deck, ops_a), "A", None));
    ok(b.apply(&with_ops(deck, ops_b), "B", None));
    ok(a.merge(&b));
    ok(b.merge(&a));
    let (deck, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(b.deck().0).is_equal_to(&deck);
    deck
}

fn issue_ids(deck: &Deck) -> Vec<&'static str> {
    validate(deck, &common::registry(), None).iter().map(|i| i.message.id()).collect()
}

#[test]
fn merged_query_reading_removed_data_is_in_error_with_issue() {
    let deck = common::data_deck();
    let chart = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s" }, "param": "blocks" },
        "element": { "id": "g", "kind": "base/chart",
            "params": { "option": { "data": "q", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } } }));
    let deck = merged(
        &deck,
        vec![op(json!({ "op": "remove_data", "data": "d" }))],
        vec![add_query("q", "top", "SELECT Month FROM sales"), chart],
    );
    assert_that!(deck.data.contains_key(&did("d"))).is_false();
    let order = deck.query_order();
    assert_that!(matches!(&order.broken[&did("q")], QueryFault::MissingInput(n) if n == "sales")).is_true();
    assert_that!(order.order.is_empty()).is_true();
    assert_that!(issue_ids(&deck)).contains("issue-data-query-missing-input");
    // The chart over the broken query is planned as derived: the data issue names the fault.
    let (plans, _) = plan_views(&deck, &common::registry());
    assert_that!(plans.iter().map(|p| p.outcome.clone()).collect::<Vec<_>>()).is_equal_to(vec![PlanOutcome::Derived]);
}

#[test]
fn merged_cycle_marks_each_data_on_it() {
    let deck = with_ops(
        &common::data_deck(),
        vec![
            add_query("a", "first", "SELECT * FROM sales"),
            add_query("b", "second", "SELECT * FROM sales"),
            add_query("c", "third", "SELECT * FROM first"),
        ],
    );
    let deck = merged(&deck, vec![set_query("a", "SELECT * FROM second")], vec![set_query("b", "SELECT * FROM first")]);
    let order = deck.query_order();
    let broken: Vec<&DataId> = order.broken.keys().collect();
    assert_that!(broken).is_equal_to(vec![&did("a"), &did("b")]);
    assert_that!(order.broken.values().all(|f| matches!(f, QueryFault::Cycle))).is_true();
    assert_that!(order.order).is_equal_to(vec![did("c")]);
    let cycles = issue_ids(&deck).into_iter().filter(|id| *id == "issue-data-query-cycle").count();
    assert_that!(cycles).is_equal_to(2);
}
