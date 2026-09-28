use speculoos::prelude::*;

use crate::{Deck, DeckReplica, Slide};

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap()
}

fn titled(deck: &Deck, t: &str) -> Deck {
    let mut d = deck.clone();
    d.slides[2].title = Some(t.into());
    d
}

#[test]
fn undo_restores_previous_deck_and_moves_label() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    r.apply(&titled(&deck, "Un"), "Renommer").unwrap();
    assert_that!(r.undo_label()).is_equal_to(Some("Renommer".to_owned()));
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&deck);
    assert_that!(r.redo_label()).is_equal_to(Some("Renommer".to_owned()));
    assert_that!(r.redo().unwrap()).is_true();
    assert_that!(r.deck().0.slides[2].title.as_deref()).is_equal_to(Some("Un"));
}

#[test]
fn two_immediate_commits_are_two_undo_steps() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    r.apply(&titled(&deck, "Un"), "Un").unwrap();
    r.apply(&titled(&deck, "Deux"), "Deux").unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0.slides[2].title.as_deref()).is_equal_to(Some("Un"));
    assert_that!(r.undo_label()).is_equal_to(Some("Un".to_owned()));
}

#[test]
fn new_replica_has_nothing_to_undo() {
    let r = DeckReplica::new(&roadmap()).unwrap();
    assert_that!(r.can_undo()).is_false();
}

#[test]
fn apply_silent_is_not_undoable() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    r.apply_silent(&titled(&deck, "Canonique")).unwrap();
    assert_that!(r.can_undo()).is_false();
}

#[test]
fn empty_apply_does_not_leak_its_label() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    r.apply(&deck, "Vide").unwrap();
    r.apply(&titled(&deck, "Deux"), "Deux").unwrap();
    assert_that!(r.undo_label()).is_equal_to(Some("Deux".to_owned()));
}

#[test]
fn undo_after_merge_keeps_remote_change() {
    let deck = roadmap();
    let base = DeckReplica::new(&deck).unwrap();
    let snap = base.snapshot().unwrap();
    let mut a = DeckReplica::from_snapshot(&snap, 1 << 20).unwrap().0;
    let mut b = DeckReplica::from_snapshot(&snap, 1 << 20).unwrap().0;
    a.apply(&titled(&deck, "de A"), "A").unwrap();
    let mut db = deck.clone();
    db.slides[3].title = Some("de B".into());
    b.apply(&db, "B").unwrap();
    a.merge(&b).unwrap();
    a.undo().unwrap();
    let got = a.deck().0;
    assert_that!(got.slides[2].title).is_equal_to(&deck.slides[2].title);
    assert_that!(got.slides[3].title.as_deref()).is_equal_to(Some("de B"));
}

#[test]
fn undo_limit_drops_oldest_steps() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    r.set_undo_limit(2);
    for t in ["1", "2", "3"] {
        r.apply(&titled(&deck, t), t).unwrap();
    }
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.undo().unwrap()).is_false();
}

fn themed(deck: &Deck, t: &str) -> Deck {
    let mut d = deck.clone();
    d.theme = Some(t.into());
    d
}

#[test]
fn group_merges_commits_into_one_undo_step_with_the_group_label() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    r.begin_group("Agent : titre plus percutant");
    r.apply(&titled(&deck, "Un"), "ignoré").unwrap();
    r.apply(&themed(&titled(&deck, "Un"), "pro"), "ignoré aussi").unwrap();
    r.end_group();
    assert_that!(r.undo_label()).is_equal_to(Some("Agent : titre plus percutant".to_owned()));
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&deck);
    assert_that!(r.can_undo()).is_false();
}

#[test]
fn group_redo_replays_every_commit() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    let after = themed(&titled(&deck, "Un"), "pro");
    r.begin_group("Tour");
    r.apply(&titled(&deck, "Un"), "a").unwrap();
    r.apply(&after, "b").unwrap();
    r.end_group();
    r.undo().unwrap();
    assert_that!(r.redo_label()).is_equal_to(Some("Tour".to_owned()));
    assert_that!(r.redo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&after);
}

#[test]
fn empty_group_creates_no_step() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    r.apply(&titled(&deck, "Avant"), "Avant").unwrap();
    r.begin_group("Tour vide");
    r.end_group();
    assert_that!(r.undo_label()).is_equal_to(Some("Avant".to_owned()));
}

#[test]
fn undo_inside_a_group_closes_it() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    r.begin_group("Tour");
    r.apply(&titled(&deck, "Un"), "a").unwrap();
    r.undo().unwrap();
    assert_that!(r.in_group()).is_false();
    r.apply(&titled(&deck, "Seul"), "Seul").unwrap();
    assert_that!(r.undo_label()).is_equal_to(Some("Seul".to_owned()));
}

#[test]
fn a_new_group_closes_the_open_one() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    r.begin_group("Premier");
    r.apply(&titled(&deck, "Un"), "a").unwrap();
    r.begin_group("Second");
    r.apply(&titled(&deck, "Deux"), "b").unwrap();
    r.end_group();
    assert_that!(r.undo_label()).is_equal_to(Some("Second".to_owned()));
    r.undo().unwrap();
    assert_that!(r.undo_label()).is_equal_to(Some("Premier".to_owned()));
}

#[test]
fn step_partly_dropped_by_loro_is_no_longer_undoable() {
    // Limite 1 : Loro garde 64 items ; renverser 80 slides en fait 79.
    let mut deck = roadmap();
    let model = deck.slides[2].clone();
    deck.slides = (0..80).map(|k| Slide { id: format!("s{k}"), ..model.clone() }).collect();
    let mut r = DeckReplica::new(&deck).unwrap();
    r.set_undo_limit(1);
    let mut reversed = deck.clone();
    reversed.slides.reverse();
    r.apply(&reversed, "Inverser").unwrap();
    assert_that!(r.can_undo()).is_false();
    assert_that!(r.undo().unwrap()).is_false();
    assert_that!(r.deck().0).is_equal_to(&reversed);
}
