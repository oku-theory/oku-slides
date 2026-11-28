//! t-1773: `rename_data` rewrites each reading query to the new name. When a query already
//! gives that name to one of its own relations (a `WITH` table, an alias), the rewritten
//! query would read that relation instead of the data, or name two relations alike. The
//! rename is refused, naming the query, rather than silently changing what the query reads.

use oku_slides_core::{DataFaultKind, Deck, Op, OpErrorKind, apply_all};
use oku_slides_i18n::{FluentValue, Localize, Localizer};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common;

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("invalid op: {e}"))
}

/// The data deck with the query `top` reading `sales` as `sql`.
fn deck_with_reader(sql: &str) -> Deck {
    let mut deck = common::data_deck();
    let add = op(json!({ "op": "add_data", "id": "q", "name": "top", "sql": sql }));
    apply_all(&mut deck, vec![add], &common::registry()).unwrap_or_else(|e| panic!("{e}"));
    deck
}

fn assert_rename_captured(sql: &str) {
    let mut deck = deck_with_reader(sql);
    let before = deck.clone();
    let rename = op(json!({ "op": "rename_data", "data": "d", "name": "ventes" }));
    let Err(err) = apply_all(&mut deck, vec![rename], &common::registry()) else {
        panic!("the rename must be refused")
    };
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::DataRefused(DataFaultKind::RenameCaptured));
    let fr = Localizer::new(Some("fr"));
    let args = [("reader", FluentValue::from("top")), ("name", FluentValue::from("ventes"))];
    assert_that!(err.op_error().localize(&fr)).is_equal_to(fr.tr_with("op-data-rename-captured", &args));
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn rename_to_a_with_table_name_of_a_reader_is_refused() {
    assert_rename_captured(
        "WITH ventes AS (SELECT 'Jan' AS m) SELECT * FROM sales JOIN ventes ON sales.Month = ventes.m",
    );
}

#[test]
fn rename_to_an_alias_of_a_reader_is_refused() {
    assert_rename_captured("SELECT sales.Month FROM sales JOIN sales AS ventes ON sales.Month = ventes.Month");
}
