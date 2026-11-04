use speculoos::prelude::*;

use crate::{Deck, DeckReplica};

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap()
}

fn titled(deck: &Deck, t: &str) -> Deck {
    let mut d = deck.clone();
    d.slides[2].params.set("title", crate::ParamValue::Text(t.into()));
    d
}

#[test]
fn group_step_names_the_open_groups_step_until_it_ends() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.begin_group("Tour");
    assert_that!(r.group_step()).is_none();
    r.apply(&titled(&deck, "Un"), "a", None).unwrap();
    assert_that!(r.group_step()).is_some();
    r.end_group();
    assert_that!(r.group_step()).is_none();
}

#[test]
fn resume_group_with_its_step_on_top_rejoins_it() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.begin_group("Tour");
    r.apply(&titled(&deck, "Un"), "a", None).unwrap();
    let step = r.group_step();
    r.undo().unwrap();
    r.redo().unwrap();
    r.resume_group("Tour", step);
    r.apply(&titled(&deck, "Deux"), "b", None).unwrap();
    r.end_group();
    r.undo().unwrap();
    assert_that!(r.deck().0).is_equal_to(&deck);
    assert_that!(r.can_undo()).is_false();
}

#[test]
fn resume_group_with_another_step_on_top_starts_a_new_step() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.begin_group("Tour");
    r.apply(&titled(&deck, "Un"), "a", None).unwrap();
    let step = r.group_step();
    r.end_group();
    r.apply(&titled(&deck, "Seul"), "Seul", None).unwrap();
    r.resume_group("Tour", step);
    r.apply(&titled(&deck, "Deux"), "b", None).unwrap();
    r.end_group();
    r.undo().unwrap();
    assert_that!(r.undo_label()).is_equal_to(Some("Seul".to_owned()));
}
