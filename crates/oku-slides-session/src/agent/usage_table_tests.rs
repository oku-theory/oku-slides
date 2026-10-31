use oku_slides_core::{By, Range, UsageRecord, UsageReport, aggregate};
use oku_slides_i18n::Localizer;
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use super::{USAGE_COLUMNS, usage_cells, usage_headers, usage_total_cells};

/// A deck turn of `day`, with or without token counts; built from JSON, as read from a ledger.
pub(crate) fn record(n: u128, day: &str, model: &str, tokens: bool) -> UsageRecord {
    let tokens = tokens.then(
        || json!({ "total": 1850, "input": 1840, "output": 10, "thought": null, "cache_read": 5, "cache_write": null }),
    );
    serde_json::from_value(json!({
        "turn": Uuid::from_u128(n), "at": format!("{day}T14:02:11Z"), "tab": "deck", "agent": "claude",
        "model": model, "tokens": tokens, "cost": { "amount": 0.04, "currency": "USD" },
    }))
    .unwrap()
}

/// The report of one deck's `records`.
pub(crate) fn report(records: &[UsageRecord], by: By) -> UsageReport {
    aggregate(records.iter().map(|r| ("", r)), by, Range::ALL)
}

fn first_row(records: &[UsageRecord], by: By, l: &Localizer) -> [String; USAGE_COLUMNS] {
    usage_cells(&report(records, by).rows[0], l)
}

#[rstest]
#[case(By::Day, "Day")]
#[case(By::Month, "Month")]
#[case(By::Model, "Model")]
#[case(By::Agent, "Agent")]
#[case(By::Tab, "Tab")]
#[case(By::Deck, "Deck")]
fn usage_headers_key_column_names_the_grouping(#[case] by: By, #[case] expected: &str) {
    assert_that!(usage_headers(by, &Localizer::new(Some("en")))[0].as_str()).is_equal_to(expected);
}

#[test]
fn usage_headers_in_french_end_with_total_then_cost() {
    let h = usage_headers(By::Model, &Localizer::new(Some("fr")));
    assert_that!(h[7].as_str()).is_equal_to("Total");
    assert_that!(h[8].as_str()).is_equal_to("Coût");
}

#[test]
fn usage_cells_row_with_tokens_and_cost_fills_every_column() {
    let cells = first_row(&[record(1, "2026-10-30", "m", true)], By::Day, &Localizer::new(Some("en")));
    let expected = ["2026-10-30", "1", "1840", "10", "0", "5", "0", "1850", "0.0400 USD"].map(str::to_owned);
    assert_that!(cells).is_equal_to(expected);
}

#[test]
fn usage_cells_row_without_cost_shows_the_placeholder() {
    let mut r = record(1, "2026-10-30", "m", true);
    r.cost = None;
    assert_that!(first_row(&[r], By::Day, &Localizer::new(Some("en")))[8].as_str()).is_equal_to("–");
}

#[test]
fn usage_cells_partial_cost_says_so() {
    let mut no_cost = record(2, "2026-10-30", "m", true);
    no_cost.cost = None;
    let cells = first_row(&[record(1, "2026-10-30", "m", true), no_cost], By::Day, &Localizer::new(Some("en")));
    assert_that!(cells[8].as_str()).is_equal_to("0.0400 USD (partial)");
}

#[test]
fn usage_cells_row_without_model_is_unknown_in_french() {
    let mut r = record(1, "2026-10-30", "m", true);
    r.model = None;
    assert_that!(first_row(&[r], By::Model, &Localizer::new(Some("fr")))[0].as_str()).is_equal_to("inconnu");
}

#[test]
fn usage_cells_key_with_control_bidi_and_format_characters_drops_them() {
    // A record read without `is_valid` (as `aggregate` takes it): the cell still never reaches a
    // terminal with an escape sequence or a reordering mark.
    let hostile = "a\u{1b}[31mb\u{202E}c\u{200B}d\u{2028}e";
    let cells = first_row(&[record(1, "2026-10-30", hostile, true)], By::Model, &Localizer::new(Some("en")));
    assert_that!(cells[0].as_str()).is_equal_to("a[31mbcde");
}

#[test]
fn usage_total_cells_key_is_the_total_label() {
    let r = report(&[record(1, "2026-10-29", "m", true), record(2, "2026-10-30", "m", false)], By::Day);
    let cells = usage_total_cells(&r.totals, &Localizer::new(Some("fr")));
    assert_that!(cells[0].as_str()).is_equal_to("Total");
    assert_that!(cells[1].as_str()).is_equal_to("2");
}
