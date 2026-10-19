//! Polices du travailleur : oubliées, ici et là-bas, quand plus aucun emplacement ne s'en sert.

use oku_slides_i18n::Localizer;
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::{Fonts, core_registry};
use sha2::{Digest, Sha256};
use speculoos::prelude::*;

use super::*;
use crate::protocol::{Request, Response};

fn worker() -> RenderWorker {
    RenderWorker::new(
        RenderLaunch::InProcess,
        RenderSettings {
            timeout_ms: 10_000,
            probe_budget_ms: 10_000,
            memory_mib: 4096,
            max_frame_mib: 64,
            max_widget_kib: 256,
            max_widgets: 64,
        },
    )
}

fn font(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/../../assets/fonts/{name}.ttf", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn id(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn slot_with(w: &RenderWorker, fonts: &Fonts) -> RenderSlot {
    w.slot(fonts, &Localizer::new(Some("fr")), core_registry(&Fonts::default()).unwrap()).unwrap()
}

/// Le travailleur connaît la police `id` : un emplacement s'y ouvre.
fn worker_knows(w: &RenderWorker, id: &str) -> bool {
    let mut link = w.link();
    match link.call(&Request::OpenSlot { slot: 9_999, fonts: vec![id.to_owned()] }, &[]).unwrap().0 {
        Response::Ok => true,
        Response::Error { .. } => false,
        other => panic!("{other:?}"),
    }
}

#[test]
fn fonts_of_a_dropped_slot_are_forgotten_by_host_and_worker() {
    let w = worker();
    let carlito = font("carlito-400");
    let s = slot_with(&w, &Fonts::new([carlito.clone()]));
    assert_that!(worker_knows(&w, &id(&carlito))).is_true();
    drop(s);
    // La requête suivante envoie `CloseSlot`, puis `DropFonts`.
    let _other = slot_with(&w, &Fonts::default());
    {
        let link = w.link();
        assert_that!(link.state.fonts.contains_key(&id(&carlito))).is_false();
        assert_that!(link.sent_fonts.contains(&id(&carlito))).is_false();
    }
    assert_that!(worker_knows(&w, &id(&carlito))).is_false();
}

#[test]
fn fonts_still_used_by_an_open_slot_are_kept() {
    let w = worker();
    let carlito = font("carlito-400");
    let caladea = font("caladea-400");
    let kept = slot_with(&w, &Fonts::new([carlito.clone()]));
    let dropped = slot_with(&w, &Fonts::new([carlito.clone(), caladea.clone()]));
    drop(dropped);
    let _other = slot_with(&w, &Fonts::default());
    {
        let link = w.link();
        assert_that!(link.state.fonts.contains_key(&id(&carlito))).is_true();
        assert_that!(link.state.fonts.contains_key(&id(&caladea))).is_false();
    }
    assert_that!(worker_knows(&w, &id(&carlito))).is_true();
    drop(kept);
}

#[test]
fn a_slot_reopened_with_forgotten_fonts_sends_them_again() {
    let w = worker();
    let carlito = font("carlito-400");
    drop(slot_with(&w, &Fonts::new([carlito.clone()])));
    let _other = slot_with(&w, &Fonts::default());
    let mut again = slot_with(&w, &Fonts::new([carlito.clone()]));
    assert_that!(w.link().sent_fonts.contains(&id(&carlito))).is_true();
    let compiled = again.render(&crate::test_packs::tiny_deck(), &std::collections::BTreeMap::new()).unwrap();
    assert_that!(compiled.svg(0).unwrap()).starts_with("<svg");
}
