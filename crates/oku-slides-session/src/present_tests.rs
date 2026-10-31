use oku_slides_core::Op;
use speculoos::prelude::*;

use crate::lib_tests_support::{data_dir, session_in};
use crate::test_support::session;
use crate::theme::PAPIER_THEME;
use crate::{Localize, PresentRefusal, Request, Session, SessionErrorKind};

fn refusal(s: &Session) -> Option<PresentRefusal> {
    let Err(e) = s.can_present() else { panic!("presenting should be refused") };
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::CannotPresent);
    e.present_refusal()
}

#[test]
fn can_present_a_deck_with_slides_returns_ok() {
    let s = session();
    assert_that!(s.can_present()).is_ok();
    assert_that!(s.snapshot().can_present).is_true();
}

#[test]
fn can_present_in_the_read_only_help_returns_help() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    s.open_help("fr", None).unwrap();
    assert_that!(refusal(&s)).is_equal_to(Some(PresentRefusal::Help));
    assert_that!(s.snapshot().can_present).is_false();
}

#[test]
fn can_present_in_a_pack_tab_returns_pack() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    s.open_pack_tab_here(crate::ESSENTIEL_PACK, None).unwrap();
    assert_that!(refusal(&s)).is_equal_to(Some(PresentRefusal::Pack));
}

#[test]
fn can_present_in_a_theme_tab_returns_theme() {
    let data = crate::test_support::fresh_dir("oku-slides-present-");
    let s = crate::theme_tab::theme_tab_tests::theme_tab(data.path(), PAPIER_THEME);
    assert_that!(refusal(&s)).is_equal_to(Some(PresentRefusal::Theme));
}

#[test]
fn can_present_with_history_open_returns_history() {
    let mut s = session();
    s.handle(Request::HistoryOpen).unwrap();
    assert_that!(refusal(&s)).is_equal_to(Some(PresentRefusal::History));
}

#[test]
fn can_present_a_deck_without_slides_returns_no_slide() {
    let mut s = session();
    let ops = s.deck().slides.iter().map(|slide| Op::RemoveSlide { slide: slide.id.clone() }).collect();
    s.apply("vide", ops, None).unwrap();
    assert_that!(refusal(&s)).is_equal_to(Some(PresentRefusal::NoSlide));
}

#[test]
fn can_present_refusal_speaks_the_user_language() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    s.open_help("fr", None).unwrap();
    let e = s.can_present().unwrap_err();
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    let en = oku_slides_i18n::Localizer::new(Some("en"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr("session-present-help"));
    assert_that!(e.localize(&fr)).is_not_equal_to(en.tr("session-present-help"));
}
