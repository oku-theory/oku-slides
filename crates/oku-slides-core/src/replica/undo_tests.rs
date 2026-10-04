use speculoos::prelude::*;

use crate::{Deck, DeckReplica, Slide};

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap()
}

fn titled(deck: &Deck, t: &str) -> Deck {
    let mut d = deck.clone();
    d.slides[2].params.set("title", crate::ParamValue::Text(t.into()));
    d
}

#[test]
fn undo_restores_previous_deck_and_moves_label() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.apply(&titled(&deck, "Un"), "Renommer", None).unwrap();
    assert_that!(r.undo_label()).is_equal_to(Some("Renommer".to_owned()));
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&deck);
    assert_that!(r.redo_label()).is_equal_to(Some("Renommer".to_owned()));
    assert_that!(r.redo().unwrap()).is_true();
    assert_that!(r.deck().0.slides[2].params.text("title")).is_equal_to(Some("Un"));
}

#[test]
fn two_immediate_commits_are_two_undo_steps() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.apply(&titled(&deck, "Un"), "Un", None).unwrap();
    r.apply(&titled(&deck, "Deux"), "Deux", None).unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0.slides[2].params.text("title")).is_equal_to(Some("Un"));
    assert_that!(r.undo_label()).is_equal_to(Some("Un".to_owned()));
}

#[test]
fn new_replica_has_nothing_to_undo() {
    let r = DeckReplica::new(&roadmap(), crate::test_registry::shared()).unwrap();
    assert_that!(r.can_undo()).is_false();
}

#[test]
fn apply_silent_is_not_undoable() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.apply_silent(&titled(&deck, "Canonique")).unwrap();
    assert_that!(r.can_undo()).is_false();
}

#[test]
fn empty_apply_does_not_leak_its_label() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.apply(&deck, "Vide", None).unwrap();
    r.apply(&titled(&deck, "Deux"), "Deux", None).unwrap();
    assert_that!(r.undo_label()).is_equal_to(Some("Deux".to_owned()));
}

#[test]
fn undo_after_merge_keeps_remote_change() {
    let deck = roadmap();
    let base = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let snap = base.snapshot().unwrap();
    let mut a = DeckReplica::from_snapshot(&snap, 1 << 20, crate::test_registry::shared()).unwrap().0;
    let mut b = DeckReplica::from_snapshot(&snap, 1 << 20, crate::test_registry::shared()).unwrap().0;
    a.apply(&titled(&deck, "de A"), "A", None).unwrap();
    let mut db = deck.clone();
    db.slides[3].params.set("title", crate::ParamValue::Text("de B".into()));
    b.apply(&db, "B", None).unwrap();
    a.merge(&b).unwrap();
    a.undo().unwrap();
    let got = a.deck().0;
    assert_that!(got.slides[2].params.get("title")).is_equal_to(deck.slides[2].params.get("title"));
    assert_that!(got.slides[3].params.text("title")).is_equal_to(Some("de B"));
}

#[test]
fn undo_limit_drops_oldest_steps() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.set_undo_limit(2);
    for t in ["1", "2", "3"] {
        r.apply(&titled(&deck, t), t, None).unwrap();
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
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.begin_group("Agent : titre plus percutant");
    r.apply(&titled(&deck, "Un"), "ignoré", None).unwrap();
    r.apply(&themed(&titled(&deck, "Un"), "pro"), "ignoré aussi", None).unwrap();
    r.end_group();
    assert_that!(r.undo_label()).is_equal_to(Some("Agent : titre plus percutant".to_owned()));
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&deck);
    assert_that!(r.can_undo()).is_false();
}

#[test]
fn group_redo_replays_every_commit() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let after = themed(&titled(&deck, "Un"), "pro");
    r.begin_group("Tour");
    r.apply(&titled(&deck, "Un"), "a", None).unwrap();
    r.apply(&after, "b", None).unwrap();
    r.end_group();
    r.undo().unwrap();
    assert_that!(r.redo_label()).is_equal_to(Some("Tour".to_owned()));
    assert_that!(r.redo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&after);
}

#[test]
fn empty_group_creates_no_step() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.apply(&titled(&deck, "Avant"), "Avant", None).unwrap();
    r.begin_group("Tour vide");
    r.end_group();
    assert_that!(r.undo_label()).is_equal_to(Some("Avant".to_owned()));
}

#[test]
fn undo_inside_a_group_closes_it() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.begin_group("Tour");
    r.apply(&titled(&deck, "Un"), "a", None).unwrap();
    r.undo().unwrap();
    assert_that!(r.in_group()).is_false();
    r.apply(&titled(&deck, "Seul"), "Seul", None).unwrap();
    assert_that!(r.undo_label()).is_equal_to(Some("Seul".to_owned()));
}

#[test]
fn a_new_group_closes_the_open_one() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.begin_group("Premier");
    r.apply(&titled(&deck, "Un"), "a", None).unwrap();
    r.begin_group("Second");
    r.apply(&titled(&deck, "Deux"), "b", None).unwrap();
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
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.set_undo_limit(1);
    let mut reversed = deck.clone();
    reversed.slides.reverse();
    r.apply(&reversed, "Inverser", None).unwrap();
    assert_that!(r.can_undo()).is_false();
    assert_that!(r.undo().unwrap()).is_false();
    assert_that!(r.deck().0).is_equal_to(&reversed);
}

fn subtitled(deck: &Deck, t: &str) -> Deck {
    let mut d = deck.clone();
    d.slides[2].params.set("subtitle", crate::ParamValue::Text(t.into()));
    d
}

#[test]
fn applies_with_the_same_merge_key_are_one_undo_step() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let one = titled(&deck, "Un");
    r.apply(&one, "Modifier la slide", Some("k1")).unwrap();
    r.apply(&subtitled(&one, "Deux"), "Modifier le texte", Some("k1")).unwrap();
    assert_that!(r.undo_label()).is_equal_to(Some("Modifier la slide".to_owned()));
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&deck);
    assert_that!(r.can_undo()).is_false();
}

#[test]
fn apply_without_key_between_two_keyed_applies_splits_them() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let one = titled(&deck, "Un");
    r.apply(&one, "A", Some("k1")).unwrap();
    let two = subtitled(&one, "Agent");
    r.apply(&two, "Agent", None).unwrap();
    r.apply(&titled(&two, "Trois"), "B", Some("k1")).unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&two);
    assert_that!(r.undo_label()).is_equal_to(Some("Agent".to_owned()));
}

#[test]
fn apply_with_another_key_opens_a_new_step() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let one = titled(&deck, "Un");
    r.apply(&one, "A", Some("k1")).unwrap();
    r.apply(&titled(&deck, "Deux"), "B", Some("k2")).unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&one);
}

#[test]
fn keyed_apply_after_undo_does_not_join_the_redone_history() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let one = titled(&deck, "Un");
    r.apply(&one, "A", Some("k1")).unwrap();
    r.undo().unwrap();
    r.redo().unwrap();
    r.apply(&titled(&deck, "Deux"), "B", Some("k1")).unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&one);
}

#[test]
fn keyed_apply_after_undoing_a_later_step_opens_a_new_step() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let one = titled(&deck, "Un");
    r.apply(&one, "A", Some("k1")).unwrap();
    r.apply(&subtitled(&one, "Agent"), "Agent", None).unwrap();
    r.undo().unwrap();
    r.apply(&subtitled(&one, "Deux"), "B", Some("k1")).unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&one);
}

#[test]
fn undo_steps_counts_new_steps_but_not_merged_applies() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let start = r.undo_steps();
    r.apply(&titled(&deck, "Un"), "a", Some("k")).unwrap();
    assert_that!(r.undo_steps()).is_equal_to(start + 1);
    // Même clé : l'étape du sommet grandit.
    r.apply(&titled(&deck, "Deux"), "a", Some("k")).unwrap();
    assert_that!(r.undo_steps()).is_equal_to(start + 1);
    r.begin_group("Tour");
    r.apply(&titled(&deck, "Trois"), "a", Some("k")).unwrap();
    r.apply(&titled(&deck, "Quatre"), "a", Some("k")).unwrap();
    r.end_group();
    // Le groupe est une nouvelle étape, une seule.
    assert_that!(r.undo_steps()).is_equal_to(start + 2);
    r.apply(&titled(&deck, "Cinq"), "a", Some("k")).unwrap();
    assert_that!(r.undo_steps()).is_equal_to(start + 3);
}

#[test]
fn clear_undo_leaves_nothing_to_undo_or_redo() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.apply(&titled(&deck, "Un"), "Un", None).unwrap();
    r.apply(&titled(&deck, "Deux"), "Deux", None).unwrap();
    r.undo().unwrap();
    r.clear_undo();
    assert_that!((r.can_undo(), r.can_redo())).is_equal_to((false, false));
}

#[test]
fn clear_undo_inside_a_group_keeps_the_next_apply_undoable_under_the_group() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.begin_group("Tour");
    r.apply(&titled(&deck, "Un"), "Un", None).unwrap();
    r.clear_undo();
    r.apply(&titled(&deck, "Deux"), "Deux", None).unwrap();
    r.end_group();
    assert_that!(r.undo_label()).is_equal_to(Some("Tour".to_owned()));
    r.undo().unwrap();
    assert_that!(r.deck().0.slides[2].params.text("title")).is_equal_to(Some("Un"));
}
