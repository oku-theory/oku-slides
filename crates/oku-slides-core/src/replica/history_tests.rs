use speculoos::prelude::*;

use crate::{Deck, SourceBundle, slide_changes};

fn roadmap() -> Deck {
    let mut deck = Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap();
    crate::canonicalize(&mut deck);
    deck
}

fn titled(deck: &Deck, t: &str) -> Deck {
    let mut d = deck.clone();
    d.slides[2].title = Some(t.into());
    d
}

fn edited(labels: &[&str]) -> (Deck, SourceBundle) {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    b.set_author(Some("Julie".into()));
    for l in labels {
        b.apply(titled(&deck, l), l).unwrap();
    }
    (deck, b)
}

#[test]
fn history_lists_creation_then_one_version_per_commit() {
    let (_, b) = edited(&["Un", "Deux"]);
    let h = b.history();
    assert_that!(h.len()).is_equal_to(3);
    let labels: Vec<_> = h.versions().iter().map(|v| v.label.clone().unwrap_or_default()).collect();
    assert_that!(labels[1..].to_vec()).is_equal_to(vec!["Un".to_owned(), "Deux".to_owned()]);
    assert_that!(h.versions()[1].author.as_deref()).is_equal_to(Some("Julie"));
    // Ruling 1 : `DeckReplica::new` committe la création avec le même pair
    // Loro que les modifications suivantes, donc le début de session est au
    // premier cran (index 0), pas au premier commit étiqueté (index 1).
    assert_that!(h.versions()[0].session_start).is_true();
    assert_that!(h.versions()[1].session_start).is_false();
    assert_that!(h.versions()[2].session_start).is_false();
}

#[test]
fn deck_at_reads_the_state_after_each_version() {
    let (deck, b) = edited(&["Un", "Deux"]);
    let h = b.history();
    assert_that!(h.deck_at(0).unwrap().0).is_equal_to(&deck);
    assert_that!(h.deck_at(1).unwrap().0.slides[2].title.as_deref()).is_equal_to(Some("Un"));
    assert_that!(h.deck_at(99).unwrap().0.slides[2].title.as_deref()).is_equal_to(Some("Deux"));
}

#[test]
fn history_marks_the_merge_and_keeps_concurrent_branches_in_each_prefix() {
    let deck = roadmap();
    let base = SourceBundle::new(deck.clone()).unwrap();
    let zip = base.to_zip().unwrap();
    let limits = crate::SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 28 };
    let (mut a, _) = SourceBundle::from_zip(&zip, limits).unwrap();
    let (mut b, _) = SourceBundle::from_zip(&zip, limits).unwrap();
    a.apply(titled(&deck, "de A"), "A").unwrap();
    let mut d = deck.clone();
    d.theme = Some("pro".into());
    b.apply(d, "B").unwrap();
    a.merge(b).unwrap();
    let mut after = a.deck().clone();
    after.slides[3].title = Some("après".into());
    a.apply(after, "Après").unwrap();
    let h = a.history();
    let last = h.len() - 1;
    assert_that!(h.versions()[last].merge).is_true();
    // Le cran juste avant la fusion contient les deux branches (préfixe, pas ancêtres).
    let before = h.deck_at(last - 1).unwrap().0;
    assert_that!(before.slides[2].title.as_deref()).is_equal_to(Some("de A"));
    assert_that!(before.theme.as_deref()).is_equal_to(Some("pro"));
}

#[test]
fn restore_brings_back_a_version_as_a_new_undoable_commit() {
    let (_, mut b) = edited(&["Un", "Deux"]);
    let h = b.history();
    b.restore(&h, 1, "Restaurer").unwrap();
    assert_that!(b.deck().slides[2].title.as_deref()).is_equal_to(Some("Un"));
    assert_that!(b.history().len()).is_equal_to(4);
    assert_that!(b.replica().undo_label()).is_equal_to(Some("Restaurer".to_owned()));
    b.undo().unwrap();
    assert_that!(b.deck().slides[2].title.as_deref()).is_equal_to(Some("Deux"));
}

#[test]
fn restore_keeps_authors_of_later_sessions() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    let old = b.history();
    b.set_author(Some("Julie".into()));
    b.apply(titled(&deck, "Un"), "Un").unwrap();
    let julie = b.replica().peer_id();
    b.restore(&old, 0, "Restaurer").unwrap();
    assert_that!(b.replica().author_of(julie)).is_equal_to(Some("Julie".to_owned()));
}

#[test]
fn restore_with_no_prior_edit_does_not_leave_a_spurious_unlabeled_version() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    b.set_author(Some("Julie".into()));
    let h = b.history();
    b.restore(&h, 0, "Restaurer").unwrap();
    // Un seul cran de plus (« Restaurer ») : pas de commit parasite sans
    // libellé venant d'un `claim_author` pris à part avant `revert_to`.
    assert_that!(b.history().len()).is_equal_to(2);
    assert_that!(b.replica().undo_label()).is_equal_to(Some("Restaurer".to_owned()));
    let peer = b.replica().peer_id();
    assert_that!(b.replica().author_of(peer)).is_equal_to(Some("Julie".to_owned()));
    b.undo().unwrap();
    assert_that!(b.replica().undo_label()).is_equal_to(None);
}

/// Restaure la version `i` de `h` et vérifie que le deck est exactement
/// cette version, en une étape d'annulation au libellé donné.
fn restore_exactly(b: &mut SourceBundle, h: &super::HistoryPreview, i: usize) {
    let want = h.deck_at(i).unwrap().0;
    b.restore(h, i, "Restaurer").unwrap();
    assert_that!(b.deck().slides.len()).named(&format!("slides, version {i}")).is_equal_to(want.slides.len());
    assert_that!(b.deck()).named(&format!("version {i}")).is_equal_to(&want);
    assert_that!(b.replica().deck().0).is_equal_to(&want);
    assert_that!(b.replica().undo_label()).is_equal_to(Some("Restaurer".to_owned()));
}

/// [`restore_exactly`], puis annuler ramène le deck d'avant.
fn restore_and_undo(b: &mut SourceBundle, h: &super::HistoryPreview, i: usize) {
    let before = b.deck().clone();
    restore_exactly(b, h, i);
    b.undo().unwrap();
    assert_that!(b.deck()).named(&format!("annuler, version {i}")).is_equal_to(&before);
}

/// Le deck avec la slide `from` déplacée en `to` (un glisser de l'interface).
fn moved(deck: &Deck, from: usize, to: usize) -> Deck {
    let mut d = deck.clone();
    let s = d.slides.remove(from);
    d.slides.insert(to, s);
    d
}

/// Le deck avec les slides `a` et `b` échangées.
fn swapped(deck: &Deck, a: usize, b: usize) -> Deck {
    let mut d = deck.clone();
    d.slides.swap(a, b);
    d
}

/// Le deck sans la slide `i`.
fn without(deck: &Deck, i: usize) -> Deck {
    let mut d = deck.clone();
    d.slides.remove(i);
    d
}

#[test]
fn restore_across_a_slide_move_brings_back_every_slide() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    b.apply(moved(&deck, 3, 0), "Déplacer").unwrap();
    let h = b.history();
    restore_and_undo(&mut b, &h, 0);
}

#[test]
fn restore_across_a_delete_then_a_move_brings_back_each_earlier_version() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    let fewer = without(&deck, 2);
    b.apply(fewer.clone(), "Supprimer").unwrap();
    b.apply(moved(&fewer, 4, 1), "Déplacer").unwrap();
    let h = b.history();
    for i in 0..h.len() - 1 {
        restore_and_undo(&mut b, &h, i);
    }
}

#[test]
fn restore_after_a_zip_round_trip_brings_back_each_earlier_version() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    let fewer = without(&deck, 5);
    b.apply(fewer.clone(), "Supprimer").unwrap();
    b.apply(moved(&fewer, 0, 4), "Déplacer").unwrap();
    let limits = crate::SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 28 };
    let (mut b, _) = SourceBundle::from_zip(&b.to_zip().unwrap(), limits).unwrap();
    let h = b.history();
    for i in 0..h.len() - 1 {
        restore_and_undo(&mut b, &h, i);
    }
}

/// Le cas de la revue : supprimer, échanger deux slides (plusieurs `mov` en
/// un commit), puis restaurer chaque version. `revert_to` y perdait une
/// slide ou échouait (`ContainerDeleted`).
#[test]
fn restore_across_a_delete_then_a_swap_brings_back_each_earlier_version() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    let fewer = without(&deck, 2);
    b.apply(fewer.clone(), "Supprimer").unwrap();
    b.apply(swapped(&fewer, 0, 3), "Échanger").unwrap();
    let h = b.history();
    for i in 0..h.len() - 1 {
        restore_exactly(&mut b, &h, i);
    }
}

/// Un échange de slides non voisines, annulé : t-263 (un `mov` par commit)
/// le rend sans perte, restauration comprise.
#[test]
fn restore_across_a_swap_can_be_undone() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    b.apply(swapped(&deck, 0, 3), "Échanger").unwrap();
    let h = b.history();
    restore_and_undo(&mut b, &h, 0);
}

/// Un réordonnancement fait plusieurs changes Loro (un `mov` par commit),
/// mais une seule édition : un seul cran, à l'état d'après, sous son libellé.
#[test]
fn a_reorder_of_several_slides_is_one_version() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    let mut reordered = deck.clone();
    reordered.slides.reverse();
    b.apply(reordered.clone(), "Réordonner").unwrap();
    assert_that!(b.replica().len_changes()).is_greater_than(2);
    let h = b.history();
    assert_that!(h.len()).is_equal_to(2);
    assert_that!(h.versions()[1].label.as_deref()).is_equal_to(Some("Réordonner"));
    assert_that!(h.deck_at(1).unwrap().0).is_equal_to(&reordered);
}

#[test]
fn restore_across_a_reorder_is_one_version() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone()).unwrap();
    b.apply(swapped(&deck, 0, 3), "Échanger").unwrap();
    let h = b.history();
    let before = h.len();
    restore_exactly(&mut b, &h, 0);
    let after = b.history();
    assert_that!(after.len()).is_equal_to(before + 1);
    assert_that!(after.versions()[before].label.as_deref()).is_equal_to(Some("Restaurer"));
    assert_that!(after.deck_at(before).unwrap().0).is_equal_to(&deck);
}

#[test]
fn restore_from_an_unreadable_history_fails_and_leaves_the_replica_as_its_view() {
    let (_, mut b) = edited(&["Un", "Deux"]);
    let before = (b.deck().clone(), b.replica().len_changes());
    let h = b.history();
    // Un aperçu qui a paniqué (version illisible) : ses verrous sont empoisonnés.
    h.poison_for_tests();
    assert_that!(b.restore(&h, 0, "Restaurer").is_err()).is_true();
    assert_that!(b.deck()).is_equal_to(&before.0);
    assert_that!(b.replica().deck().0).is_equal_to(&before.0);
    // Rien en attente : un export ne committe rien de plus.
    b.to_zip().unwrap();
    assert_that!(b.replica().len_changes()).is_equal_to(before.1);
    assert_that!(b.replica().undo_label()).is_equal_to(Some("Deux".to_owned()));
}

#[test]
fn slide_changes_compares_slides_by_id() {
    let current = roadmap();
    let mut version = current.clone();
    version.slides[2].title = Some("autre".into());
    let removed = version.slides.remove(4).id;
    let mut extra = version.slides[0].clone();
    extra.id = "s-disparue".into();
    version.slides.push(extra);
    let c = slide_changes(&version, &current);
    assert_that!(c.changed_since).is_equal_to(vec![current.slides[2].id.clone()]);
    assert_that!(c.removed_since).is_equal_to(vec!["s-disparue".to_owned()]);
    assert_that!(c.added_since).is_equal_to(vec![removed]);
}
