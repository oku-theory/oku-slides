use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_i18n::Localizer;
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::{Fonts, PackWidgets, RenderErrorKind, base_registry};
use speculoos::prelude::*;

use super::*;
use crate::test_packs::{LOOP_ON_IMPORT, LOOP_ON_RENDER, tiny_deck};

fn worker(timeout_ms: u64) -> RenderWorker {
    RenderWorker::new(RenderLaunch::InProcess, RenderSettings { timeout_ms, memory_mib: 4096, max_frame_mib: 64 })
}

fn slot(w: &RenderWorker) -> RenderSlot {
    w.slot(&Fonts::default(), &Localizer::new(Some("fr")), base_registry(&Fonts::default()).unwrap()).unwrap()
}

#[test]
fn handles_are_send_and_worker_is_sync() {
    fn send<T: Send>() {}
    fn sync<T: Sync>() {}
    send::<RenderWorker>();
    sync::<RenderWorker>();
    send::<RenderSlot>();
    send::<RemoteCompiled>();
}

#[test]
fn slot_render_then_svg_returns_svg() {
    let w = worker(10_000);
    let mut s = slot(&w);
    let c = s.render(&tiny_deck(), &BTreeMap::new()).unwrap();
    assert_that!(c.svg(0).unwrap()).starts_with("<svg");
}

#[test]
fn probe_is_cached_by_fingerprint() {
    let w = worker(10_000);
    let pack = PackWidgets::new("vide", "#let registry = ()").unwrap();
    assert_that!(Arc::ptr_eq(&w.probe(&pack).unwrap(), &w.probe(&pack).unwrap())).is_true();
}

#[test]
fn probe_that_loops_marks_pack_faulty() {
    let w = worker(300);
    let pack = PackWidgets::new("boucle", LOOP_ON_IMPORT).unwrap();
    let err = w.probe(&pack).unwrap_err();
    assert_that!(err.render_error().map(|e| e.kind())).is_equal_to(Some(RenderErrorKind::Timeout));
    assert_that!(w.is_faulty(&pack)).is_true();
}

#[test]
fn render_with_looping_widget_times_out_then_is_disabled_without_retry() {
    let w = worker(300);
    let pack = PackWidgets::new("rendu", LOOP_ON_RENDER).unwrap();
    let registry = w.probe(&pack).unwrap();
    let mut s = slot(&w);
    s.set_widgets(Some(&pack), registry).unwrap();
    let deck = oku_slides_core::Deck::from_json(r#"{"version":2,"params":{"title":"T"},"slides":[{"id":"s","kind":"content","params":{"blocks":[{"id":"b","kind":"boucle","params":{"body":"x"}}]}}]}"#).unwrap();
    assert_that!(s.render(&deck, &BTreeMap::new()).unwrap_err().kind()).is_equal_to(RenderErrorKind::Timeout);
    let started = std::time::Instant::now();
    assert_that!(s.render(&deck, &BTreeMap::new()).unwrap_err().kind()).is_equal_to(RenderErrorKind::PackDisabled);
    assert_that!(started.elapsed()).is_less_than(std::time::Duration::from_millis(100));
}

#[test]
fn dropping_remote_compiled_releases_its_generation() {
    let w = worker(10_000);
    let mut s = slot(&w);
    let first = s.render(&tiny_deck(), &BTreeMap::new()).unwrap();
    let generation = first.generation;
    drop(first);
    let second = s.render(&tiny_deck(), &BTreeMap::new()).unwrap();
    // La génération libérée n'est plus servie.
    let stale = RemoteCompiled { generation, ..second.clone_handle_for_test() };
    assert_that!(stale.svg(0).unwrap_err().kind()).is_equal_to(RenderErrorKind::Stale);
}

/// Relance le travailleur `InProcess` (un fil ne se tue pas) : une sonde qui
/// boucle dépasse le délai et abandonne la connexion.
fn force_restart(w: &RenderWorker, id: &str) {
    let pack = PackWidgets::new(id, LOOP_ON_IMPORT).unwrap();
    assert_that!(w.probe(&pack).unwrap_err().render_error().map(|e| e.kind()))
        .is_equal_to(Some(RenderErrorKind::Timeout));
}

#[test]
fn remote_compiled_of_previous_worker_is_stale_and_its_release_is_dropped() {
    let w = worker(3_000);
    let mut s = slot(&w);
    let first = s.render(&tiny_deck(), &BTreeMap::new()).unwrap();
    let old = first.clone_handle_for_test();
    force_restart(&w, "boucle-a");
    // Le travailleur neuf repart de la même génération que `first`.
    let second = s.render(&tiny_deck(), &BTreeMap::new()).unwrap();
    assert_that!(second.generation).is_equal_to(old.generation);
    drop(first);
    // Le `Release` de `first` ne libère pas `second`.
    assert_that!(second.svg(0).unwrap()).starts_with("<svg");
    force_restart(&w, "boucle-b");
    assert_that!(old.svg(0).unwrap_err().kind()).is_equal_to(RenderErrorKind::Stale);
    // Sans appel : le lien abandonné n'a pas été relancé.
    assert_that!(w.link().connected()).is_false();
}

#[test]
fn set_widgets_with_faulty_pack_is_disabled_without_call() {
    let w = worker(3_000);
    let pack = PackWidgets::new("boucle", LOOP_ON_IMPORT).unwrap();
    force_restart(&w, "boucle");
    let mut s = slot(&w);
    force_restart(&w, "boucle-b");
    let base = base_registry(&Fonts::default()).unwrap();
    assert_that!(s.set_widgets(Some(&pack), base).unwrap_err().kind()).is_equal_to(RenderErrorKind::PackDisabled);
    // Sans appel : le lien abandonné n'a pas été relancé.
    assert_that!(w.link().connected()).is_false();
}

#[test]
fn dropped_slot_is_forgotten_with_its_assets() {
    let w = worker(10_000);
    let mut s = slot(&w);
    let id = s.id;
    let asset = "00ff".to_owned();
    drop(s.render(&tiny_deck(), &BTreeMap::from([(asset.clone(), vec![1, 2, 3])])).unwrap());
    assert_that!(w.link().sent_assets.contains(&asset)).is_true();
    drop(s);
    // Une requête suivante envoie `CloseSlot`.
    let _other = slot(&w);
    let link = w.link();
    assert_that!(link.state.slots.contains_key(&id)).is_false();
    assert_that!(link.sent_assets.contains(&asset)).is_false();
}
