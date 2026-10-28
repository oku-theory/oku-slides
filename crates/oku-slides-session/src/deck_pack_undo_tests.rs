use speculoos::prelude::*;

use super::*;
use crate::test_support::{adopt_deck_only_pack, fresh_dir, opened_on, pin, rename_title, slide_widget};
use crate::test_support_packs::{LINKED_TAB, apply_valid, deck_layouts_pack, linked_owner, ok, owner_state};
use crate::undo::FAIL_OPPOSITE;
use crate::{Request, SessionErrorKind};

/// Links `tab` to `pack` in `owner`, the seed dropped.
fn link(owner: &mut Session, tab: &str, pack: Uuid) {
    drop(ok(owner.link_deck_pack(tab, "Mise en page", pack), "link"));
}

/// Asserts that `req` is refused with `kind`, the deck, its pins and its undo stacks unchanged.
fn assert_stopped(owner: &mut Session, req: Request, kind: SessionErrorKind) {
    let (before, pins) = (owner_state(owner), owner.linked_pins());
    let refused = owner.handle(req).map(|_| ()).unwrap_err();
    assert_that!(refused.kind()).is_equal_to(kind);
    assert_that!(owner.linked_pins()).is_equal_to(pins);
    assert_that!(owner_state(owner)).is_equal_to(before);
}

#[test]
fn owner_undo_stops_at_linked_pack_step() {
    let data = fresh_dir("oku-slides-deck-pack-undo-");
    let (mut owner, _) = linked_owner(data.path());
    apply_valid(&mut owner, LINKED_TAB);
    assert_stopped(&mut owner, Request::Undo, SessionErrorKind::UndoInPackTab);
}

#[test]
fn owner_undo_of_a_pre_link_pack_step_is_refused() {
    let data = fresh_dir("oku-slides-deck-pack-undo-");
    let mut owner = opened_on(data.path());
    let layouts = deck_layouts_pack(&mut owner);
    link(&mut owner, LINKED_TAB, layouts.id);
    assert_stopped(&mut owner, Request::Undo, SessionErrorKind::UndoInPackTab);
}

#[test]
fn owner_redo_of_a_linked_pack_step_is_refused() {
    let data = fresh_dir("oku-slides-deck-pack-undo-");
    let mut owner = opened_on(data.path());
    let layouts = deck_layouts_pack(&mut owner);
    ok(owner.set_widget(None, "cover2", slide_widget("cover2")), "second layout");
    owner.end_widget_session();
    ok(owner.undo(), "undo the second layout");
    link(&mut owner, LINKED_TAB, layouts.id);
    assert_stopped(&mut owner, Request::Redo, SessionErrorKind::UndoInPackTab);
}

#[test]
fn owner_undo_agent_turn_over_a_linked_pack_step_is_refused() {
    let data = fresh_dir("oku-slides-deck-pack-undo-");
    let mut owner = opened_on(data.path());
    owner.begin_turn(1, "mise en page");
    ok(owner.set_widget(None, "cover", slide_widget("cover")), "turn's layout");
    owner.end_turn();
    let layouts = pin(&owner, "deck-layouts").unwrap();
    link(&mut owner, LINKED_TAB, layouts.id);
    let (before, pins) = (owner_state(&owner), owner.linked_pins());
    let refused = owner.undo_agent_turn(1).unwrap_err();
    assert_that!(refused.kind()).is_equal_to(SessionErrorKind::UndoInPackTab);
    assert_that!(owner.linked_pins()).is_equal_to(pins);
    assert_that!(owner_state(&owner)).is_equal_to(before);
}

#[test]
fn owner_undo_of_another_step_still_passes() {
    let data = fresh_dir("oku-slides-deck-pack-undo-");
    let (mut owner, _) = linked_owner(data.path());
    apply_valid(&mut owner, LINKED_TAB);
    let title = owner.deck().params.title.clone();
    ok(owner.apply("titre", vec![rename_title("Autre titre")], None), "rename");
    ok(owner.handle(Request::Undo), "undo the rename");
    assert_that!(owner.deck().params.title).is_equal_to(title);
}

#[test]
fn owner_undo_after_unlink_undoes_the_pack_step() {
    let data = fresh_dir("oku-slides-deck-pack-undo-");
    let (mut owner, opened) = linked_owner(data.path());
    apply_valid(&mut owner, LINKED_TAB);
    owner.unlink_deck_pack(LINKED_TAB);
    ok(owner.handle(Request::Undo), "undo");
    assert_that!(pin(&owner, "deck")).is_equal_to(Some(opened));
}

#[test]
fn owner_undo_compensation_failure_is_not_undo_in_pack_tab() {
    let data = fresh_dir("oku-slides-deck-pack-undo-");
    let (mut owner, _) = linked_owner(data.path());
    apply_valid(&mut owner, LINKED_TAB);
    owner.doc.dirty = false;
    FAIL_OPPOSITE.with(|fail| fail.set(true));
    let refused = owner.handle(Request::Undo).map(|_| ()).unwrap_err();
    FAIL_OPPOSITE.with(|fail| fail.set(false));
    assert_that!(refused.kind()).is_equal_to(SessionErrorKind::UndoCompensationFailed);
    // The step stays played: the deck says so and its widgets follow its pins.
    let synced = owner.widgets_pins == crate::deck_closure::combined_pins(owner.doc.sources.deck());
    assert_that!((owner.doc.dirty, synced)).is_equal_to((true, true));
}

#[test]
fn owner_receive_after_a_refused_undo_starts_a_new_step() {
    let data = fresh_dir("oku-slides-deck-pack-undo-");
    let (mut owner, _) = linked_owner(data.path());
    apply_valid(&mut owner, LINKED_TAB);
    assert_that!(owner.undo().map_err(|e| e.kind())).is_equal_to(Err(SessionErrorKind::UndoInPackTab));
    let steps = owner.doc.sources.undo_steps();
    apply_valid(&mut owner, LINKED_TAB);
    assert_that!(owner.doc.sources.undo_steps()).is_equal_to(steps + 1);
}

#[test]
// A regression guard only: it cannot isolate decision 1's "compensating undo cuts the merge",
// since any redo follows an undo that already reset the merge key of the step below.
fn owner_refused_redo_cuts_the_merge_of_the_step_below() {
    let data = fresh_dir("oku-slides-deck-pack-undo-");
    let mut owner = opened_on(data.path());
    let widgets = adopt_deck_only_pack(&mut owner);
    let layouts = deck_layouts_pack(&mut owner);
    link(&mut owner, LINKED_TAB, widgets.id);
    // The step below: the first receive of the linked tab, keyed to merge the next ones.
    apply_valid(&mut owner, LINKED_TAB);
    ok(owner.set_widget(None, "cover2", slide_widget("cover2")), "second layout");
    owner.end_widget_session();
    ok(owner.undo(), "undo the second layout");
    link(&mut owner, "t-3", layouts.id);
    assert_that!(owner.redo().map_err(|e| e.kind())).is_equal_to(Err(SessionErrorKind::UndoInPackTab));
    let steps = owner.doc.sources.undo_steps();
    apply_valid(&mut owner, LINKED_TAB);
    assert_that!(owner.doc.sources.undo_steps()).is_equal_to(steps + 1);
}
