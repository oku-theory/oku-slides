use std::sync::{Arc, Barrier};

use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::super::tests::hub;
use super::{Reserve, TabClaim, TabTarget};

const A: Uuid = Uuid::from_u128(0xa);
const B: Uuid = Uuid::from_u128(0xb);

#[test]
fn reserve_tab_grants_one_of_many_concurrent_claims() {
    let hub = hub();
    let turn = hub.begin_turn();
    let start = Arc::new(Barrier::new(8));
    let answers: Vec<Reserve> = (0..8_u128)
        .map(|i| {
            let (hub, start) = (Arc::clone(&hub), Arc::clone(&start));
            std::thread::spawn(move || {
                start.wait();
                hub.reserve_tab(turn, TabTarget::Pack(Uuid::from_u128(100 + i)))
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|t| t.join().unwrap_or_else(|_| panic!("claim thread")))
        .collect();
    assert_that!(answers.iter().filter(|a| **a == Reserve::Granted).count()).is_equal_to(1);
    assert_that!(answers.iter().filter(|a| **a == Reserve::Taken).count()).is_equal_to(7);
}

#[test]
fn reserve_tab_refuses_a_turn_that_is_not_open() {
    let hub = hub();
    let open = hub.begin_turn();
    assert_that!(hub.reserve_tab(open + 1, TabTarget::Pack(A))).is_equal_to(Reserve::NotThisTurn);
    assert_that!(hub.state().tab_claim).is_none();
}

#[test]
fn reserve_tab_refuses_a_turn_that_ended() {
    let hub = hub();
    let turn = hub.begin_turn();
    hub.end_turn(turn, "");
    assert_that!(hub.reserve_tab(turn, TabTarget::Pack(A))).is_equal_to(Reserve::NotThisTurn);
}

#[test]
fn reserve_tab_refuses_a_closed_hub() {
    let hub = hub();
    let turn = hub.begin_turn();
    hub.close();
    assert_that!(hub.reserve_tab(turn, TabTarget::Pack(A))).is_equal_to(Reserve::NotThisTurn);
}

#[test]
fn reserve_tab_grants_the_open_turn_once_per_pack() {
    let hub = hub();
    let turn = hub.begin_turn();
    assert_that!(hub.reserve_tab(turn, TabTarget::Pack(A))).is_equal_to(Reserve::Granted);
    assert_that!(hub.reserve_tab(turn, TabTarget::Pack(A))).is_equal_to(Reserve::Granted);
    assert_that!(hub.reserve_tab(turn, TabTarget::Pack(B))).is_equal_to(Reserve::Taken);
    assert_that!(hub.state().tab_claim).is_equal_to(Some(TabClaim { turn, target: TabTarget::Pack(A), claims: 2 }));
}

#[test]
fn reserve_tab_replaces_the_claim_of_an_earlier_turn() {
    let hub = hub();
    let first = hub.begin_turn();
    assert_that!(hub.reserve_tab(first, TabTarget::Pack(A))).is_equal_to(Reserve::Granted);
    let second = hub.begin_turn();
    assert_that!(hub.reserve_tab(second, TabTarget::Pack(B))).is_equal_to(Reserve::Granted);
    assert_that!(hub.state().tab_claim).is_equal_to(Some(TabClaim {
        turn: second,
        target: TabTarget::Pack(B),
        claims: 1,
    }));
}

#[test]
fn reserve_tab_never_overwrites_a_later_turn() {
    let hub = hub();
    hub.begin_turn();
    hub.begin_turn();
    let open = hub.begin_turn();
    let later = TabClaim { turn: open + 2, target: TabTarget::Pack(A), claims: 1 };
    hub.state().tab_claim = Some(later);
    assert_that!(hub.reserve_tab(open, TabTarget::Pack(B))).is_equal_to(Reserve::Taken);
    assert_that!(hub.reserve_tab(open, TabTarget::Pack(A))).is_equal_to(Reserve::Taken);
    assert_that!(hub.state().tab_claim).is_equal_to(Some(later));
}

#[test]
fn reserve_tab_counts_claims_without_overflow() {
    let hub = hub();
    let turn = hub.begin_turn();
    let full = TabClaim { turn, target: TabTarget::Pack(A), claims: u32::MAX };
    hub.state().tab_claim = Some(full);
    assert_that!(hub.reserve_tab(turn, TabTarget::Pack(A))).is_equal_to(Reserve::Taken);
    assert_that!(hub.state().tab_claim).is_equal_to(Some(full));
}

#[test]
fn release_tab_keeps_a_claim_still_held() {
    let hub = hub();
    let turn = hub.begin_turn();
    let _ = hub.reserve_tab(turn, TabTarget::Pack(A));
    let _ = hub.reserve_tab(turn, TabTarget::Pack(A));
    hub.release_tab(turn, TabTarget::Pack(A));
    assert_that!(hub.state().tab_claim).is_equal_to(Some(TabClaim { turn, target: TabTarget::Pack(A), claims: 1 }));
    assert_that!(hub.reserve_tab(turn, TabTarget::Pack(B))).is_equal_to(Reserve::Taken);
}

#[test]
fn release_tab_frees_the_turn_once_every_claim_is_given_back() {
    let hub = hub();
    let turn = hub.begin_turn();
    let _ = hub.reserve_tab(turn, TabTarget::Pack(A));
    hub.release_tab(turn, TabTarget::Pack(A));
    assert_that!(hub.state().tab_claim).is_none();
    assert_that!(hub.reserve_tab(turn, TabTarget::Pack(B))).is_equal_to(Reserve::Granted);
}

#[rstest]
#[case::another_pack(1, TabTarget::Pack(B))]
#[case::another_turn(2, TabTarget::Pack(A))]
fn release_tab_ignores_a_claim_that_is_not_the_callers(#[case] turn: u32, #[case] target: TabTarget) {
    let hub = hub();
    let _ = hub.begin_turn();
    let _ = hub.reserve_tab(1, TabTarget::Pack(A));
    hub.release_tab(turn, target);
    assert_that!(hub.state().tab_claim).is_equal_to(Some(TabClaim { turn: 1, target: TabTarget::Pack(A), claims: 1 }));
}

const PAPIER: Uuid = Uuid::from_u128(0xfa);

#[test]
fn reserve_tab_of_a_theme_after_a_pack_in_one_turn_is_taken() {
    let hub = hub();
    let turn = hub.begin_turn();
    assert_that!(hub.reserve_tab(turn, TabTarget::Pack(A))).is_equal_to(Reserve::Granted);
    assert_that!(hub.reserve_tab(turn, TabTarget::Theme(PAPIER))).is_equal_to(Reserve::Taken);
}

#[test]
fn reserve_tab_of_the_same_target_twice_needs_two_releases() {
    let hub = hub();
    let turn = hub.begin_turn();
    let _ = hub.reserve_tab(turn, TabTarget::Theme(PAPIER));
    let _ = hub.reserve_tab(turn, TabTarget::Theme(PAPIER));
    hub.release_tab(turn, TabTarget::Theme(PAPIER));
    assert_that!(hub.reserve_tab(turn, TabTarget::Pack(A))).is_equal_to(Reserve::Taken);
}

#[test]
fn release_tab_of_a_pack_never_releases_a_theme_claim_of_the_same_uuid() {
    let hub = hub();
    let turn = hub.begin_turn();
    let id = Uuid::from_u128(7);
    let _ = hub.reserve_tab(turn, TabTarget::Theme(id));
    hub.release_tab(turn, TabTarget::Pack(id));
    assert_that!(hub.reserve_tab(turn, TabTarget::Pack(A))).is_equal_to(Reserve::Taken);
}

#[test]
fn turn_is_open_only_for_the_open_turn() {
    let hub = hub();
    assert_that!(hub.turn_is_open(1)).is_false();
    let first = hub.begin_turn();
    assert_that!(hub.turn_is_open(first)).is_true();
    assert_that!(hub.turn_is_open(first + 1)).is_false();
    let second = hub.begin_turn();
    assert_that!(hub.turn_is_open(first)).is_false();
    assert_that!(hub.turn_is_open(second)).is_true();
    hub.end_turn(second, "");
    assert_that!(hub.turn_is_open(second)).is_false();
}

#[test]
fn turn_is_open_is_false_on_a_closed_hub() {
    let hub = hub();
    let turn = hub.begin_turn();
    hub.close();
    assert_that!(hub.turn_is_open(turn)).is_false();
}

#[test]
fn writes_elsewhere_follows_a_claim_of_the_open_turn() {
    let hub = hub();
    let turn = hub.begin_turn();
    assert_that!(hub.writes_elsewhere(turn)).is_false();
    let _ = hub.reserve_tab(turn, TabTarget::Pack(A));
    assert_that!(hub.writes_elsewhere(turn)).is_true();
}

#[test]
fn writes_elsewhere_follows_a_focus_of_the_open_turn() {
    let hub = hub();
    let turn = hub.begin_turn();
    hub.note_elsewhere(turn);
    assert_that!(hub.writes_elsewhere(turn)).is_true();
}

#[test]
fn writes_elsewhere_is_false_for_the_next_turn() {
    let hub = hub();
    let first = hub.begin_turn();
    let _ = hub.reserve_tab(first, TabTarget::Pack(A));
    hub.note_elsewhere(first);
    let second = hub.begin_turn();
    assert_that!(hub.writes_elsewhere(first)).is_false();
    assert_that!(hub.writes_elsewhere(second)).is_false();
}

#[test]
fn writes_elsewhere_ends_with_the_claim_given_back() {
    let hub = hub();
    let turn = hub.begin_turn();
    let _ = hub.reserve_tab(turn, TabTarget::Pack(A));
    hub.release_tab(turn, TabTarget::Pack(A));
    assert_that!(hub.writes_elsewhere(turn)).is_false();
}

#[test]
fn note_elsewhere_keeps_the_latest_turn() {
    let hub = hub();
    hub.begin_turn();
    let second = hub.begin_turn();
    hub.note_elsewhere(second);
    hub.note_elsewhere(second - 1);
    assert_that!(hub.state().elsewhere).is_equal_to(Some(second));
}
