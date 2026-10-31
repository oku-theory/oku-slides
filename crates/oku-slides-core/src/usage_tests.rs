use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;
use time::macros::datetime;
use uuid::Uuid;

use super::*;

/// A valid record for turn `n`, `input` input tokens, everything else fixed.
pub(crate) fn record(n: u128, input: u64) -> UsageRecord {
    UsageRecord {
        model: Some("claude-opus-5-5".into()),
        tokens: Some(UsageTokens {
            total: input + 10,
            input,
            output: 10,
            thought: None,
            cache_read: Some(5),
            cache_write: None,
        }),
        cost: Some(UsageCost { amount: 0.04, currency: "USD".into() }),
        ..UsageRecord::new(Uuid::from_u128(n), datetime!(2026-10-30 14:02:11 UTC), UsageTab::Deck, "claude")
    }
}

/// [`record`] with an unknown field `pad` of `len` bytes, as a newer app might write.
fn padded(n: u128, len: usize) -> UsageRecord {
    let mut r = record(n, 1);
    r.extra.insert("pad".into(), serde_json::json!("p".repeat(len)));
    r
}

fn line(r: &UsageRecord) -> Vec<u8> {
    serde_json::to_vec(r).unwrap()
}

fn lines(rs: &[UsageRecord]) -> Vec<u8> {
    rs.iter().flat_map(|r| [line(r), b"\n".to_vec()]).flatten().collect()
}

#[test]
fn to_jsonl_then_from_jsonl_returns_same_ledger() {
    let l = UsageLedger::from_iter([record(1, 100), record(2, 200)]);
    let (back, issues) = UsageLedger::from_jsonl(&l.to_jsonl());
    assert_that!(issues).is_empty();
    assert_that!(back).is_equal_to(&l);
}

#[test]
fn to_jsonl_same_ledger_returns_same_bytes() {
    let a = UsageLedger::from_iter([record(1, 1), record(2, 2)]);
    let b = UsageLedger::from_iter([record(2, 2), record(1, 1)]);
    assert_that!(a.to_jsonl()).is_equal_to(b.to_jsonl());
}

#[test]
fn records_are_sorted_by_end_of_turn_then_turn() {
    let mut early = record(9, 1);
    early.at = datetime!(2026-10-29 08:00 UTC);
    let l = UsageLedger::from_iter([record(2, 1), early, record(1, 1)]);
    let turns: Vec<u128> = l.records().map(|r| r.turn.as_u128()).collect();
    assert_that!(turns).is_equal_to(vec![9, 1, 2]);
}

/// One `record(1, 1)` line with a field this app does not know.
fn newer_line() -> Vec<u8> {
    let mut value = serde_json::to_value(record(1, 1)).unwrap();
    value["from_a_newer_app"] = serde_json::json!({"k": [1, 2]});
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn from_jsonl_unknown_field_keeps_the_record() {
    let (back, issues) = UsageLedger::from_jsonl(&newer_line());
    assert_that!(issues).is_empty();
    assert_that!(back.len()).is_equal_to(1);
}

#[test]
fn to_jsonl_writes_unknown_fields_back() {
    let (back, _) = UsageLedger::from_jsonl(&newer_line());
    let text = String::from_utf8(back.to_jsonl()).unwrap();
    assert_that!(text.as_str()).contains(r#""from_a_newer_app":{"k":[1,2]}"#);
}

#[test]
fn union_with_a_resaved_copy_reports_nothing() {
    let (mut original, _) = UsageLedger::from_jsonl(&newer_line());
    let (resaved, _) = UsageLedger::from_jsonl(&original.to_jsonl());
    let issues = original.union(resaved);
    assert_that!(issues).is_empty();
    assert_that!(original.len()).is_equal_to(1);
}

/// Turns costed as differences of cumulative totals, the way the agent reports them.
fn computed_costs() -> UsageLedger {
    let mut amounts = vec![0.1 + 0.2, 0.920_399_999_999_999_9];
    amounts.extend((1..=500).map(|i| f64::from(i) * 0.0173 - f64::from(i - 1) * 0.0173 + f64::from(i) * 1e-7));
    (0u128..)
        .zip(amounts)
        .map(|(n, amount)| UsageRecord { cost: Some(UsageCost { amount, currency: "USD".into() }), ..record(n, 1) })
        .collect()
}

#[test]
fn from_jsonl_computed_costs_read_back_exactly() {
    let l = computed_costs();
    let (back, issues) = UsageLedger::from_jsonl(&l.to_jsonl());
    assert_that!(issues).is_empty();
    assert_that!(back).is_equal_to(&l);
}

#[test]
fn union_with_computed_costs_read_back_reports_no_forged_turn() {
    let mut held = computed_costs();
    let (back, _) = UsageLedger::from_jsonl(&held.to_jsonl());
    assert_that!(held.union(back)).is_empty();
}

#[test]
fn from_jsonl_invalid_line_drops_it_and_keeps_others() {
    let mut bytes = b"{not json}\n".to_vec();
    bytes.extend(lines(&[record(1, 1)]));
    let (back, issues) = UsageLedger::from_jsonl(&bytes);
    assert_that!(back.len()).is_equal_to(1);
    assert_that!(issues).is_equal_to(vec![Issue::usage_line_invalid(1)]);
}

#[test]
fn from_jsonl_blank_lines_are_skipped_silently() {
    let mut bytes = b"\n  \n".to_vec();
    bytes.extend(lines(&[record(1, 1)]));
    let (back, issues) = UsageLedger::from_jsonl(&bytes);
    assert_that!(back.len()).is_equal_to(1);
    assert_that!(issues).is_empty();
}

#[test]
fn from_jsonl_line_over_4_kib_is_invalid() {
    let mut r = record(1, 1);
    r.model = Some("m".repeat(150));
    let mut value = serde_json::to_value(&r).unwrap();
    value["padding"] = serde_json::json!("p".repeat(5000));
    let (back, issues) = UsageLedger::from_jsonl(&serde_json::to_vec(&value).unwrap());
    assert_that!(back.is_empty()).is_true();
    assert_that!(issues).is_equal_to(vec![Issue::usage_line_invalid(1)]);
}

#[rstest]
#[case::empty_agent(|r: &mut UsageRecord| r.agent = String::new())]
#[case::long_agent(|r: &mut UsageRecord| r.agent = "a".repeat(65))]
#[case::control_in_agent(|r: &mut UsageRecord| r.agent = "cl\u{1b}[31maude".into())]
#[case::bidi_in_model(|r: &mut UsageRecord| r.model = Some("opus\u{202E}5".into()))]
#[case::zero_width_in_agent(|r: &mut UsageRecord| r.agent = "cla\u{200B}ude".into())]
#[case::line_separator_in_model(|r: &mut UsageRecord| r.model = Some("opus\u{2028}5".into()))]
#[case::long_model(|r: &mut UsageRecord| r.model = Some("m".repeat(201)))]
#[case::lower_currency(|r: &mut UsageRecord| r.cost = Some(UsageCost { amount: 1.0, currency: "usd".into() }))]
#[case::long_currency(|r: &mut UsageRecord| r.cost = Some(UsageCost { amount: 1.0, currency: "USDT".into() }))]
#[case::negative_amount(|r: &mut UsageRecord| r.cost = Some(UsageCost { amount: -1.0, currency: "USD".into() }))]
#[case::huge_amount(|r: &mut UsageRecord| r.cost = Some(UsageCost { amount: 1e10, currency: "USD".into() }))]
#[case::not_utc(|r: &mut UsageRecord| r.at = datetime!(2026-10-30 14:02:11 +02:00))]
fn from_jsonl_record_failing_checks_is_dropped(#[case] spoil: fn(&mut UsageRecord)) {
    let mut r = record(1, 1);
    spoil(&mut r);
    let (back, issues) = UsageLedger::from_jsonl(&line(&r));
    assert_that!(back.is_empty()).is_true();
    assert_that!(issues).is_equal_to(vec![Issue::usage_line_invalid(1)]);
}

#[test]
fn from_jsonl_nan_amount_is_dropped() {
    // JSON has no NaN: an out-of-range number is the closest a hostile line gets; it is dropped.
    let text = String::from_utf8(line(&record(1, 1))).unwrap().replace("0.04", "1e400");
    let (back, issues) = UsageLedger::from_jsonl(text.as_bytes());
    assert_that!(back.is_empty()).is_true();
    assert_that!(issues).has_length(1);
}

#[test]
fn insert_invalid_record_is_not_stored() {
    let mut r = record(1, 1);
    r.agent = String::new();
    let mut l = UsageLedger::default();
    assert_that!(l.insert(r)).is_equal_to(Inserted::Invalid);
    assert_that!(l.is_empty()).is_true();
}

#[test]
fn insert_same_record_twice_returns_same() {
    let mut l = UsageLedger::default();
    assert_that!(l.insert(record(1, 1))).is_equal_to(Inserted::Added);
    assert_that!(l.insert(record(1, 1))).is_equal_to(Inserted::Same);
}

#[test]
fn insert_past_the_record_cap_returns_full() {
    let mut l: UsageLedger = (0..USAGE_MAX_RECORDS as u128).map(|n| record(n, 1)).collect();
    assert_that!(l.insert(record(USAGE_MAX_RECORDS as u128, 1))).is_equal_to(Inserted::Full);
    assert_that!(l.len()).is_equal_to(USAGE_MAX_RECORDS);
}

#[test]
fn insert_past_the_byte_cap_returns_full_and_the_ledger_still_fits() {
    let mut l = UsageLedger::default();
    let mut n = 0;
    while l.insert(padded(n, 3600)) == Inserted::Added {
        n += 1;
    }
    assert_that!(n).is_greater_than(3000);
    assert_that!(l.insert(padded(n + 1, 3600))).is_equal_to(Inserted::Full);
    let written = u64::try_from(l.to_jsonl().len()).unwrap();
    assert_that!(written).is_less_than_or_equal_to(USAGE_MAX_BYTES);
}

#[test]
fn insert_record_whose_line_is_over_4_kib_is_invalid() {
    let mut l = UsageLedger::default();
    assert_that!(l.insert(padded(1, 5000))).is_equal_to(Inserted::Invalid);
    assert_that!(l.is_empty()).is_true();
}

#[rstest]
#[case::letter('a', true)]
#[case::accent('é', true)]
#[case::cjk('中', true)]
#[case::space(' ', true)]
#[case::escape('\u{1b}', false)]
#[case::arabic_letter_mark('\u{61C}', false)]
#[case::zero_width_space('\u{200B}', false)]
#[case::zero_width_joiner('\u{200D}', false)]
#[case::rtl_override('\u{202E}', false)]
#[case::word_joiner('\u{2060}', false)]
#[case::isolate('\u{2066}', false)]
#[case::line_separator('\u{2028}', false)]
#[case::paragraph_separator('\u{2029}', false)]
#[case::byte_order_mark('\u{FEFF}', false)]
#[case::tag_letter('\u{E0041}', false)]
fn shown_char_refuses_controls_format_and_separators(#[case] c: char, #[case] shown: bool) {
    assert_that!(shown_char(c)).is_equal_to(shown);
}

#[rstest]
#[case::small_first(1, 3)]
#[case::large_first(3, 1)]
fn insert_forged_turn_keeps_the_smaller_line(#[case] first: u64, #[case] second: u64) {
    let mut l = UsageLedger::from_iter([record(1, first)]);
    assert_that!(l.insert(record(1, second))).is_equal_to(Inserted::Forged);
    let kept = l.records().next().and_then(|r| r.tokens).map(|t| t.input);
    assert_that!(kept).is_equal_to(Some(1));
}

#[test]
fn from_jsonl_forged_turn_is_reported_once() {
    let bytes = lines(&[record(1, 1), record(1, 2), record(1, 3)]);
    let (back, issues) = UsageLedger::from_jsonl(&bytes);
    assert_that!(back.len()).is_equal_to(1);
    assert_that!(issues).is_equal_to(vec![Issue::usage_line_forged(&Uuid::from_u128(1).to_string())]);
}

#[test]
fn from_jsonl_many_bad_lines_reports_twenty_then_a_summary() {
    let bytes = "garbage\n".repeat(25);
    let (_, issues) = UsageLedger::from_jsonl(bytes.as_bytes());
    assert_that!(issues).has_length(21);
    assert_that!(issues[19]).is_equal_to(Issue::usage_line_invalid(20));
    assert_that!(issues[20]).is_equal_to(Issue::usage_lines_dropped(5));
}

#[test]
fn read_lines_over_record_cap_drops_and_counts_the_rest() {
    let bytes = lines(&[record(1, 1), record(2, 1), record(3, 1)]);
    let (back, issues) = UsageLedger::read_lines(&bytes, 2);
    assert_that!(back.len()).is_equal_to(2);
    assert_that!(issues).is_equal_to(vec![Issue::usage_lines_dropped(1)]);
}

#[test]
fn union_same_turn_twice_is_idempotent() {
    let mut a = UsageLedger::from_iter([record(1, 1)]);
    let issues = a.union(a.clone());
    assert_that!(issues).is_empty();
    assert_that!(a.len()).is_equal_to(1);
}

#[rstest]
#[case::distinct_turns(vec![record(1, 1)], vec![record(2, 2)])]
#[case::forged_turn(vec![record(1, 1), record(2, 2)], vec![record(1, 7)])]
#[case::forged_both_ways(vec![record(1, 7)], vec![record(1, 1)])]
fn union_is_commutative(#[case] a: Vec<UsageRecord>, #[case] b: Vec<UsageRecord>) {
    let (a, b) = (UsageLedger::from_iter(a), UsageLedger::from_iter(b));
    let (mut ab, mut ba) = (a.clone(), b.clone());
    let _ = ab.union(b);
    let _ = ba.union(a);
    assert_that!(ab).is_equal_to(ba);
}

#[test]
fn union_forged_turn_is_reported() {
    let mut a = UsageLedger::from_iter([record(1, 1)]);
    let issues = a.union(UsageLedger::from_iter([record(1, 2)]));
    assert_that!(issues).is_equal_to(vec![Issue::usage_line_forged(&Uuid::from_u128(1).to_string())]);
}

#[test]
fn to_jsonl_null_tokens_cost_and_model_are_written_null() {
    let mut r = record(1, 1);
    (r.tokens, r.cost, r.model) = (None, None, None);
    let text = String::from_utf8(UsageLedger::from_iter([r]).to_jsonl()).unwrap();
    assert_that!(text.as_str()).contains(r#""model":null"#);
    assert_that!(text.as_str()).contains(r#""tokens":null"#);
    assert_that!(text.as_str()).contains(r#""cost":null"#);
}

#[test]
fn to_jsonl_unreported_token_kinds_stay_null() {
    let text = String::from_utf8(UsageLedger::from_iter([record(1, 1)]).to_jsonl()).unwrap();
    assert_that!(text.as_str()).contains(r#""thought":null"#);
}

#[test]
fn sanitized_strips_control_and_bidi_characters() {
    let mut r = record(1, 1);
    r.agent = "cl\u{1b}aude\u{202E}".into();
    r.model = Some("opus\u{2066}-5".into());
    let s = r.sanitized().unwrap();
    assert_that!(s.agent.as_str()).is_equal_to("claude");
    assert_that!(s.model.as_deref()).is_equal_to(Some("opus-5"));
    assert_that!(s.is_valid()).is_true();
}

#[test]
fn sanitized_truncates_agent_and_model_to_their_caps() {
    let mut r = record(1, 1);
    r.agent = "é".repeat(70);
    r.model = Some("m".repeat(300));
    let s = r.sanitized().unwrap();
    assert_that!(s.agent.chars().count()).is_equal_to(64);
    assert_that!(s.model.map(|m| m.chars().count())).is_equal_to(Some(200));
}

#[rstest]
#[case::nan(f64::NAN, "USD")]
#[case::negative(-0.5, "USD")]
#[case::lower_currency(0.5, "usd")]
#[case::huge(2e9, "USD")]
fn sanitized_invalid_cost_becomes_none(#[case] amount: f64, #[case] currency: &str) {
    let mut r = record(1, 1);
    r.cost = Some(UsageCost { amount, currency: currency.into() });
    assert_that!(r.sanitized().and_then(|s| s.cost)).is_none();
}

#[test]
fn sanitized_converts_the_end_of_turn_to_utc() {
    let mut r = record(1, 1);
    r.at = datetime!(2026-10-30 16:02:11 +02:00);
    assert_that!(r.sanitized().map(|s| s.at)).is_equal_to(Some(datetime!(2026-10-30 14:02:11 UTC)));
}

#[test]
fn sanitized_empty_model_becomes_none() {
    let mut r = record(1, 1);
    r.model = Some("\u{7}".into());
    assert_that!(r.sanitized().and_then(|s| s.model)).is_none();
}

#[test]
fn sanitized_agent_empty_after_cleaning_returns_none() {
    let mut r = record(1, 1);
    r.agent = "\u{1b}\u{202E}".into();
    assert_that!(r.sanitized()).is_none();
}

#[test]
fn usage_line_invalid_localized_in_french_names_the_line() {
    let fr = Localizer::new(Some("fr"));
    assert_that!(Issue::usage_line_invalid(3).message.id()).is_equal_to("issue-usage-line-invalid");
    assert_that!(Issue::usage_line_invalid(3).localize(&fr)).contains("ligne 3");
}

#[test]
fn usage_lines_dropped_localized_in_french_counts_them() {
    let fr = Localizer::new(Some("fr"));
    assert_that!(Issue::usage_lines_dropped(5).localize(&fr)).contains("5");
    assert_that!(Issue::usage_lines_dropped(5).message.id()).is_equal_to("issue-usage-lines-dropped");
}

#[test]
fn shown_text_strips_refused_characters_and_keeps_the_rest() {
    assert_that!(shown_text("cl\u{1b}aude\u{202E} é中")).is_equal_to("claude é中".to_string());
}

#[rstest]
#[case::deck(UsageTab::Deck)]
#[case::pack(UsageTab::Pack)]
#[case::theme(UsageTab::Theme)]
fn usage_tab_as_str_equals_its_serde_name(#[case] tab: UsageTab) {
    let json = serde_json::to_string(&tab).unwrap();
    assert_that!(json).is_equal_to(format!("\"{}\"", tab.as_str()));
}

#[rstest]
#[case::short_first(3, 10_000)]
#[case::long_first(10_000, 3)]
fn insert_forged_turn_keeps_the_shorter_line_and_never_grows(#[case] first: u64, #[case] second: u64) {
    let mut l = UsageLedger::from_iter([record(1, first)]);
    let before = l.to_jsonl().len();
    assert_that!(l.insert(record(1, second))).is_equal_to(Inserted::Forged);
    let kept = l.records().next().and_then(|r| r.tokens).map(|t| t.input);
    assert_that!(kept).is_equal_to(Some(3));
    assert_that!(l.to_jsonl().len()).is_less_than_or_equal_to(before.max(line(&record(1, 3)).len() + 1));
    assert_that!(l.bytes).is_equal_to(u64::try_from(l.to_jsonl().len()).unwrap());
}

#[test]
fn union_forged_lines_of_different_length_is_commutative() {
    let (a, b) = (UsageLedger::from_iter([record(1, 3)]), UsageLedger::from_iter([record(1, 10_000)]));
    let (mut ab, mut ba) = (a.clone(), b.clone());
    let _ = ab.union(b);
    let _ = ba.union(a);
    assert_that!(ab.clone()).is_equal_to(ba);
    assert_that!(ab.records().next().and_then(|r| r.tokens).map(|t| t.input)).is_equal_to(Some(3));
}
