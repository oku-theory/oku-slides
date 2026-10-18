use oku_slides_core::Op;
use oku_slides_reference::Raster;
use speculoos::prelude::*;

use super::super::{ImportTarget, TuneErrorKind};
use super::{MARGIN, ZOOM};
use crate::Request;
use crate::test_support::{WhiteWitness, fresh_dir, imported_fixture, witnessed};

/// Le hi-fi de `kitchen` sur une référence blanche, à `tries` essais par slide, et sa
/// première slide `pending` de la file.
fn pending(data: &std::path::Path, tries: u32) -> (crate::Session, String) {
    let mut s = imported_fixture(data, "kitchen", crate::test_support::hifi(), |settings| {
        settings.import.tune_iterations = std::num::NonZeroU32::new(tries).unwrap();
    });
    witnessed(&mut s, &WhiteWitness::default());
    let report = s.import_report(&crate::Localizer::new(Some("en"))).unwrap();
    let slides = report["slides"].as_array().unwrap();
    let pending = slides.iter().find(|e| e["state"] == "pending").unwrap();
    (s, pending["slide"].as_str().unwrap().to_owned())
}

/// Retire le `n`-ième élément des slots de `slide` : sa page change.
fn change(s: &mut crate::Session, slide: &str, n: usize) {
    let slide_ref = s.deck().slide(slide).unwrap();
    let element = slide_ref.params.slots().flat_map(|(_, list)| list.iter().map(|e| e.id.clone())).nth(n).unwrap();
    let ops = vec![Op::RemoveElement { slide: slide.to_owned(), element }];
    s.handle(Request::Apply { label: "Retirer".into(), ops, merge: None }).unwrap();
}

#[test]
fn import_diff_of_a_whole_slide_gives_both_pages_at_960_px_and_its_regions() {
    let data = fresh_dir("oku-slides-import-diff-whole-");
    let (mut s, slide) = pending(data.path(), 3);
    let diff = s.import_diff(&slide, None).unwrap();
    for png in [&diff.reference, &diff.oku] {
        let r = Raster::from_png(png).unwrap();
        assert_that!((r.width(), r.height())).is_equal_to((960, 540));
    }
    let region = &diff.json["regions"][0];
    assert_that!(region["item"].is_string()).is_true();
    assert_that!(region["frame"]["w"].as_f64().unwrap()).is_greater_than(0.0);
    assert_that!(region["elements"].is_array() || region["param"].is_string()).is_true();
    assert_that!(diff.json["regions"].as_array().unwrap().len()).is_less_than_or_equal_to(super::LISTED);
}

#[test]
fn import_diff_first_look_and_unchanged_page_count_no_try() {
    let data = fresh_dir("oku-slides-import-diff-free-");
    let (mut s, slide) = pending(data.path(), 3);
    for _ in 0..2 {
        let diff = s.import_diff(&slide, None).unwrap();
        assert_that!(diff.json["counted"].as_bool()).is_equal_to(Some(false));
        assert_that!(diff.json["tries_left"].as_u64()).is_equal_to(Some(3));
    }
}

#[test]
fn import_diff_after_a_change_counts_a_try_and_is_refused_once_none_is_left() {
    let data = fresh_dir("oku-slides-import-diff-tries-");
    let (mut s, slide) = pending(data.path(), 1);
    s.import_diff(&slide, None).unwrap();
    change(&mut s, &slide, 0);
    let diff = s.import_diff(&slide, None).unwrap();
    assert_that!(diff.json["counted"].as_bool()).is_equal_to(Some(true));
    assert_that!(diff.json["tries_left"].as_u64()).is_equal_to(Some(0));
    let e = s.import_diff(&slide, None).unwrap_err();
    assert_that!(e.tune_kind()).is_equal_to(Some(TuneErrorKind::TriesUsed));
    // Sans essai, le zoom est refusé aussi.
    let item = diff.json["regions"][0]["item"].as_str().unwrap().to_owned();
    let e = s.import_diff(&slide, Some(&item)).unwrap_err();
    assert_that!(e.tune_kind()).is_equal_to(Some(TuneErrorKind::TriesUsed));
}

#[test]
fn import_diff_first_whole_diff_after_a_report_counts_a_changed_page() {
    let data = fresh_dir("oku-slides-import-diff-after-report-");
    let (mut s, slide) = pending(data.path(), 3);
    // Le rapport a noté la slide : c'est sa base ; le premier diff d'une page changée depuis est un essai.
    change(&mut s, &slide, 0);
    let diff = s.import_diff(&slide, None).unwrap();
    assert_that!(diff.json["counted"].as_bool()).is_equal_to(Some(true));
    assert_that!(diff.json["charge"].as_str()).is_equal_to(Some("slide"));
    assert_that!(diff.json["tries_left"].as_u64()).is_equal_to(Some(2));
}

#[test]
fn import_diff_of_a_slide_whose_neighbour_changed_counts_no_try() {
    // Sur alto, retoucher s6 changeait l'empreinte Typst de la page de s7 (introspection), pas son raster.
    let data = fresh_dir("oku-slides-import-diff-neighbour-");
    let mut s = imported_fixture(data.path(), "alto-wealth", crate::test_support::hifi(), |_| {});
    let witness = WhiteWitness::default();
    witnessed(&mut s, &witness);
    s.import_report(&crate::Localizer::new(Some("en"))).unwrap();
    let element = |s: &crate::Session, slide: &str| {
        let slide = s.deck().slide(slide).unwrap();
        slide.params.slots().flat_map(|(_, list)| list.iter()).find(|e| e.frame.is_some()).cloned().unwrap()
    };
    let moved = element(&s, "s6");
    let mut frame = moved.frame.unwrap();
    frame.x += 12.0;
    let ops = vec![Op::SetFrame { slide: "s6".into(), element: moved.id, frame: Some(frame) }];
    s.handle(Request::Apply { label: "Déplacer".into(), ops, merge: None }).unwrap();
    let diff = s.import_diff("s7", None).unwrap();
    assert_that!(diff.json["counted"].as_bool()).is_equal_to(Some(false));
    assert_that!(diff.json["charge"].as_str()).is_equal_to(Some("unchanged"));
    assert_that!(diff.json["tries_left"].as_u64()).is_equal_to(Some(3));
}

#[test]
fn import_diff_first_look_at_a_slide_never_scored_is_its_free_baseline() {
    let data = fresh_dir("oku-slides-import-diff-first-");
    let mut s = imported_fixture(data.path(), "kitchen", crate::test_support::hifi(), |_| {});
    let slide = s.deck().slides[0].id.clone();
    witnessed(&mut s, &WhiteWitness::default());
    let diff = s.import_diff(&slide, None).unwrap();
    assert_that!(diff.json["charge"].as_str()).is_equal_to(Some("baseline"));
    assert_that!(diff.json["counted"].as_bool()).is_equal_to(Some(false));
}

#[test]
fn import_diff_of_a_slide_without_tries_is_refused_whole_and_zoomed() {
    let data = fresh_dir("oku-slides-import-diff-done-");
    // Un seuil hors d'atteinte : toutes les slides sont `done`, sans essai.
    let mut s = imported_fixture(data.path(), "kitchen", crate::test_support::hifi(), |settings| {
        settings.import.score_threshold = 1000.0;
    });
    let witness = WhiteWitness::default();
    witnessed(&mut s, &witness);
    let report = s.import_report(&crate::Localizer::new(Some("en"))).unwrap();
    assert_that!(report["slides"].as_array().map(Vec::len)).is_equal_to(Some(0));
    let slide = s.deck().slides[0].id.clone();
    let item = s.import_state().unwrap().regions(&slide)[0].item.clone();
    // Notée par le rapport : sans essai, le diff entier et le zoom sont refusés.
    for region in [None, Some(item.as_str())] {
        let e = s.import_diff(&slide, region).unwrap_err();
        assert_that!(e.tune_kind()).is_equal_to(Some(TuneErrorKind::Untunable));
    }
}

#[test]
fn import_diff_of_a_region_crops_both_pages_around_it_at_twice_the_size() {
    let data = fresh_dir("oku-slides-import-diff-zoom-");
    let (mut s, slide) = pending(data.path(), 3);
    let whole = s.import_diff(&slide, None).unwrap();
    let item = whole.json["regions"][0]["item"].as_str().unwrap().to_owned();
    let zoom = s.import_diff(&slide, Some(&item)).unwrap();
    let [_, _, w, h] = [0, 1, 2, 3].map(|i| zoom.json["crop"][i].as_u64().unwrap() as u32);
    for png in [&zoom.reference, &zoom.oku] {
        let r = Raster::from_png(png).unwrap();
        assert_that!((r.width(), r.height())).is_equal_to((w * ZOOM, h * ZOOM));
    }
    let frame = &zoom.json["region"]["frame"];
    // Le cadre (pt, à 0,5 px par pt) plus la marge de chaque côté, borné à la page.
    let expected_w = ((frame["w"].as_f64().unwrap() * 0.5).round() as u32 + 2 * MARGIN).min(960);
    assert_that!(w).is_less_than_or_equal_to(expected_w);
    assert_that!(zoom.json["region"]["item"].as_str()).is_equal_to(Some(item.as_str()));
    // Le zoom ne compte pas d'essai.
    let again = s.import_diff(&slide, None).unwrap();
    assert_that!(again.json["tries_left"].as_u64()).is_equal_to(Some(3));
}

#[test]
fn import_diff_of_an_unknown_slide_or_region_is_refused() {
    let data = fresh_dir("oku-slides-import-diff-unknown-");
    let (mut s, slide) = pending(data.path(), 3);
    let e = s.import_diff("nope", None).unwrap_err();
    assert_that!(e.tune_kind()).is_equal_to(Some(TuneErrorKind::UnknownSlide));
    let e = s.import_diff(&slide, Some("nope")).unwrap_err();
    assert_that!(e.tune_kind()).is_equal_to(Some(TuneErrorKind::UnknownRegion));
}

#[test]
fn import_diff_of_a_retrofit_import_is_refused() {
    let data = fresh_dir("oku-slides-import-diff-retrofit-");
    let target = ImportTarget::Retrofit { pack: "Essentiel".into(), theme: None };
    let mut s = imported_fixture(data.path(), "minimal", target, |_| {});
    let slide = s.deck().slides[0].id.clone();
    let e = s.import_diff(&slide, None).unwrap_err();
    assert_that!(e.tune_kind()).is_equal_to(Some(TuneErrorKind::HifiOnly));
}

#[test]
fn import_diff_whose_comparison_fails_counts_no_try() {
    let data = fresh_dir("oku-slides-import-diff-failed-");
    let (mut s, slide) = pending(data.path(), 3);
    change(&mut s, &slide, 0);
    let state = s.import.as_mut().unwrap();
    let reference = std::sync::Arc::get_mut(&mut state.reference).unwrap().get_mut().unwrap();
    reference.pages.as_mut().unwrap().remove(&slide);
    let e = s.import_diff(&slide, None).unwrap_err();
    assert_that!(e.tune_kind()).is_equal_to(Some(TuneErrorKind::Compare));
    assert_that!(s.import_state().unwrap().tuning(&slide).map(|t| t.tries)).is_equal_to(Some(0));
}

#[test]
fn import_diff_before_the_reference_job_is_refused_as_pending() {
    let data = fresh_dir("oku-slides-import-diff-pending-");
    let mut s = imported_fixture(data.path(), "kitchen", crate::test_support::hifi(), |_| {});
    let slide = s.deck().slides[0].id.clone();
    let e = s.import_diff(&slide, None).unwrap_err();
    assert_that!(e.tune_kind()).is_equal_to(Some(TuneErrorKind::ReferencePending));
}
