use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use oku_slides_i18n::Localize;
use oku_slides_import::report::ReferenceCause;
use oku_slides_reference::{Raster, SlideSpan};
use oku_slides_render::RenderError;
use speculoos::prelude::*;

use super::*;
use crate::Platform;
use crate::import::reference::span_len;
use crate::import::{
    Charge, ImportRequest, ImportTarget, Imported, PackLookup, WitnessPages, WitnessRenderer, import, render,
};
use crate::test_support::{fresh_dir, logged, pptx_fixture, settings_on, white_png};
use oku_slides_import::plan::ItemTarget;

/// Un témoin de test : des pages blanches, ou l'échec `fail` ; compte ses appels.
struct Fake {
    calls: AtomicUsize,
    /// Hauteur des pages pour une largeur de 1 pixel (celle de la page source).
    ratio: f64,
    fail: Option<fn() -> RenderError>,
    /// Pages rendues en moins que la tranche.
    missing: usize,
    /// Les limites du témoin imité.
    limits: Witness,
    /// L'attente de chaque rendu : un témoin lent, pour des appels concurrents.
    delay: Duration,
}

impl Fake {
    fn white(ratio: f64) -> Self {
        let calls = AtomicUsize::new(0);
        Self { calls, ratio, fail: None, missing: 0, limits: Witness::Office2Pdf, delay: Duration::ZERO }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl WitnessRenderer for Fake {
    fn name(&self) -> &str {
        "témoin"
    }

    fn limits(&self) -> Witness {
        self.limits
    }

    fn render(&self, _: &[u8], _: &Fonts, span: SlideSpan, width: u32) -> Result<WitnessPages, RenderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(self.delay);
        if let Some(fail) = self.fail {
            return Err(fail());
        }
        let count = span_len(span) - self.missing;
        let height = (f64::from(width) * self.ratio).round() as u32;
        let pages = (0..count).map(|_| white_png(width, height)).collect();
        Ok(WitnessPages { witness: "témoin 1.0".to_owned(), count, pages })
    }
}

/// L'import hi-fi de la fixture `minimal` et sa plateforme.
fn imported(data: &std::path::Path) -> (Platform, Imported) {
    let platform = Platform::with_settings(settings_on(data)).unwrap();
    let request = ImportRequest {
        pptx: pptx_fixture(data, "minimal"),
        target: crate::test_support::hifi(),
        system_fonts: false,
        packs: PackLookup::Library,
    };
    let imported = import(&platform, &Fonts::default(), &request).unwrap();
    (platform, imported)
}

fn ratio(state: &ImportState) -> f64 {
    state.source().size.h / state.source().size.w
}

/// Fait la référence de `state` par `fake` s'il en reste à faire, puis le nom du témoin ou la cause de son échec.
fn made(state: &mut ImportState, fake: &Fake) -> Option<Result<String, ReferenceCause>> {
    if let Some(job) = state.reference_job() {
        job.run_with(fake);
    }
    state.settle();
    state.reference_status().map(|r| r.map(str::to_owned))
}

#[test]
fn reference_is_rendered_once_and_kept() {
    let data = fresh_dir("oku-slides-import-state-once-");
    let (_, Imported { mut state, .. }) = imported(data.path());
    let fake = Fake::white(ratio(&state));
    assert_that!(made(&mut state, &fake)).is_equal_to(Some(Ok("témoin 1.0".to_owned())));
    assert_that!(made(&mut state, &fake)).is_equal_to(Some(Ok("témoin 1.0".to_owned())));
    assert_that!(fake.calls()).is_equal_to(1);
    let first = state.report().slides[0].slide.clone();
    assert_that!(state.reference_page(&first).is_some()).is_true();
}

#[test]
fn reference_failure_is_kept_and_not_retried() {
    let data = fresh_dir("oku-slides-import-state-failure-");
    let (_, Imported { mut state, .. }) = imported(data.path());
    let fake = Fake { fail: Some(|| RenderError::timeout(10)), ..Fake::white(ratio(&state)) };
    assert_that!(made(&mut state, &fake)).is_equal_to(Some(Err(ReferenceCause::Timeout)));
    assert_that!(made(&mut state, &fake)).is_equal_to(Some(Err(ReferenceCause::Timeout)));
    assert_that!(fake.calls()).is_equal_to(1);
}

#[test]
fn reference_failure_logs_its_cause_in_the_app_language() {
    let data = fresh_dir("oku-slides-import-state-failure-log-");
    let (_, Imported { mut state, .. }) = imported(data.path());
    let fake = Fake { fail: Some(|| RenderError::timeout(10)), ..Fake::white(ratio(&state)) };
    let log = logged(|| assert_that!(made(&mut state, &fake).is_some_and(|r| r.is_err())).is_true());
    let fr = ReferenceCause::Timeout.localize(&Localizer::new(Some("fr")));
    assert_that!(fr).is_not_equal_to(ReferenceCause::Timeout.localize(&Localizer::new(Some("en"))));
    assert_that!(log).contains(fr.as_str());
}

#[test]
fn reference_with_fewer_pages_than_slides_names_both_counts() {
    let data = fresh_dir("oku-slides-import-state-pages-");
    let (_, Imported { mut state, .. }) = imported(data.path());
    let fake = Fake { missing: 1, ..Fake::white(ratio(&state)) };
    assert_that!(made(&mut state, &fake)).is_equal_to(Some(Err(ReferenceCause::Pages { reference: 2, deck: 3 })));
}

#[test]
fn score_deck_scores_every_slide_and_names_the_witness() {
    let data = fresh_dir("oku-slides-import-state-deck-");
    let (platform, Imported { sources, widgets, mut state, .. }) = imported(data.path());
    let doc = render(&platform, state.fonts(), &widgets, &sources).unwrap();
    let fake = Fake::white(ratio(&state));
    made(&mut state, &fake);
    state.score_deck(doc.compiled());
    assert_that!(state.report().slides.iter().all(|s| s.score.is_some())).is_true();
    let reference = state.report().reference.clone().unwrap();
    assert_that!((reference.witness.as_str(), reference.unavailable)).is_equal_to(("témoin 1.0", None));
    let first = state.report().slides[0].slide.clone();
    assert_that!(state.scored_page(&first)).is_some();
}

#[test]
fn score_deck_after_a_witness_failure_leaves_no_score_and_says_why() {
    let data = fresh_dir("oku-slides-import-state-deck-failure-");
    let (platform, Imported { sources, widgets, mut state, .. }) = imported(data.path());
    let doc = render(&platform, state.fonts(), &widgets, &sources).unwrap();
    let fake = Fake { fail: Some(|| RenderError::out_of_memory(2048)), ..Fake::white(ratio(&state)) };
    made(&mut state, &fake);
    state.score_deck(doc.compiled());
    assert_that!(state.report().slides.iter().all(|s| s.score.is_none())).is_true();
    let reference = state.report().reference.clone().unwrap();
    assert_that!((reference.witness.as_str(), reference.unavailable))
        .is_equal_to(("témoin", Some(ReferenceCause::Memory)));
}

#[test]
fn witness_missing_leaves_no_score_and_names_libreoffice() {
    let data = fresh_dir("oku-slides-import-state-missing-");
    let (_, Imported { mut state, .. }) = imported(data.path());
    state.witness_missing("--reference soffice".to_owned());
    assert_that!(state.report().slides.iter().all(|s| s.score.is_none())).is_true();
    let reference = state.report().reference.clone().unwrap();
    assert_that!((reference.witness.as_str(), reference.unavailable))
        .is_equal_to((SOFFICE_WITNESS, Some(ReferenceCause::Missing)));
    assert_that!(state.reference_job()).is_none();
    let fake = Fake::white(ratio(&state));
    assert_that!(made(&mut state, &fake)).is_equal_to(Some(Err(ReferenceCause::Missing)));
}

#[test]
fn witness_refused_leaves_no_score_and_says_libreoffice_failed() {
    let data = fresh_dir("oku-slides-import-state-refused-");
    let (_, Imported { mut state, .. }) = imported(data.path());
    state.witness_refused("soffice : un script".to_owned());
    assert_that!(state.report().slides.iter().all(|s| s.score.is_none())).is_true();
    let reference = state.report().reference.clone().unwrap();
    assert_that!((reference.witness.as_str(), reference.unavailable))
        .is_equal_to((SOFFICE_WITNESS, Some(ReferenceCause::Failed)));
    assert_that!(state.reference_job()).is_none();
}

#[test]
fn regions_are_the_source_items_on_the_oku_page() {
    let data = fresh_dir("oku-slides-import-state-regions-");
    let (_, Imported { state, .. }) = imported(data.path());
    let report = &state.report().slides[0];
    let src = state.source().slides.iter().find(|s| s.number == report.source).unwrap();
    let rect = hifi::source_rect(state.source());
    let k = rect.w / state.source().size.w;
    let expected: Vec<Frame> = metrics::regions(src, 1.0, Some(Witness::Office2Pdf))
        .iter()
        .map(|r| Frame { x: rect.x + r.x * k, y: rect.y + r.y * k, w: r.w * k, h: r.h * k })
        .collect();
    let regions = state.regions(&report.slide);
    assert_that!(regions.iter().map(|r| r.frame).collect::<Vec<_>>()).is_equal_to(expected);
    assert_that!(regions).is_not_empty();
}

#[test]
fn regions_carry_the_elements_projected_from_their_item() {
    let data = fresh_dir("oku-slides-import-state-elements-");
    let (_, Imported { sources, state, .. }) = imported(data.path());
    let slots = sources.deck().slides.iter().flat_map(|s| s.params.slots().flat_map(|(_, list)| list.to_vec()));
    let ids: Vec<String> = slots.map(|e| e.id).collect();
    let all: Vec<SlideRegion> = state.report().slides.iter().flat_map(|r| state.regions(&r.slide)).collect();
    let known: Vec<String> = all
        .iter()
        .filter_map(|r| match &r.target {
            Some(ItemTarget::Elements(ids)) => Some(ids.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    assert_that!(known).is_not_empty();
    assert_that!(known.iter().all(|e| ids.contains(e))).is_true();
}

#[test]
fn regions_of_minimal_all_have_a_target() {
    let data = fresh_dir("oku-slides-import-state-targets-");
    let (_, Imported { state, .. }) = imported(data.path());
    let all: Vec<SlideRegion> = state.report().slides.iter().flat_map(|r| state.regions(&r.slide)).collect();
    let missing: Vec<&str> = all.iter().filter(|r| r.target.is_none()).map(|r| r.item.as_str()).collect();
    assert_that!(missing).is_empty();
}

#[test]
fn reference_job_is_offered_until_done_then_none() {
    let data = fresh_dir("oku-slides-import-state-job-");
    let (_, Imported { state, .. }) = imported(data.path());
    let fake = Fake::white(ratio(&state));
    let (first, second) = (state.reference_job().unwrap(), state.reference_job().unwrap());
    first.run_with(&fake);
    second.run_with(&fake);
    assert_that!(state.reference_job()).is_none();
    assert_that!(state.reference_status()).is_equal_to(Some(Ok("témoin 1.0")));
    assert_that!(fake.calls()).is_equal_to(1);
}

#[test]
fn reference_jobs_run_concurrently_render_the_witness_once() {
    let data = fresh_dir("oku-slides-import-state-concurrent-");
    let (_, Imported { mut state, .. }) = imported(data.path());
    let fake = Fake { delay: Duration::from_millis(200), ..Fake::white(ratio(&state)) };
    let jobs = [state.reference_job().unwrap(), state.reference_job().unwrap()];
    std::thread::scope(|scope| {
        for job in &jobs {
            scope.spawn(|| job.run_with(&fake));
        }
    });
    assert_that!(fake.calls()).is_equal_to(1);
    assert_that!(made(&mut state, &fake)).is_equal_to(Some(Ok("témoin 1.0".to_owned())));
}

#[test]
fn reference_job_of_another_import_leaves_this_one_without_reference() {
    let data = fresh_dir("oku-slides-import-state-identity-");
    let (_, Imported { state: this, .. }) = imported(data.path());
    let (_, Imported { state: other, .. }) = imported(data.path());
    other.reference_job().unwrap().run_with(&Fake::white(ratio(&other)));
    assert_that!(other.reference_status()).is_equal_to(Some(Ok("témoin 1.0")));
    assert_that!(this.reference_status()).is_none();
    assert_that!(this.reference_job()).is_some();
}

#[test]
fn settle_frees_the_presentation_once_the_reference_is_made() {
    let data = fresh_dir("oku-slides-import-state-bytes-");
    let (_, Imported { mut state, .. }) = imported(data.path());
    assert_that!(state.bytes.is_some()).is_true();
    let fake = Fake::white(ratio(&state));
    made(&mut state, &fake);
    assert_that!(state.bytes.is_none()).is_true();
}

#[test]
fn set_witness_none_offers_no_job_and_frees_the_presentation() {
    let data = fresh_dir("oku-slides-import-state-no-witness-");
    let (_, Imported { mut state, .. }) = imported(data.path());
    state.set_witness(None);
    assert_that!(state.reference_job()).is_none();
    assert_that!(state.bytes.is_none()).is_true();
}

#[test]
fn retrofit_import_keeps_neither_witness_nor_presentation() {
    let data = fresh_dir("oku-slides-import-state-retrofit-");
    let platform = Platform::with_settings(settings_on(data.path())).unwrap();
    let target = ImportTarget::Retrofit { layouts: Vec::new(), theme: None };
    let request = ImportRequest {
        pptx: pptx_fixture(data.path(), "minimal"),
        target,
        system_fonts: false,
        packs: PackLookup::Library,
    };
    let state = import(&platform, &Fonts::default(), &request).unwrap().state;
    assert_that!((state.witness.is_none(), state.bytes.is_none())).is_equal_to((true, true));
    assert_that!(state.reference_job()).is_none();
}

#[test]
fn regions_of_an_unknown_slide_are_empty() {
    let data = fresh_dir("oku-slides-import-state-unknown-");
    let (_, Imported { state, .. }) = imported(data.path());
    assert_that!(state.regions("nope")).is_empty();
}

#[test]
fn observe_scores_a_slide_once_then_keeps_an_unchanged_raster_free() {
    let data = fresh_dir("oku-slides-import-state-observe-");
    let (_, Imported { mut state, .. }) = imported(data.path());
    let fake = Fake::white(ratio(&state));
    made(&mut state, &fake);
    let slide = state.report().slides[0].slide.clone();
    assert_that!(state.tuning(&slide)).is_none();
    let oku = Raster::from_png(&white_png(960, 540)).unwrap();
    let (charge, score) = state.observe(&slide, &oku, Some(7), (1, 1)).unwrap();
    assert_that!((charge, score.is_some())).is_equal_to((Charge::Baseline, true));
    assert_that!(state.scored_page(&slide)).is_equal_to(Some(7));
    let (charge, _) = state.observe(&slide, &oku, Some(8), (1, 1)).unwrap();
    assert_that!(charge).is_equal_to(Charge::Unchanged);
    assert_that!(state.tuning(&slide).map(|t| t.tries)).is_equal_to(Some(0));
}

/// Les limites des régions de toutes les slides de `state`.
fn suspects(state: &ImportState) -> Vec<Option<WitnessLimit>> {
    state.report().slides.iter().flat_map(|r| state.regions(&r.slide)).map(|r| r.suspect).collect()
}

/// L'import hi-fi de `fixtures/import/libreoffice.pptx` (un SmartArt, des graphiques).
fn imported_libreoffice(data: &std::path::Path) -> ImportState {
    let platform = Platform::with_settings(settings_on(data)).unwrap();
    let pptx = data.join("libreoffice.pptx");
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/import/libreoffice.pptx");
    std::fs::copy(fixture, &pptx).unwrap();
    let request =
        ImportRequest { pptx, target: crate::test_support::hifi(), system_fonts: false, packs: PackLookup::Library };
    import(&platform, &Fonts::default(), &request).unwrap().state
}

#[test]
fn regions_are_suspect_under_office2pdf() {
    let data = fresh_dir("oku-slides-import-state-office2pdf-");
    let mut state = imported_libreoffice(data.path());
    let fake = Fake::white(ratio(&state));
    made(&mut state, &fake);
    assert_that!(suspects(&state).iter().any(Option::is_some)).is_true();
}

#[test]
fn regions_are_suspect_under_libreoffice_for_its_own_limits() {
    let data = fresh_dir("oku-slides-import-state-libreoffice-");
    let mut state = imported_libreoffice(data.path());
    let fake = Fake { limits: Witness::LibreOffice, ..Fake::white(ratio(&state)) };
    made(&mut state, &fake);
    let limits = suspects(&state);
    // Ses titres en Calibri Light et son SmartArt ; jamais son graphique, que LibreOffice dessine selon l'OOXML.
    assert_that!(limits.contains(&Some(WitnessLimit::NoSubstitute))).is_true();
    assert_that!(limits.contains(&Some(WitnessLimit::SmartArtStyle))).is_true();
    assert_that!(limits.contains(&Some(WitnessLimit::Chart))).is_false();
}

#[test]
fn regions_are_never_suspect_when_the_soffice_witness_is_missing() {
    let data = fresh_dir("oku-slides-import-state-missing-");
    let mut state = imported_libreoffice(data.path());
    state.witness_missing("test".to_owned());
    assert_that!(suspects(&state).iter().all(Option::is_none)).is_true();
}
