use oku_slides_core::{By, Range, UsageRecord, UsageReport, aggregate};
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{CappedReport, USAGE_JSON_MAX_ROWS};

/// `n` turns, one model each: `n` rows by model.
fn models(n: u128) -> UsageReport {
    let records: Vec<UsageRecord> = (0..n)
        .map(|i| {
            serde_json::from_value(json!({
                "turn": format!("00000000-0000-0000-0000-{i:012x}"), "at": "2026-10-30T14:02:11Z",
                "tab": "deck", "agent": "claude", "model": format!("m{i:05}"),
                "tokens": { "total": 2, "input": 1, "output": 1, "thought": null, "cache_read": null, "cache_write": null },
                "cost": null,
            }))
            .unwrap()
        })
        .collect();
    aggregate(records.iter().map(|r| ("", r)), By::Model, Range::ALL)
}

fn capped(n: u128) -> Value {
    serde_json::to_value(CappedReport::new(models(n))).unwrap()
}

#[test]
fn capped_report_past_the_cap_keeps_the_first_rows_and_counts_the_rest() {
    let full = models(1200);
    let v = capped(1200);
    let rows = v["rows"].as_array().unwrap();
    assert_that!(rows.len()).is_equal_to(USAGE_JSON_MAX_ROWS);
    assert_that!(rows.last().unwrap()["key"].as_str()).is_equal_to(full.rows[USAGE_JSON_MAX_ROWS - 1].key.as_deref());
    assert_that!(v["rows_omitted"].as_u64()).is_equal_to(Some(200));
}

#[test]
fn capped_report_past_the_cap_keeps_totals_over_every_row() {
    assert_that!(capped(1200)["totals"]["turns"].as_u64()).is_equal_to(Some(1200));
}

#[test]
fn capped_report_at_the_cap_has_no_rows_omitted_field() {
    let v = capped(1000);
    assert_that!(v["rows"].as_array().map(Vec::len)).is_equal_to(Some(1000));
    assert_that!(v.get("rows_omitted")).is_none();
    assert_that!(v.get("by")).is_some();
}
