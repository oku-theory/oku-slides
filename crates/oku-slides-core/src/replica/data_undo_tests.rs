//! Undo steps record the data they touch, so a compaction can tell whether undo needs it.

use super::*;
use crate::replica::Dropped;

fn two() -> Deck {
    deck_of(vec![("d1", data("sales", table())), ("d2", data("costs", table()))])
}

fn cell_edit(deck: &Deck, id: &str) -> Deck {
    edited(deck, id, |d| set_cell(d.table_mut().unwrap(), "r2", "c2", DataCell::Number(7.0)))
}

#[test]
fn undo_step_touches_only_the_edited_data() {
    let deck = two();
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    r.apply(&cell_edit(&deck, "d1"), "Edit", None).unwrap();
    assert_that!(r.undo_touches_data(&did("d1"))).is_true();
    assert_that!(r.undo_touches_data(&did("d2"))).is_false();
}

#[test]
fn redo_step_still_touches_the_data() {
    let deck = two();
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    r.apply(&cell_edit(&deck, "d1"), "Edit", None).unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.undo_touches_data(&did("d1"))).is_true();
}

#[test]
fn silent_apply_touches_no_undo_step() {
    let deck = two();
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    r.apply_silent(&cell_edit(&deck, "d1"), crate::KnowledgeSync::Changed).unwrap();
    assert_that!(r.undo_touches_data(&did("d1"))).is_false();
}

#[test]
fn drop_data_undo_cuts_both_stacks() {
    let deck = two();
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let one = cell_edit(&deck, "d1");
    r.apply(&one, "One", None).unwrap();
    r.apply(&cell_edit(&one, "d2"), "Two", None).unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.drop_data_undo(&did("d1"))).is_equal_to(Dropped::Cut);
    assert_that!(r.can_undo()).is_false();
    assert_that!(r.redo_label()).is_none();
    assert_that!(r.undo_touches_data(&did("d2"))).is_false();
}

#[test]
fn drop_data_undo_without_a_touching_step_keeps_the_stacks() {
    let deck = two();
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    r.apply(&cell_edit(&deck, "d2"), "Two", None).unwrap();
    assert_that!(r.drop_data_undo(&did("d1"))).is_equal_to(Dropped::Nothing);
    assert_that!(r.can_undo()).is_true();
}
