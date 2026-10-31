use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;
use time::OffsetDateTime;
use time::macros::{date, datetime};

use super::*;
use crate::usage::usage_tests::record;
use crate::usage::{UsageCost, UsageRecord, UsageTab};

fn at(mut r: UsageRecord, when: OffsetDateTime) -> UsageRecord {
    r.at = when;
    r
}

fn report(rs: &[UsageRecord], by: By, range: Range) -> UsageReport {
    aggregate(rs.iter().map(|r| ("d", r)), by, range)
}

#[test]
fn aggregate_by_day_sums_tokens_per_utc_day() {
    let rs = [at(record(1, 100), datetime!(2026-10-30 23:59 UTC)), at(record(2, 50), datetime!(2026-10-31 00:01 UTC))];
    let r = report(&rs, By::Day, Range::ALL);
    let keys: Vec<_> = r.rows.iter().map(|row| row.key.clone()).collect();
    assert_that!(keys).is_equal_to(vec![Some("2026-10-30".to_owned()), Some("2026-10-31".to_owned())]);
    assert_that!((r.totals.input, r.totals.total)).is_equal_to((150, 170));
}

#[rstest]
#[case::month(By::Month, "2026-10")]
#[case::model(By::Model, "claude-opus-5-5")]
#[case::agent(By::Agent, "claude")]
#[case::tab(By::Tab, "deck")]
#[case::deck(By::Deck, "d")]
fn aggregate_groups_by_key(#[case] by: By, #[case] key: &str) {
    let r = report(&[record(1, 1)], by, Range::ALL);
    assert_that!(r.rows[0].key.as_deref()).is_equal_to(Some(key));
}

#[test]
fn aggregate_by_tab_names_pack_and_theme() {
    let mut pack = record(2, 1);
    pack.tab = UsageTab::Pack;
    let mut theme = record(3, 1);
    theme.tab = UsageTab::Theme;
    let r = report(&[record(1, 1), pack, theme], By::Tab, Range::ALL);
    let keys: Vec<_> = r.rows.iter().filter_map(|row| row.key.clone()).collect();
    assert_that!(keys).is_equal_to(vec!["deck".to_owned(), "pack".to_owned(), "theme".to_owned()]);
}

#[test]
fn aggregate_by_deck_keeps_decks_apart() {
    let (a, b) = (record(1, 1), record(2, 1));
    let r = aggregate([("a", &a), ("b", &b)], By::Deck, Range::ALL);
    assert_that!(r.rows).has_length(2);
}

#[test]
fn aggregate_without_model_has_unknown_key_first() {
    let mut r1 = record(1, 1);
    r1.model = None;
    let r = report(&[record(2, 1), r1], By::Model, Range::ALL);
    assert_that!(r.rows[0].key).is_none();
}

#[test]
fn aggregate_keeps_currencies_apart() {
    let mut eur = record(2, 1);
    eur.cost = Some(UsageCost { amount: 1.0, currency: "EUR".into() });
    let r = report(&[record(1, 1), eur], By::Day, Range::ALL);
    let currencies: Vec<_> = r.totals.costs.iter().map(|c| c.currency.as_str()).collect();
    assert_that!(currencies).is_equal_to(vec!["EUR", "USD"]);
}

#[test]
fn aggregate_sums_one_currency() {
    let r = report(&[record(1, 1), record(2, 1)], By::Day, Range::ALL);
    assert_that!(r.totals.costs).has_length(1);
    assert_that!((r.totals.costs[0].amount - 0.08).abs()).is_less_than(1e-9);
    assert_that!(r.totals.costs[0].partial).is_false();
}

#[test]
fn aggregate_missing_cost_marks_partial() {
    let mut free = record(2, 1);
    free.cost = None;
    let r = report(&[record(1, 1), free], By::Day, Range::ALL);
    assert_that!(r.totals.costs[0].partial).is_true();
}

#[test]
fn aggregate_counts_turns_without_tokens() {
    let mut blind = record(2, 1);
    blind.tokens = None;
    let r = report(&[record(1, 1), blind], By::Day, Range::ALL);
    assert_that!((r.totals.turns, r.totals.turns_without_tokens)).is_equal_to((2, 1));
}

#[test]
fn aggregate_unreported_token_kinds_count_as_zero() {
    let r = report(&[record(1, 1)], By::Day, Range::ALL);
    assert_that!((r.totals.thought, r.totals.cache_read, r.totals.cache_write)).is_equal_to((0, 5, 0));
}

#[test]
fn aggregate_range_is_inclusive() {
    let rs = [
        at(record(1, 1), datetime!(2026-10-29 12:00 UTC)),
        at(record(2, 1), datetime!(2026-10-30 00:00 UTC)),
        at(record(3, 1), datetime!(2026-10-31 23:59 UTC)),
        at(record(4, 1), datetime!(2026-11-01 00:00 UTC)),
    ];
    let range = Range::new(Some(date!(2026 - 10 - 30)), Some(date!(2026 - 10 - 31))).unwrap();
    assert_that!(report(&rs, By::Day, range).totals.turns).is_equal_to(2);
}

#[test]
fn aggregate_empty_returns_no_rows_and_zero_totals() {
    let r = report(&[], By::Day, Range::ALL);
    assert_that!(r.rows).is_empty();
    assert_that!(r.totals.turns).is_equal_to(0);
}

#[test]
fn range_since_after_until_is_refused() {
    let err = Range::new(Some(date!(2026 - 10 - 31)), Some(date!(2026 - 10 - 30))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(UsageRangeErrorKind::Reversed);
}

#[rstest]
#[case::open(None, None)]
#[case::since_only(Some("2026-10-30"), None)]
#[case::until_only(None, Some("2026-10-30"))]
#[case::same_day(Some("2026-10-30"), Some("2026-10-30"))]
fn range_parse_valid_bounds_is_ok(#[case] since: Option<&str>, #[case] until: Option<&str>) {
    assert_that!(Range::parse(since, until)).is_ok();
}

#[rstest]
#[case::words("yesterday")]
#[case::slashes("2026/10/30")]
#[case::no_day("2026-10")]
#[case::bad_month("2026-13-01")]
fn parse_day_bad_text_returns_bad_date(#[case] text: &str) {
    assert_that!(parse_day(text).map_err(|e| e.kind())).is_equal_to(Err(UsageRangeErrorKind::BadDate));
}

#[test]
fn parse_day_iso_date_returns_it() {
    assert_that!(parse_day("2026-10-30").ok()).is_equal_to(Some(date!(2026 - 10 - 30)));
}

#[test]
fn range_errors_localize_in_french() {
    let fr = Localizer::new(Some("fr"));
    let bad = parse_day("hier").unwrap_err().localize(&fr);
    assert_that!(bad.as_str()).contains("hier");
    let reversed = Range::parse(Some("2026-10-31"), Some("2026-10-30")).unwrap_err().localize(&fr);
    assert_that!(reversed.as_str()).contains("2026-10-31");
    assert_that!(reversed.as_str()).contains("après");
}

#[test]
fn usage_report_serializes_by_lowercase_and_skips_internals() {
    let json = serde_json::to_value(report(&[record(1, 1)], By::Day, Range::ALL)).unwrap();
    assert_that!(json["by"].as_str()).is_equal_to(Some("day"));
    assert_that!(json["totals"].get("with_cost")).is_none();
}
