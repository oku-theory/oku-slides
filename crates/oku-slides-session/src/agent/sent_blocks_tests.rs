use speculoos::prelude::*;

use super::SentBlock;

const D1: [u8; 32] = [1; 32];
const D2: [u8; 32] = [2; 32];

#[test]
fn sent_block_empty_holds_nothing() {
    let sent = SentBlock::default();
    assert_that!(sent.holds("a", &D1)).is_false();
    assert_that!(sent.digest()).is_none();
}

#[test]
fn sent_block_holds_only_its_session_and_digest() {
    let mut sent = SentBlock::default();
    sent.record("a", D1);
    assert_that!(sent.holds("a", &D1)).is_true();
    assert_that!(sent.holds("b", &D1)).is_false();
    assert_that!(sent.holds("a", &D2)).is_false();
}

#[test]
fn sent_block_redigest_keeps_the_session() {
    let mut sent = SentBlock::default();
    sent.record("a", D1);
    sent.redigest(D2);
    assert_that!(sent.holds("a", &D2)).is_true();
}

#[test]
fn sent_block_redigest_of_nothing_sent_stays_empty() {
    let mut sent = SentBlock::default();
    sent.redigest(D2);
    assert_that!(sent).is_equal_to(SentBlock::default());
}
