//! `oku-slides usage`, run for real on saved decks whose sources hold a hand-written ledger.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::Output;

use common::{FIXTURE, oku_slides, workdir};
use oku_slides_core::Issue;
use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

fn run(cmd: &mut std::process::Command) -> Output {
    cmd.output().unwrap_or_else(|e| panic!("oku-slides ne se lance pas : {e}"))
}

/// One `usage.jsonl` line: `input` tokens on `at`, cost in USD when given.
fn line(turn: u128, at: &str, input: u64, usd: Option<f64>) -> String {
    serde_json::json!({
        "turn": format!("00000000-0000-0000-0000-{turn:012x}"),
        "at": at, "tab": "deck", "agent": "claude", "model": "claude-opus-5-5",
        "tokens": { "total": input + 1, "input": input, "output": 1, "thought": null, "cache_read": null, "cache_write": null },
        "cost": usd.map(|amount| serde_json::json!({ "amount": amount, "currency": "USD" })),
    })
    .to_string()
}

/// `FIXTURE` saved as `<dir>/<name>.oku-slides.pdf`, its sources holding `lines` as `usage.jsonl`.
fn deck_with_usage(dir: &Path, name: &str, lines: &[String]) -> PathBuf {
    let pdf = dir.join(format!("{name}.oku-slides.pdf"));
    let saved = run(oku_slides(dir, "en").args(["save", FIXTURE]).arg(&pdf));
    assert_that!(saved.status.success()).is_true();
    let bytes = std::fs::read(&pdf).unwrap_or_else(|e| panic!("lecture du PDF : {e}"));
    let found = oku_slides_pdf::extract_sources(&bytes)
        .unwrap_or_else(|e| panic!("sources du PDF : {e}"))
        .unwrap_or_else(|| panic!("PDF sans sources"));
    // `save` wrote no ledger (no agent turn): copy every entry, then add `usage.jsonl`.
    let mut src = zip::ZipArchive::new(Cursor::new(found.zip)).unwrap_or_else(|e| panic!("zip : {e}"));
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..src.len() {
        let entry = src.by_index_raw(i).unwrap_or_else(|e| panic!("entrée {i} : {e}"));
        w.raw_copy_file(entry).unwrap_or_else(|e| panic!("copie de l'entrée {i} : {e}"));
    }
    w.start_file("usage.jsonl", zip::write::SimpleFileOptions::default())
        .unwrap_or_else(|e| panic!("usage.jsonl : {e}"));
    w.write_all(lines.join("\n").as_bytes()).unwrap_or_else(|e| panic!("usage.jsonl : {e}"));
    let zip = w.finish().unwrap_or_else(|e| panic!("zip : {e}")).into_inner();
    let pdf_bytes = oku_slides_pdf::embed_sources(&bytes, &zip).unwrap_or_else(|e| panic!("embed : {e}"));
    std::fs::write(&pdf, pdf_bytes).unwrap_or_else(|e| panic!("écriture du PDF : {e}"));
    pdf
}

/// `usage --json` of `decks` with `extra` arguments, parsed.
fn usage_json(dir: &Path, decks: &[&Path], extra: &[&str]) -> serde_json::Value {
    let out = run(oku_slides(dir, "en").arg("usage").args(decks).args(extra).arg("--json"));
    assert_that!(out.status.success()).is_true();
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("JSON : {e}"))
}

fn keys(report: &serde_json::Value) -> Vec<String> {
    report["rows"]
        .as_array()
        .map_or_else(Vec::new, |rows| rows.iter().map(|r| r["key"].as_str().unwrap_or("?").to_owned()).collect())
}

#[test]
fn usage_by_deck_over_two_decks_keys_rows_by_path_without_extension() {
    let dir = workdir("usage-by-deck");
    let a = deck_with_usage(&dir, "alpha", &[line(1, "2026-10-30T10:00:00Z", 1234, None)]);
    let b = deck_with_usage(&dir, "beta", &[line(2, "2026-10-30T11:00:00Z", 4321, None)]);
    let report = usage_json(&dir, &[&a, &b], &["--by", "deck"]);
    let label = |n: &str| dir.join(n).display().to_string();
    assert_that!(keys(&report)).is_equal_to(vec![label("alpha"), label("beta")]);
    assert_that!(report["totals"]["input"].as_u64()).is_equal_to(Some(5555));
}

#[test]
fn usage_turn_in_two_decks_counts_once() {
    let dir = workdir("usage-dedup");
    let shared = line(1, "2026-10-30T10:00:00Z", 1234, Some(0.25));
    let a = deck_with_usage(&dir, "original", std::slice::from_ref(&shared));
    let b = deck_with_usage(&dir, "copie", &[shared, line(2, "2026-10-30T11:00:00Z", 7, None)]);
    let report = usage_json(&dir, &[&a, &b], &[]);
    assert_that!(report["totals"]["turns"].as_u64()).is_equal_to(Some(2));
    assert_that!(report["totals"]["input"].as_u64()).is_equal_to(Some(1241));
}

#[test]
fn usage_since_keeps_only_the_days_from_it() {
    let dir = workdir("usage-since");
    let a = deck_with_usage(
        &dir,
        "d",
        &[line(1, "2026-10-29T23:59:00Z", 1000, None), line(2, "2026-10-30T00:01:00Z", 37, None)],
    );
    let report = usage_json(&dir, &[&a], &["--since", "2026-10-30"]);
    assert_that!(keys(&report)).is_equal_to(vec!["2026-10-30".to_owned()]);
    assert_that!(report["totals"]["input"].as_u64()).is_equal_to(Some(37));
}

#[test]
fn usage_json_deck_without_ledger_reports_no_turn() {
    let dir = workdir("usage-none");
    let report = usage_json(&dir, &[Path::new(FIXTURE)], &[]);
    assert_that!(report["totals"]["turns"].as_u64()).is_equal_to(Some(0));
    assert_that!(keys(&report)).is_empty();
}

#[test]
fn usage_invalid_line_is_logged_once_and_the_command_succeeds() {
    let dir = workdir("usage-invalid");
    let a = deck_with_usage(&dir, "d", &[line(1, "2026-10-30T10:00:00Z", 12, None), "{pas du json}".to_owned()]);
    let out = run(oku_slides(&dir, "en").arg("usage").arg(&a).arg("--json"));
    assert_that!(out.status.success()).is_true();
    let issue = Issue::usage_line_invalid(2).localize(&Localizer::new(Some("en")));
    assert_that!(String::from_utf8_lossy(&out.stderr).matches(issue.as_str()).count()).is_equal_to(1);
}

#[test]
fn usage_logs_no_widget_issue_of_the_deck() {
    // Read with an empty registry, every widget of the fixture is unknown: none of that is logged.
    let dir = workdir("usage-quiet");
    let a = deck_with_usage(&dir, "d", &[line(1, "2026-10-30T10:00:00Z", 12, None)]);
    let out = run(oku_slides(&dir, "en").arg("usage").arg(&a).arg("--json"));
    assert_that!(out.status.success()).is_true();
    let prefix = Localizer::new(Some("en")).tr_with("cli-deck-issue", &[("issue", "".into())]);
    assert_that!(String::from_utf8_lossy(&out.stderr).contains(prefix.trim_end())).is_false();
}

#[test]
fn usage_since_after_until_fails_without_a_report() {
    let dir = workdir("usage-range");
    let out = run(oku_slides(&dir, "fr").args(["usage", FIXTURE, "--since", "2026-10-31", "--until", "2026-10-30"]));
    assert_that!(out.status.success()).is_false();
    assert_that!(out.stdout.is_empty()).is_true();
}

#[test]
fn usage_unreadable_day_fails_without_a_report() {
    let dir = workdir("usage-day");
    let out = run(oku_slides(&dir, "en").args(["usage", FIXTURE, "--since", "30/10/2026"]));
    assert_that!(out.status.success()).is_false();
    assert_that!(out.stdout.is_empty()).is_true();
}

#[test]
fn usage_table_in_french_has_localized_headers_and_total() {
    let dir = workdir("usage-table");
    let a = deck_with_usage(&dir, "d", &[line(1, "2026-10-30T10:00:00Z", 4321, Some(0.4321))]);
    let out = run(oku_slides(&dir, "fr").arg("usage").arg(&a));
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let fr = Localizer::new(Some("fr"));
    assert_that!(text.lines().next().unwrap_or_default()).starts_with(fr.tr("usage-col-day").as_str());
    let total = text.lines().last().unwrap_or_default().to_owned();
    assert_that!(total).starts_with(fr.tr("usage-row-total").as_str());
    assert_that!(total).contains("4321");
    assert_that!(total).contains("0.4321 USD");
}
