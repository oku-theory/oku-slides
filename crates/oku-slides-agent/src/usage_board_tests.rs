use agent_client_protocol::schema::v1::{Cost, SessionId, Usage};
use oku_slides_core::{UsageCost, UsageTab, UsageTokens};
use oku_slides_session::DocumentKind;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

fn c(amount: f64, currency: &str) -> UsageCost {
    UsageCost { amount, currency: currency.into() }
}

fn told(amount: f64, currency: &str) -> Cost {
    Cost::new(amount, currency)
}

/// Plays turns of one session: each is told its costs in order, then the turn's share is taken.
fn shares(resumed: bool, turns: &[Vec<Cost>]) -> Vec<Option<UsageCost>> {
    let board = UsageBoard::default();
    let id = SessionId::new("s");
    if resumed {
        board.resumed(1, &id);
    }
    turns
        .iter()
        .map(|seen| {
            for cost in seen {
                board.told(1, &id, Some(cost));
            }
            board.take_turn_cost(1, &id)
        })
        .collect()
}

#[rstest]
#[case::first_turn_is_whole(false, vec![vec![told(0.5, "USD")]], vec![Some(c(0.5, "USD"))])]
#[case::growing_total_gives_the_growth(
    false,
    vec![vec![told(0.5, "USD")], vec![told(0.75, "USD")]],
    vec![Some(c(0.5, "USD")), Some(c(0.25, "USD"))]
)]
#[case::last_update_of_the_turn_counts(false, vec![vec![told(0.25, "USD"), told(0.5, "USD")]], vec![Some(c(0.5, "USD"))])]
#[case::smaller_total_is_a_new_session(
    false,
    vec![vec![told(0.5, "USD")], vec![told(0.125, "USD")]],
    vec![Some(c(0.5, "USD")), Some(c(0.125, "USD"))]
)]
#[case::currency_change_restarts(
    false,
    vec![vec![told(0.5, "USD")], vec![told(0.25, "EUR")]],
    vec![Some(c(0.5, "USD")), Some(c(0.25, "EUR"))]
)]
#[case::turn_without_update_has_no_cost(false, vec![vec![told(0.5, "USD")], vec![]], vec![Some(c(0.5, "USD")), None])]
#[case::resumed_first_cost_is_only_the_base(
    true,
    vec![vec![told(2.0, "USD")], vec![told(2.5, "USD")]],
    vec![None, Some(c(0.5, "USD"))]
)]
#[case::resumed_without_cost_waits_for_one(
    true,
    vec![vec![], vec![told(2.0, "USD")], vec![told(2.5, "USD")]],
    vec![None, None, Some(c(0.5, "USD"))]
)]
#[case::not_finite_amount_is_ignored(false, vec![vec![told(f64::NAN, "USD")]], vec![None])]
#[case::negative_amount_is_ignored(false, vec![vec![told(-1.0, "USD")]], vec![None])]
#[case::lower_case_currency_is_ignored(false, vec![vec![told(1.0, "usd")]], vec![None])]
#[case::implausible_amount_is_ignored(false, vec![vec![told(2e9, "USD")]], vec![None])]
fn take_turn_cost_sequence_gives_each_turn_its_share(
    #[case] resumed: bool,
    #[case] turns: Vec<Vec<Cost>>,
    #[case] want: Vec<Option<UsageCost>>,
) {
    assert_that!(shares(resumed, &turns)).is_equal_to(want);
}

#[test]
fn told_without_cost_changes_nothing() {
    let board = UsageBoard::default();
    let id = SessionId::new("s");
    board.told(1, &id, None);
    assert_that!(board.take_turn_cost(1, &id)).is_none();
}

#[test]
fn forget_session_makes_its_next_cost_whole() {
    let board = UsageBoard::default();
    let id = SessionId::new("s");
    board.told(1, &id, Some(&told(0.5, "USD")));
    board.take_turn_cost(1, &id);
    board.forget(1, &id);
    board.told(1, &id, Some(&told(0.75, "USD")));
    assert_that!(board.take_turn_cost(1, &id)).is_equal_to(Some(c(0.75, "USD")));
}

#[test]
fn forget_generation_keeps_other_generations() {
    let board = UsageBoard::default();
    let id = SessionId::new("s");
    board.told(1, &id, Some(&told(0.5, "USD")));
    board.told(2, &id, Some(&told(0.25, "USD")));
    board.forget_generation(1);
    assert_that!(board.take_turn_cost(1, &id)).is_none();
    assert_that!(board.take_turn_cost(2, &id)).is_equal_to(Some(c(0.25, "USD")));
}

#[rstest]
#[case::base_absent(None, c(0.5, "USD"), c(0.5, "USD"))]
#[case::growth(Some(c(0.5, "USD")), c(0.75, "USD"), c(0.25, "USD"))]
#[case::equal_total_is_free(Some(c(0.5, "USD")), c(0.5, "USD"), c(0.0, "USD"))]
#[case::reset(Some(c(0.5, "USD")), c(0.125, "USD"), c(0.125, "USD"))]
#[case::other_currency(Some(c(0.5, "USD")), c(0.25, "EUR"), c(0.25, "EUR"))]
fn turn_cost_base_and_total_gives_share(
    #[case] base: Option<UsageCost>,
    #[case] now: UsageCost,
    #[case] want: UsageCost,
) {
    assert_that!(turn_cost(base.as_ref(), &now)).is_equal_to(want);
}

#[test]
fn tokens_of_reply_without_optional_counts_keeps_them_unknown() {
    let want = UsageTokens { total: 30, input: 20, output: 10, thought: None, cache_read: None, cache_write: None };
    assert_that!(tokens_of(&Usage::new(30, 20, 10))).is_equal_to(want);
}

#[test]
fn tokens_of_reply_with_optional_counts_keeps_them() {
    let usage = Usage::new(30, 20, 10).thought_tokens(4).cached_read_tokens(100).cached_write_tokens(7);
    let t = tokens_of(&usage);
    assert_that!((t.thought, t.cache_read, t.cache_write)).is_equal_to((Some(4), Some(100), Some(7)));
}

#[rstest]
#[case::deck(DocumentKind::Deck, Some(UsageTab::Deck))]
#[case::pack(DocumentKind::Pack, Some(UsageTab::Pack))]
#[case::theme(DocumentKind::Theme, Some(UsageTab::Theme))]
#[case::help(DocumentKind::Help, None)]
fn usage_tab_document_kind_gives_record_tab(#[case] kind: DocumentKind, #[case] want: Option<UsageTab>) {
    assert_that!(usage_tab(kind)).is_equal_to(want);
}

#[rstest]
#[case::routed_live(Some(false), true)]
#[case::routed_replaying(Some(true), false)]
#[case::unknown_session(None, false)]
fn tells_route_state_decides_whether_a_state_update_counts(#[case] replaying: Option<bool>, #[case] want: bool) {
    assert_that!(tells(replaying, &SessionId::new("s"), "cost")).is_equal_to(want);
}
