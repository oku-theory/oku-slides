use oku_slides_i18n::{Localize, Localizer};
use oku_slides_session::agent::TabOpen;
use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::TabVerdict;
use crate::hub::tests::hub;
use crate::hub::{Reserve, TabTarget};

const ACME: Uuid = Uuid::from_u128(0xac);
const PAPIER: Uuid = Uuid::from_u128(0xfa);

/// A real session error: a pack tab on a pack nobody knows.
fn unknown_tab_error() -> oku_slides_session::SessionError {
    use oku_slides_session::{Fonts, Platform, Session, Settings};
    let data = tempfile::tempdir().unwrap_or_else(|e| panic!("test dir: {e}"));
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
    settings.paths.data = Some(data.path().to_owned());
    settings.paths.decks = Some(data.path().join("decks"));
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}"));
    let mut session = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session: {e}"));
    match session.open_pack_tab_here(Uuid::from_u128(0xdead), None) {
        Err(e) => e,
        Ok(()) => panic!("an unknown pack must not open"),
    }
}

/// t-1191 bug 3: a refusal noted on Focus is cleared by a later grant.
#[test]
fn verdict_after_a_refused_focus_then_a_granted_new_tab_reports_no_refusal() {
    let hub = hub();
    let turn = hub.begin_turn();
    let mut v = TabVerdict::default();
    // The Focus, asked for a turn that is not open, then the NewTab.
    assert_that!(v.ask(&hub, Some(turn + 1), TabTarget::Pack(ACME))).is_false();
    assert_that!(v.ask(&hub, Some(turn), TabTarget::Pack(ACME))).is_true();
    assert_that!(v.refusal.is_none()).is_true();
}

/// Decision 20: a Focus spends the turn's one move.
#[test]
fn verdict_focus_granted_claims_the_turn() {
    let hub = hub();
    let turn = hub.begin_turn();
    let mut v = TabVerdict::default();
    assert_that!(v.ask(&hub, Some(turn), TabTarget::Pack(ACME))).is_true();
    assert_that!(hub.reserve_tab(turn, TabTarget::Theme(PAPIER))).is_equal_to(Reserve::Taken);
}

/// The second gate of one call rides on the first one's claim.
#[test]
fn verdict_new_tab_after_a_granted_focus_in_the_same_call_is_granted() {
    let hub = hub();
    let turn = hub.begin_turn();
    let mut v = TabVerdict::default();
    let _ = v.ask(&hub, Some(turn), TabTarget::Pack(ACME));
    assert_that!(v.ask(&hub, Some(turn), TabTarget::Pack(ACME))).is_true();
}

#[test]
fn verdict_after_a_claim_refuses_once_the_turn_is_over() {
    let hub = hub();
    let turn = hub.begin_turn();
    let mut v = TabVerdict::default();
    let _ = v.ask(&hub, Some(turn), TabTarget::Pack(ACME));
    hub.end_turn(turn, "");
    assert_that!(v.ask(&hub, Some(turn), TabTarget::Pack(ACME))).is_false();
}

#[test]
fn verdict_outside_a_turn_refuses_with_no_turn() {
    let hub = hub();
    let mut v = TabVerdict::default();
    assert_that!(v.ask(&hub, None, TabTarget::Theme(PAPIER))).is_false();
    let fr = Localizer::new(Some("fr"));
    assert_that!(v.refusal().localize(&fr)).is_equal_to(fr.tr("agent-error-tab-no-turn"));
}

/// t-1191 bug 2: an outcome that moved no tab gives the claim back.
#[rstest]
#[case::here(Ok(TabOpen::Here))]
#[case::refused(Ok(TabOpen::Refused))]
#[case::error(Err(unknown_tab_error()))]
fn settle_without_a_move_releases_the_claim(#[case] outcome: oku_slides_session::Result<TabOpen>) {
    let hub = hub();
    let turn = hub.begin_turn();
    let mut v = TabVerdict::default();
    assert_that!(v.ask(&hub, Some(turn), TabTarget::Pack(ACME))).is_true();
    v.settle(&hub, Some(turn), TabTarget::Pack(ACME), &outcome);
    assert_that!(hub.reserve_tab(turn, TabTarget::Theme(PAPIER))).is_equal_to(Reserve::Granted);
}

#[rstest]
#[case::opened(TabOpen::Opened { tab: "t-2".into(), shown: true })]
#[case::focused(TabOpen::Focused { tab: "t-2".into(), shown: false })]
fn settle_with_a_move_keeps_the_claim_and_notes_writes_elsewhere(#[case] moved: TabOpen) {
    let hub = hub();
    let turn = hub.begin_turn();
    let mut v = TabVerdict::default();
    let _ = v.ask(&hub, Some(turn), TabTarget::Pack(ACME));
    v.settle(&hub, Some(turn), TabTarget::Pack(ACME), &Ok(moved));
    assert_that!((hub.reserve_tab(turn, TabTarget::Theme(PAPIER)), hub.writes_elsewhere(turn)))
        .is_equal_to((Reserve::Taken, true));
}

#[test]
fn settle_without_a_claim_releases_nothing() {
    let hub = hub();
    let turn = hub.begin_turn();
    let _ = hub.reserve_tab(turn, TabTarget::Pack(ACME)); // another call's claim
    let v = TabVerdict::default();
    v.settle(&hub, Some(turn), TabTarget::Pack(ACME), &Ok(TabOpen::Here));
    assert_that!(hub.reserve_tab(turn, TabTarget::Theme(PAPIER))).is_equal_to(Reserve::Taken);
}

/// A move notes writes elsewhere by itself, with no claim held that would already say so.
#[test]
fn settle_with_a_move_and_no_claim_notes_writes_elsewhere() {
    let hub = hub();
    let turn = hub.begin_turn();
    let v = TabVerdict::default();
    v.settle(&hub, Some(turn), TabTarget::Pack(ACME), &Ok(TabOpen::Focused { tab: "t-2".into(), shown: true }));
    assert_that!(hub.writes_elsewhere(turn)).is_true();
}
