//! Démarrage paresseux du rendu du deck (t-342).

use speculoos::prelude::*;

use crate::lib_tests_support::{data_dir, fixture_path};
use crate::{Fonts, Platform, Session};

fn platform(data: &std::path::Path) -> Platform {
    Platform::with_settings(crate::test_support::settings_on(data)).unwrap()
}

#[test]
fn unrendered_session_launches_no_worker_until_its_first_render() {
    let data = data_dir();
    let mut s = Session::unrendered(Fonts::default(), &platform(&data.0)).unwrap();
    assert_that!(s.worker.is_started()).is_false();
    assert_that!(s.compiled.is_none()).is_true();
    s.ensure_rendered().unwrap();
    assert_that!(s.worker.is_started()).is_true();
    assert_that!(s.compiled.is_some()).is_true();
}

#[test]
fn unrendered_session_opens_a_file_without_rendering_the_blank_deck() {
    let data = data_dir();
    let mut s = Session::unrendered(Fonts::default(), &platform(&data.0)).unwrap();
    s.open(&fixture_path()).unwrap();
    let rendered = s.compiled.as_ref().map(|c| c.layout().slides.iter().map(|p| p.slide.clone()).collect::<Vec<_>>());
    let slides = s.deck().slides.iter().map(|sl| sl.id.clone()).collect::<Vec<_>>();
    assert_that!(rendered).is_equal_to(Some(slides));
}

#[test]
fn new_session_is_rendered() {
    let data = data_dir();
    let s = Session::new(Fonts::default(), &platform(&data.0)).unwrap();
    assert_that!(s.compiled.is_some()).is_true();
}
