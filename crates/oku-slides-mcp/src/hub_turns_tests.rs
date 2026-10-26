use std::sync::{Arc, Barrier};

use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::super::tests::hub;
use super::{PackTabClaim, Reserve};

const A: Uuid = Uuid::from_u128(0xa);
const B: Uuid = Uuid::from_u128(0xb);

#[test]
fn reserve_pack_tab_grants_one_of_many_concurrent_claims() {
    let hub = hub();
    let turn = hub.begin_turn();
    let start = Arc::new(Barrier::new(8));
    let answers: Vec<Reserve> = (0..8_u128)
        .map(|i| {
            let (hub, start) = (Arc::clone(&hub), Arc::clone(&start));
            std::thread::spawn(move || {
                start.wait();
                hub.reserve_pack_tab(turn, Uuid::from_u128(100 + i))
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
fn reserve_pack_tab_refuses_a_turn_that_is_not_open() {
    let hub = hub();
    let open = hub.begin_turn();
    assert_that!(hub.reserve_pack_tab(open + 1, A)).is_equal_to(Reserve::NotThisTurn);
    assert_that!(hub.state().pack_tab).is_none();
}

#[test]
fn reserve_pack_tab_refuses_a_turn_that_ended() {
    let hub = hub();
    let turn = hub.begin_turn();
    hub.end_turn(turn, "");
    assert_that!(hub.reserve_pack_tab(turn, A)).is_equal_to(Reserve::NotThisTurn);
}

#[test]
fn reserve_pack_tab_refuses_a_closed_hub() {
    let hub = hub();
    let turn = hub.begin_turn();
    hub.close();
    assert_that!(hub.reserve_pack_tab(turn, A)).is_equal_to(Reserve::NotThisTurn);
}

#[test]
fn reserve_pack_tab_grants_the_open_turn_once_per_pack() {
    let hub = hub();
    let turn = hub.begin_turn();
    assert_that!(hub.reserve_pack_tab(turn, A)).is_equal_to(Reserve::Granted);
    assert_that!(hub.reserve_pack_tab(turn, A)).is_equal_to(Reserve::Granted);
    assert_that!(hub.reserve_pack_tab(turn, B)).is_equal_to(Reserve::Taken);
    assert_that!(hub.state().pack_tab).is_equal_to(Some(PackTabClaim { turn, pack: A, claims: 2 }));
}

#[test]
fn reserve_pack_tab_replaces_the_claim_of_an_earlier_turn() {
    let hub = hub();
    let first = hub.begin_turn();
    assert_that!(hub.reserve_pack_tab(first, A)).is_equal_to(Reserve::Granted);
    let second = hub.begin_turn();
    assert_that!(hub.reserve_pack_tab(second, B)).is_equal_to(Reserve::Granted);
    assert_that!(hub.state().pack_tab).is_equal_to(Some(PackTabClaim { turn: second, pack: B, claims: 1 }));
}

#[test]
fn reserve_pack_tab_never_overwrites_a_later_turn() {
    let hub = hub();
    hub.begin_turn();
    hub.begin_turn();
    let open = hub.begin_turn();
    let later = PackTabClaim { turn: open + 2, pack: A, claims: 1 };
    hub.state().pack_tab = Some(later);
    assert_that!(hub.reserve_pack_tab(open, B)).is_equal_to(Reserve::Taken);
    assert_that!(hub.reserve_pack_tab(open, A)).is_equal_to(Reserve::Taken);
    assert_that!(hub.state().pack_tab).is_equal_to(Some(later));
}

#[test]
fn reserve_pack_tab_counts_claims_without_overflow() {
    let hub = hub();
    let turn = hub.begin_turn();
    let full = PackTabClaim { turn, pack: A, claims: u32::MAX };
    hub.state().pack_tab = Some(full);
    assert_that!(hub.reserve_pack_tab(turn, A)).is_equal_to(Reserve::Taken);
    assert_that!(hub.state().pack_tab).is_equal_to(Some(full));
}

#[test]
fn release_pack_tab_keeps_a_claim_still_held() {
    let hub = hub();
    let turn = hub.begin_turn();
    let _ = hub.reserve_pack_tab(turn, A);
    let _ = hub.reserve_pack_tab(turn, A);
    hub.release_pack_tab(turn, A);
    assert_that!(hub.state().pack_tab).is_equal_to(Some(PackTabClaim { turn, pack: A, claims: 1 }));
    assert_that!(hub.reserve_pack_tab(turn, B)).is_equal_to(Reserve::Taken);
}

#[test]
fn release_pack_tab_frees_the_turn_once_every_claim_is_given_back() {
    let hub = hub();
    let turn = hub.begin_turn();
    let _ = hub.reserve_pack_tab(turn, A);
    hub.release_pack_tab(turn, A);
    assert_that!(hub.state().pack_tab).is_none();
    assert_that!(hub.reserve_pack_tab(turn, B)).is_equal_to(Reserve::Granted);
}

#[rstest]
#[case::another_pack(1, B)]
#[case::another_turn(2, A)]
fn release_pack_tab_ignores_a_claim_that_is_not_the_callers(#[case] turn: u32, #[case] pack: Uuid) {
    let hub = hub();
    let _ = hub.begin_turn();
    let _ = hub.reserve_pack_tab(1, A);
    hub.release_pack_tab(turn, pack);
    assert_that!(hub.state().pack_tab).is_equal_to(Some(PackTabClaim { turn: 1, pack: A, claims: 1 }));
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
    let _ = hub.reserve_pack_tab(turn, A);
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
    let _ = hub.reserve_pack_tab(first, A);
    hub.note_elsewhere(first);
    let second = hub.begin_turn();
    assert_that!(hub.writes_elsewhere(first)).is_false();
    assert_that!(hub.writes_elsewhere(second)).is_false();
}

#[test]
fn writes_elsewhere_ends_with_the_claim_given_back() {
    let hub = hub();
    let turn = hub.begin_turn();
    let _ = hub.reserve_pack_tab(turn, A);
    hub.release_pack_tab(turn, A);
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
