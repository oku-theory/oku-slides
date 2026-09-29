//! Loro 1.16.2 : défaire un item d'annulation qui porte plusieurs `mov` sur
//! une `LoroMovableList` de conteneurs perd des éléments (suppression
//! durable dans l'oplog), voire panique. Échanger deux slides non voisines
//! suffisait : 42 échanges sur 56 perdaient une slide du fixture.

use oku_slides_core::{Deck, ParamValue, SourceBundle};
use speculoos::prelude::*;

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("fixture : {e}"))
}

fn bundle(deck: &Deck) -> SourceBundle {
    SourceBundle::new(deck.clone(), crate::common::shared()).unwrap_or_else(|e| panic!("bundle : {e}"))
}

/// Générateur congruentiel : des permutations reproductibles sans dépendance.
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % n
    }
}

/// Une édition au hasard : permutation, suppression, ajout et titre changé.
fn edited(deck: &Deck, rng: &mut Lcg, round: usize) -> Deck {
    let mut d = deck.clone();
    for k in (1..d.slides.len()).rev() {
        let j = rng.below(k + 1);
        d.slides.swap(k, j);
    }
    if rng.below(2) == 0 && d.slides.len() > 2 {
        let gone = rng.below(d.slides.len());
        d.slides.remove(gone);
    }
    if rng.below(2) == 0 {
        let mut copy = d.slides[rng.below(d.slides.len())].clone();
        copy.id = format!("s-copie-{round}");
        let at = rng.below(d.slides.len() + 1);
        d.slides.insert(at, copy);
    }
    let t = rng.below(d.slides.len());
    d.slides[t].params.set("title", ParamValue::Text(format!("Titre {round}")));
    d
}

#[test]
fn undo_swap_of_any_two_slides_restores_the_deck() {
    let deck = roadmap();
    for i in 0..deck.slides.len() {
        for j in 0..deck.slides.len() {
            if i == j {
                continue;
            }
            let mut b = bundle(&deck);
            let mut swapped = deck.clone();
            swapped.slides.swap(i, j);
            b.apply(swapped.clone(), "Échanger", None).unwrap_or_else(|e| panic!("apply : {e}"));
            assert_that!(b.undo().unwrap_or_else(|e| panic!("undo : {e}"))).is_true();
            assert_that!(b.replica().deck().0).named(&format!("annuler {i}↔{j}")).is_equal_to(&deck);
            assert_that!(b.redo().unwrap_or_else(|e| panic!("redo : {e}"))).is_true();
            assert_that!(b.replica().deck().0).named(&format!("rétablir {i}↔{j}")).is_equal_to(&swapped);
        }
    }
}

#[test]
fn undo_and_redo_random_edit_sequences_walk_back_and_forth_exactly() {
    for seed in [1_u64, 7, 42, 2026] {
        let mut rng = Lcg(seed);
        let mut b = bundle(&roadmap());
        let mut states = vec![roadmap()];
        for round in 0..6 {
            let next = edited(states.last().unwrap_or_else(|| unreachable!()), &mut rng, round);
            b.apply(next.clone(), &format!("Étape {round}"), None).unwrap_or_else(|e| panic!("apply : {e}"));
            states.push(next);
        }
        for (k, want) in states.iter().enumerate().rev().skip(1) {
            assert_that!(b.replica().undo_label()).is_equal_to(Some(format!("Étape {k}")));
            assert_that!(b.undo().unwrap_or_else(|e| panic!("undo : {e}"))).is_true();
            assert_that!(b.replica().deck().0).named(&format!("graine {seed}, annuler vers {k}")).is_equal_to(want);
        }
        assert_that!(b.replica().can_undo()).is_false();
        for (k, want) in states.iter().enumerate().skip(1) {
            assert_that!(b.redo().unwrap_or_else(|e| panic!("redo : {e}"))).is_true();
            assert_that!(b.replica().deck().0).named(&format!("graine {seed}, rétablir vers {k}")).is_equal_to(want);
        }
    }
}

#[test]
fn undo_group_of_several_reorders_restores_the_deck_in_one_step() {
    let deck = roadmap();
    let mut b = bundle(&deck);
    b.begin_group("Agent");
    let mut d = deck.clone();
    for (i, j) in [(0, 5), (1, 7), (2, 4)] {
        d.slides.swap(i, j);
        b.apply(d.clone(), "Échanger", None).unwrap_or_else(|e| panic!("apply : {e}"));
    }
    b.end_group();
    assert_that!(b.replica().undo_label()).is_equal_to(Some("Agent".to_owned()));
    assert_that!(b.undo().unwrap_or_else(|e| panic!("undo : {e}"))).is_true();
    assert_that!(b.replica().deck().0).is_equal_to(&deck);
    assert_that!(b.replica().can_undo()).is_false();
    assert_that!(b.redo().unwrap_or_else(|e| panic!("redo : {e}"))).is_true();
    assert_that!(b.replica().deck().0).is_equal_to(&d);
}

#[test]
fn undo_limit_counts_a_reorder_as_one_step() {
    let deck = roadmap();
    let mut b = bundle(&deck);
    b.set_undo_limit(2);
    let mut reversed = deck.clone();
    reversed.slides.reverse();
    b.apply(reversed.clone(), "Inverser", None).unwrap_or_else(|e| panic!("apply : {e}"));
    let mut back = reversed.clone();
    back.slides.reverse();
    b.apply(back.clone(), "Remettre", None).unwrap_or_else(|e| panic!("apply : {e}"));
    assert_that!(b.undo().unwrap_or_else(|e| panic!("undo : {e}"))).is_true();
    assert_that!(b.undo().unwrap_or_else(|e| panic!("undo : {e}"))).is_true();
    assert_that!(b.replica().deck().0).is_equal_to(&deck);
}
