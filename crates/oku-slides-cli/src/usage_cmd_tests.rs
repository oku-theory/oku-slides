use oku_slides_core::{By, Issue, Range, UsageLedger, UsageRecord, aggregate};
use oku_slides_i18n::Localizer;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

/// A record read from its JSON line, as `usage.jsonl` holds it (no `time` dependency here).
fn record(turn: u128, at: &str, model: Option<&str>, input: u64, cost: Option<(f64, &str)>) -> UsageRecord {
    let line = serde_json::json!({
        "turn": format!("00000000-0000-0000-0000-{turn:012x}"),
        "at": at,
        "tab": "deck",
        "agent": "claude",
        "model": model,
        "tokens": { "total": input + 10, "input": input, "output": 10, "thought": null, "cache_read": 3, "cache_write": null },
        "cost": cost.map(|(amount, currency)| serde_json::json!({ "amount": amount, "currency": currency })),
    });
    serde_json::from_value(line).unwrap()
}

fn report(records: &[UsageRecord], by: By) -> UsageReport {
    aggregate(records.iter().map(|r| ("d", r)), by, Range::ALL)
}

#[rstest]
#[case(CliBy::Day, By::Day)]
#[case(CliBy::Month, By::Month)]
#[case(CliBy::Model, By::Model)]
#[case(CliBy::Agent, By::Agent)]
#[case(CliBy::Tab, By::Tab)]
#[case(CliBy::Deck, By::Deck)]
fn cli_by_maps_to_the_core_grouping_of_the_same_name(#[case] cli: CliBy, #[case] by: By) {
    assert_that!(By::from(cli)).is_equal_to(by);
}

#[rstest]
#[case("talks/q4.oku-slides.pdf", "talks/q4")]
#[case("talks/draft.json", "talks/draft")]
#[case("q4", "q4")]
fn deck_label_drops_the_deck_extension(#[case] path: &str, #[case] label: &str) {
    assert_that!(deck_label(std::path::Path::new(path))).is_equal_to(label.to_owned());
}

#[test]
fn once_per_turn_turn_in_two_decks_keeps_the_first_deck() {
    let shared = record(1, "2026-10-30T10:00:00Z", Some("m"), 100, None);
    let a: UsageLedger = [shared.clone()].into_iter().collect();
    let b: UsageLedger = [shared, record(2, "2026-10-30T11:00:00Z", Some("m"), 7, None)].into_iter().collect();
    let decks = vec![("a".to_owned(), a), ("b".to_owned(), b)];
    let kept: Vec<(&str, u128)> = once_per_turn(&decks).iter().map(|(l, r)| (*l, r.turn.as_u128())).collect();
    assert_that!(kept).is_equal_to(vec![("a", 1), ("b", 2)]);
}

#[test]
fn format_table_lines_share_one_width_and_end_with_the_total() {
    let l = Localizer::new(Some("en"));
    let rs = [
        record(1, "2026-10-30T10:00:00Z", Some("m"), 1234, None),
        record(2, "2026-10-31T10:00:00Z", Some("m"), 5, None),
    ];
    let text = format_table(&report(&rs, By::Day), &l);
    let lines: Vec<&str> = text.lines().collect();
    assert_that!(lines).has_length(4);
    assert_that!(lines[0]).starts_with(l.tr("usage-col-day").as_str());
    assert_that!(lines[3]).starts_with(l.tr("usage-row-total").as_str());
    let widths: Vec<usize> = lines.iter().map(|x| x.chars().count()).collect();
    assert_that!(widths.windows(2).all(|w| w[0] == w[1])).is_true();
}

#[test]
fn format_table_numbers_are_right_aligned_and_plain() {
    let l = Localizer::new(Some("en"));
    let rs = [
        record(1, "2026-10-30T10:00:00Z", Some("m"), 1234, None),
        record(2, "2026-10-31T10:00:00Z", Some("m"), 5, None),
    ];
    let text = format_table(&report(&rs, By::Day), &l);
    let total = text.lines().last().unwrap_or_default();
    assert_that!(total).contains("  1239  ");
}

#[test]
fn format_table_without_model_shows_the_localized_unknown_key_in_french() {
    let l = Localizer::new(Some("fr"));
    let text = format_table(&report(&[record(1, "2026-10-30T10:00:00Z", None, 1, None)], By::Model), &l);
    assert_that!(text.lines().nth(1).unwrap_or_default()).starts_with(l.tr("usage-unknown").as_str());
}

#[test]
fn format_table_row_without_cost_shows_a_placeholder() {
    let l = Localizer::new(Some("en"));
    let text = format_table(&report(&[record(1, "2026-10-30T10:00:00Z", Some("m"), 1, None)], By::Day), &l);
    assert_that!(text.lines().nth(1).unwrap_or_default().trim_end()).ends_with("–");
}

#[test]
fn format_table_partial_cost_uses_the_partial_message() {
    let l = Localizer::new(Some("fr"));
    let rs = [
        record(1, "2026-10-30T10:00:00Z", Some("m"), 1, Some((0.25, "USD"))),
        record(2, "2026-10-30T11:00:00Z", Some("m"), 1, None),
    ];
    let text = format_table(&report(&rs, By::Day), &l);
    let want = l.tr_with("usage-badge-cost-partial", &[("amount", "0.2500".into()), ("currency", "USD".into())]);
    assert_that!(text.lines().last().unwrap_or_default()).contains(want.as_str());
}

#[test]
fn format_table_keeps_currencies_apart_in_one_cell() {
    let l = Localizer::new(Some("en"));
    let rs = [
        record(1, "2026-10-30T10:00:00Z", Some("m"), 1, Some((0.25, "USD"))),
        record(2, "2026-10-30T11:00:00Z", Some("m"), 1, Some((1.5, "EUR"))),
    ];
    let total = format_table(&report(&rs, By::Day), &l).lines().last().unwrap_or_default().to_owned();
    assert_that!(total).contains("0.2500 USD");
    assert_that!(total).contains("1.5000 EUR");
}

#[rstest]
#[case("a\u{1b}[2Jb\u{7}.json", "a[2Jb")]
#[case("q4\u{202e}fdp.json", "q4fdp")]
#[case("a\u{2028}b\u{feff}c\u{200b}.oku-slides.pdf", "abc")]
fn deck_label_drops_characters_core_refuses(#[case] path: &str, #[case] shown: &str) {
    assert_that!(deck_label(std::path::Path::new(path))).is_equal_to(shown.to_owned());
}

#[test]
fn deck_label_drops_a_bidi_override_of_the_path() {
    assert_that!(deck_label(std::path::Path::new("a\u{202e}b.oku-slides.pdf"))).is_equal_to("ab".to_owned());
}

#[test]
fn usage_issues_keeps_only_the_ledger_issues() {
    let issues = vec![Issue::usage_line_invalid(2), Issue::pack_version_mismatch("abc"), Issue::usage_too_large()];
    let kept: Vec<&str> = usage_issues(&issues).iter().map(|i| i.message.id()).collect();
    assert_that!(kept).is_equal_to(vec!["issue-usage-line-invalid", "issue-usage-too-large"]);
}

#[rstest]
#[case::escape("a\u{1b}]8;;https://evil\u{7}b.oku-slides.pdf")]
#[case::bidi("a\u{202e}fdp.oku-slides.pdf")]
fn read_deck_usage_missing_hostile_path_error_is_one_filtered_line(#[case] name: &str) {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::local().unwrap();
    let registry = std::sync::Arc::new(oku_slides_core::WidgetRegistry::default());
    let limits = SourceLimits { replica_bytes: 1 << 20, total_bytes: 1 << 20, compact_after_secs: 0 };
    let path = dir.path().join(name);
    let Err(e) = read_deck_usage(&storage, &Localizer::new(Some("en")), &path, limits, &registry) else {
        panic!("a missing deck reads");
    };
    assert_that!(e.chain().count()).is_equal_to(1);
    let text = e.to_string();
    assert_that!(text.chars().any(|c| !oku_slides_core::shown_char(c))).is_false();
    assert_that!(text.as_str()).contains(shown_text(&path.display().to_string()).as_str());
}
