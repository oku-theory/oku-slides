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
    RenderWorker::new(
        launch(),
        RenderSettings {
            timeout_ms,
            probe_budget_ms: timeout_ms,
            memory_mib,
            max_frame_mib: 256,
            max_widget_kib: 256,
            max_widgets: 64,
        },
    )
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
    // La maquette est en `papier` : le même thème est posé au travailleur par `SetTheme`.
    let papier = include_str!("../../../typst/oku-slides/themes/papier.json");
    let papier: serde_json::Value = serde_json::from_str(papier).unwrap();
    let tokens = papier["tokens"].as_object().cloned();
    local.set_theme(tokens.clone()).unwrap();
    let expected = local.render(&roadmap(), &BTreeMap::new()).unwrap();
    let w = worker(30_000, 4096);
    let mut s = w.slot(&Fonts::default(), &l, base_registry(&Fonts::default()).unwrap()).unwrap();
    s.set_theme(tokens.as_ref()).unwrap();
    let got = s.render(&roadmap(), &BTreeMap::new()).unwrap();
    assert_that!(got.page_count()).is_equal_to(expected.page_count());
    for i in 0..expected.page_count() {
        assert_that!(got.svg(i).unwrap()).is_equal_to(expected.svg(i).unwrap());
    }
    assert_that!(serde_json::to_value(got.layout()).unwrap())
        .is_equal_to(serde_json::to_value(expected.layout()).unwrap());
}

/// `SetWidgets` du pack `id` au seul fichier `name` : l'erreur du travailleur.
fn set_widgets_error(w: &RenderWorker, id: &str, name: &str, source: &str) -> RenderErrorKind {
    let l = Localizer::new(Some("fr"));
    let base = || base_registry(&Fonts::default()).unwrap_or_else(|e| panic!("{e}"));
    let mut s = w.slot(&Fonts::default(), &l, base()).unwrap_or_else(|e| panic!("{e}"));
    let pack = PackWidgets::new(id, test_packs::files(&[(name, source)])).unwrap_or_else(|e| panic!("{e}"));
    s.set_widgets(Some(&pack), base()).map_or_else(|e| e.kind(), |()| panic!("SetWidgets a réussi"))
}

#[test]
fn process_set_widgets_that_loops_times_out() {
    // Le délai borne aussi le lancement du processus et l'ouverture de
    // l'emplacement : 500 ms n'y suffisent pas toujours. La boucle dure bien plus.
    let w = worker(3_000, 4096);
    let kind = set_widgets_error(&w, "boucle", "boucle", test_packs::LOOP_ON_IMPORT);
    assert_that!(kind).is_equal_to(RenderErrorKind::Timeout);
}

#[test]
fn process_set_widgets_that_allocates_is_stopped_out_of_memory() {
    let w = worker(60_000, 300);
    let kind = set_widgets_error(&w, "gros", "gros", test_packs::ALLOCATE);
    assert_that!(kind).is_equal_to(RenderErrorKind::OutOfMemory);
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
    let pack = PackWidgets::new("vide", BTreeMap::new()).unwrap();
    let registry = w.probe_pack(&pack).unwrap().registry;
    let mut s = w.slot(&Fonts::default(), &l, base_registry(&Fonts::default()).unwrap()).unwrap();
    s.set_widgets(Some(&pack), registry).unwrap();
    s.render(&roadmap(), &BTreeMap::new()).unwrap();
    w.kill_for_test();
    s.render(&roadmap(), &BTreeMap::new()).unwrap();
    assert_that!(w.is_faulty(&pack)).is_false();
}

#[test]
fn process_probe_pack_with_a_file_that_allocates_gives_a_diagnostic_and_accuses_nothing() {
    let w = worker(60_000, 300);
    let pack = PackWidgets::new("gros", test_packs::files(&[("gros", test_packs::ALLOCATE)])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    let gros = oku_slides_core::WidgetFile::parse("gros").unwrap();
    assert_that!(probe.files[&gros].is_valid()).is_false();
    assert_that!(probe.pack).is_none();
    assert_that!(w.is_faulty(&pack)).is_false();
}
