use std::time::Duration;

use speculoos::prelude::*;

use crate::test_support::{session, theme_choices};
use crate::theme_picker::{Bounds, THUMB_BYTES};

const WIDE: Bounds = Bounds { budget: Duration::from_secs(600), thumb_bytes: THUMB_BYTES };

/// The active theme's thumbnail of `slide`, and the deck's own render of it.
fn active_thumbnail(slide: &str) -> (Option<std::sync::Arc<str>>, std::sync::Arc<str>) {
    let mut s = session();
    let choices = theme_choices(&mut s, Some(slide), WIDE).unwrap().unwrap();
    let active = choices.cards.into_iter().find(|c| c.namespace == choices.active).unwrap();
    (active.svg, s.svg(slide).unwrap())
}

/// The outline lists every chapter of the deck: a one-slide clone would list none (t-1076).
#[test]
fn theme_job_active_thumbnail_of_the_outline_matches_the_deck_render() {
    let (thumbnail, deck) = active_thumbnail("s-toc");
    assert_that!(thumbnail.as_deref()).is_equal_to(Some(&*deck));
}

/// Page numbers count from the deck's first slide, not from the thumbnail's own (t-1076).
#[test]
fn theme_job_active_thumbnail_of_a_later_slide_matches_the_deck_render() {
    let (thumbnail, deck) = active_thumbnail("s-libre");
    assert_that!(thumbnail.as_deref()).is_equal_to(Some(&*deck));
}

#[test]
fn theme_job_hands_the_preview_slot_back_to_the_session() {
    let mut s = session();
    theme_choices(&mut s, None, WIDE).unwrap();
    assert_that!(s.variant_renderer.is_some()).is_true();
}

#[test]
fn theme_job_lends_the_slot_while_it_is_out() {
    let mut s = session();
    let _job = s.theme_job_within(None, WIDE).unwrap().unwrap();
    assert_that!(s.variant_renderer.is_none()).is_true();
}

/// Another preview opened a slot while the job was out: the session keeps that one.
#[test]
fn theme_job_slot_returned_after_another_opened_is_dropped() {
    let mut s = session();
    let first = s.theme_job_within(None, WIDE).unwrap().unwrap();
    let (_, other) = s.theme_job_within(None, WIDE).unwrap().unwrap().run();
    let kept = format!("{:?}", other.as_ref().map(|p| &p.slot));
    s.restore_preview_slot(other);
    let (_, lent) = first.run();
    s.restore_preview_slot(lent);
    assert_that!(format!("{:?}", s.variant_renderer.as_ref())).is_equal_to(kept);
}

/// The session keeps working while a job is out (t-1085): an edit lands meanwhile.
#[test]
fn theme_job_out_leaves_the_session_editable() {
    let mut s = session();
    let job = s.theme_job_within(None, WIDE).unwrap().unwrap();
    s.pick_theme(&crate::test_support::ns(crate::PRO_NAMESPACE)).unwrap();
    s.restore_preview_slot(job.run().1);
    assert_that!(s.deck().theme.clone()).is_equal_to(Some(crate::test_support::ns(crate::PRO_NAMESPACE)));
}

#[test]
fn theme_job_run_after_an_edit_still_gives_cards() {
    let mut s = session();
    let job = s.theme_job_within(None, WIDE).unwrap().unwrap();
    s.pick_theme(&crate::test_support::ns(crate::PRO_NAMESPACE)).unwrap();
    assert_that!(job.run().0.cards.is_empty()).is_false();
}

#[test]
fn theme_job_slot_lent_before_a_reset_is_dropped_on_return() {
    let mut s = session();
    let job = s.theme_job_within(None, WIDE).unwrap().unwrap();
    s.reset_preview_slot();
    let (_, slot) = job.run();
    s.restore_preview_slot(slot);
    assert_that!(s.variant_renderer.is_none()).is_true();
}

#[test]
fn theme_job_on_an_empty_deck_lends_no_slot() {
    let mut s = session();
    let ids: Vec<String> = s.deck().slides.iter().map(|sl| sl.id.clone()).collect();
    let ops = ids.into_iter().map(|slide| oku_slides_core::Op::RemoveSlide { slide }).collect();
    s.apply("Vider", ops, None).unwrap();
    let (choices, slot) = s.theme_job_within(None, WIDE).unwrap().unwrap().run();
    assert_that!((slot.is_none(), choices.cards.iter().all(|c| c.svg.is_none()))).is_equal_to((true, true));
}

/// The job shares the session's assets: taking them under the lock copies no byte (t-1388).
#[test]
fn theme_job_shares_the_session_assets() {
    let mut s = session();
    let job = s.theme_job_within(None, WIDE).unwrap().unwrap();
    assert_that!(std::sync::Arc::ptr_eq(job.assets(), &s.doc.sources.assets)).is_true();
}
