//! Le travailleur dans un vrai sous-processus : bornes et relance.

use std::collections::BTreeMap;
use std::path::PathBuf;

use oku_slides_i18n::Localizer;
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::{Fonts, PackWidgets, RenderErrorKind, Renderer, base_registry};
use oku_slides_render_worker::RenderWorker;
use speculoos::prelude::*;

// `cfg(test)` : les règles des tests (`expect` permis) valent dans ce module partagé.
#[cfg(test)]
#[path = "../src/test_packs.rs"]
mod test_packs;

fn launch() -> RenderLaunch {
    RenderLaunch::Process { program: PathBuf::from(env!("CARGO_BIN_EXE_oku-slides-render-worker")) }
}

fn worker(timeout_ms: u64, memory_mib: u64) -> RenderWorker {
    RenderWorker::new(launch(), RenderSettings { timeout_ms, memory_mib, max_frame_mib: 256 })
}

#[cfg(test)]
fn roadmap() -> oku_slides_core::Deck {
    oku_slides_core::Deck::from_json(include_str!("../../../fixtures/roadmap.json")).unwrap()
}

#[test]
fn process_render_is_identical_to_in_memory_render() {
    let l = Localizer::new(Some("fr"));
    let mut local = Renderer::new(Fonts::default()).unwrap();
    local.set_language(&l).unwrap();
    let expected = local.render(&roadmap(), &BTreeMap::new()).unwrap();
    let w = worker(30_000, 4096);
    let mut s = w.slot(&Fonts::default(), &l, base_registry(&Fonts::default()).unwrap()).unwrap();
    let got = s.render(&roadmap(), &BTreeMap::new()).unwrap();
    assert_that!(got.page_count()).is_equal_to(expected.page_count());
    for i in 0..expected.page_count() {
        assert_that!(got.svg(i).unwrap()).is_equal_to(expected.svg(i).unwrap());
    }
    assert_that!(serde_json::to_value(got.layout()).unwrap())
        .is_equal_to(serde_json::to_value(expected.layout()).unwrap());
}

#[test]
fn process_probe_that_loops_times_out() {
    let w = worker(500, 4096);
    let err = w.probe(&PackWidgets::new("boucle", test_packs::LOOP_ON_IMPORT).unwrap()).unwrap_err();
    assert_that!(err.render_error().map(|e| e.kind())).is_equal_to(Some(RenderErrorKind::Timeout));
}

#[test]
fn process_probe_that_allocates_is_stopped_out_of_memory() {
    let w = worker(60_000, 300);
    let err = w.probe(&PackWidgets::new("gros", test_packs::ALLOCATE).unwrap()).unwrap_err();
    assert_that!(err.render_error().map(|e| e.kind())).is_equal_to(Some(RenderErrorKind::OutOfMemory));
}

#[test]
fn process_killed_at_rest_is_restarted_without_error() {
    let w = worker(30_000, 4096);
    let l = Localizer::new(Some("fr"));
    let mut s = w.slot(&Fonts::default(), &l, base_registry(&Fonts::default()).unwrap()).unwrap();
    s.render(&roadmap(), &BTreeMap::new()).unwrap();
    w.kill_for_test();
    // Mort au repos : la requête suivante relance un travailleur, sans erreur (t-340).
    assert_that!(s.render(&roadmap(), &BTreeMap::new()).map(|c| c.page_count()).unwrap()).is_greater_than(0);
}

#[test]
fn process_killed_at_rest_does_not_blame_the_slot_pack() {
    let w = worker(30_000, 4096);
    let l = Localizer::new(Some("fr"));
    let pack = PackWidgets::new("vide", "#let registry = ()").unwrap();
    let registry = w.probe(&pack).unwrap();
    let mut s = w.slot(&Fonts::default(), &l, base_registry(&Fonts::default()).unwrap()).unwrap();
    s.set_widgets(Some(&pack), registry).unwrap();
    s.render(&roadmap(), &BTreeMap::new()).unwrap();
    w.kill_for_test();
    s.render(&roadmap(), &BTreeMap::new()).unwrap();
    assert_that!(w.is_faulty(&pack)).is_false();
}
