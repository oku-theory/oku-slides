use std::collections::BTreeMap;

use oku_slides_i18n::Localizer;
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::{Fonts, PackWidgets, RenderErrorKind, base_registry};
use speculoos::prelude::*;

use super::*;
use crate::test_packs::{LOOP_ON_IMPORT, LOOP_ON_RENDER, QUOTE, files, tiny_deck};
use crate::{PackProbe, RemoteCompiled};

fn worker(timeout_ms: u64) -> RenderWorker {
    RenderWorker::new(
        RenderLaunch::InProcess,
        RenderSettings {
            timeout_ms,
            probe_budget_ms: timeout_ms,
            memory_mib: 4096,
            max_frame_mib: 64,
            max_widget_kib: 256,
            max_widgets: 64,
        },
    )
}

fn slot(w: &RenderWorker) -> RenderSlot {
    w.slot(&Fonts::default(), &Localizer::new(Some("fr")), base_registry(&Fonts::default()).unwrap()).unwrap()
}

#[test]
fn worker_is_started_by_its_first_slot_only() {
    let w = worker(10_000);
    assert_that!(w.is_started()).is_false();
    let _slot = slot(&w);
    assert_that!(w.is_started()).is_true();
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
fn set_widgets_that_loops_marks_pack_faulty() {
    let w = worker(300);
    let pack = PackWidgets::new("boucle", files(&[("boucle", LOOP_ON_IMPORT)])).unwrap();
    let mut s = slot(&w);
    let err = s.set_widgets(Some(&pack), base_registry(&Fonts::default()).unwrap()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Timeout);
    assert_that!(w.is_faulty(&pack)).is_true();
}

#[test]
fn render_with_looping_widget_times_out_then_is_disabled_without_retry() {
    let w = worker(300);
    let pack = PackWidgets::new("rendu", files(&[("boucle", LOOP_ON_RENDER)])).unwrap();
    let registry = w.probe_pack(&pack).unwrap().registry;
    let mut s = slot(&w);
    s.set_widgets(Some(&pack), registry).unwrap();
    let deck = oku_slides_core::Deck::from_json(r#"{"version":5,"params":{"title":"T"},"slides":[{"id":"s","kind":"content","params":{"blocks":[{"id":"b","kind":"boucle","params":{"body":"x"}}]}}]}"#).unwrap();
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
    let mut stale = second.clone_handle_for_test();
    stale.generation = generation;
    assert_that!(stale.svg(0).unwrap_err().kind()).is_equal_to(RenderErrorKind::Stale);
}

/// Relance le travailleur `InProcess` (un fil ne se tue pas) : la sonde d'un
/// fichier qui boucle (propre à `id`, hors cache) dépasse le délai et
/// abandonne la connexion, sans accuser de pack.
fn force_restart(w: &RenderWorker, id: &str) {
    let source = format!("{LOOP_ON_IMPORT}// {id}\n");
    let pack = PackWidgets::new(id, files(&[("boucle", &source)])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    assert_that!(probe.files[&wf("boucle")].diagnostics[0].message.as_str()).contains("took more than");
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
fn remote_reference_of_previous_worker_is_stale() {
    let w = worker(3_000);
    let pptx = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/import/libreoffice.pptx")).unwrap();
    let old = w.reference(&pptx, &Fonts::default(), None).unwrap();
    force_restart(&w, "boucle-r");
    assert_that!(old.png(0, 480).unwrap_err().kind()).is_equal_to(RenderErrorKind::Stale);
}

#[test]
fn set_widgets_with_faulty_pack_is_disabled_without_call() {
    let w = worker(3_000);
    let pack = PackWidgets::new("boucle", files(&[("boucle", LOOP_ON_IMPORT)])).unwrap();
    let mut s = slot(&w);
    let base = base_registry(&Fonts::default()).unwrap();
    assert_that!(s.set_widgets(Some(&pack), base.clone()).unwrap_err().kind()).is_equal_to(RenderErrorKind::Timeout);
    force_restart(&w, "boucle-b");
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

fn worker_with(timeout_ms: u64, max_widgets: usize) -> RenderWorker {
    let settings = RenderSettings {
        timeout_ms,
        probe_budget_ms: timeout_ms,
        memory_mib: 4096,
        max_frame_mib: 64,
        max_widget_kib: 256,
        max_widgets,
    };
    RenderWorker::new(RenderLaunch::InProcess, settings)
}

/// Un travailleur au délai `timeout_ms` dont le budget de sonde est épuisé
/// d'emblée : un seul fichier exécuté par appel, quelle que soit la durée réelle.
fn worker_without_budget(timeout_ms: u64) -> RenderWorker {
    let settings = RenderSettings {
        timeout_ms,
        probe_budget_ms: 0,
        memory_mib: 4096,
        max_frame_mib: 64,
        max_widget_kib: 256,
        max_widgets: 64,
    };
    RenderWorker::new(RenderLaunch::InProcess, settings)
}

/// Les fichiers non sondés de `probe`, par nom.
fn unprobed(probe: &PackProbe) -> Vec<&str> {
    probe.unprobed.iter().map(oku_slides_core::WidgetFile::as_str).collect()
}

fn wf(name: &str) -> oku_slides_core::WidgetFile {
    oku_slides_core::WidgetFile::parse(name).unwrap()
}

/// Un deck d'une diapositive au bloc `boucle`.
fn looping_deck() -> oku_slides_core::Deck {
    let mut deck = tiny_deck();
    deck.slides[0] = serde_json::from_value(serde_json::json!({
        "id": "s", "kind": "content",
        "params": { "blocks": [{ "id": "b", "kind": "boucle", "params": { "body": "x" } }] }
    }))
    .unwrap();
    deck
}

#[test]
fn probe_pack_keeps_the_valid_widgets_and_reports_the_broken_one() {
    let w = worker(10_000);
    let pack = PackWidgets::new("mixte", files(&[("quote", QUOTE), ("casse", "#let widget = (")])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    assert_that!(probe.files[&wf("quote")].is_valid()).is_true();
    assert_that!(probe.files[&wf("casse")].is_valid()).is_false();
    assert_that!(probe.registry.get("quote")).is_some();
    let kept = probe.pack.unwrap();
    assert_that!(kept.files().keys().cloned().collect::<Vec<_>>()).is_equal_to(vec![wf("quote")]);
}

#[test]
fn probe_pack_alternating_two_packs_with_the_same_names_keeps_both_probes() {
    let w = worker(10_000);
    let a = PackWidgets::new("a", files(&[("quote", &format!("{QUOTE}// a\n"))])).unwrap();
    let b = PackWidgets::new("b", files(&[("quote", &format!("{QUOTE}// b\n"))])).unwrap();
    for _ in 0..2 {
        w.probe_pack(&a).unwrap();
        w.probe_pack(&b).unwrap();
    }
    assert_that!(w.1.file_probes.lock().unwrap().len()).is_equal_to(2);
}

#[test]
fn probe_pack_keeps_a_bounded_number_of_probes_per_file() {
    let w = worker(10_000);
    for i in 0..oku_slides_render::FileProbeCache::PER_FILE + 2 {
        let pack = PackWidgets::new("p", files(&[("quote", &format!("{QUOTE}// {i}\n"))])).unwrap();
        assert_that!(w.probe_pack(&pack).unwrap().files[&wf("quote")].is_valid()).is_true();
    }
    assert_that!(w.1.file_probes.lock().unwrap().len()).is_equal_to(oku_slides_render::FileProbeCache::PER_FILE);
}

#[test]
fn probe_pack_with_a_file_that_loops_gives_a_diagnostic_and_accuses_nothing() {
    let w = worker_without_budget(300);
    let pack = PackWidgets::new("boucle", files(&[("boucle", LOOP_ON_IMPORT), ("quote", QUOTE)])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    assert_that!(probe.files[&wf("boucle")].is_valid()).is_false();
    assert_that!(unprobed(&probe)).is_equal_to(vec!["quote"]);
    assert_that!(w.is_faulty(&pack)).is_false();
    // Gardé : le seul fichier exécuté de l'appel est `quote`, pas `boucle` de nouveau.
    let probe = w.probe_pack(&pack).unwrap();
    assert_that!(probe.files[&wf("boucle")].is_valid()).is_false();
    assert_that!(probe.unprobed.is_empty()).is_true();
    assert_that!(probe.files[&wf("quote")].is_valid()).is_true();
    assert_that!(w.is_faulty(probe.pack.as_ref().unwrap())).is_false();
}

/// Un fichier `name` qui boucle au chargement.
fn looping_file(name: &str) -> String {
    LOOP_ON_IMPORT.replace("widget(\"boucle\"", &format!("widget(\"{name}\""))
}

#[test]
fn probe_pack_of_looping_files_spends_one_timeout_and_leaves_the_rest_unprobed() {
    let w = worker_without_budget(300);
    let (a, b, c) = (looping_file("a"), looping_file("b"), looping_file("c"));
    let pack = PackWidgets::new("boucles", files(&[("a", &a), ("b", &b), ("c", &c)])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    assert_that!(probe.files[&wf("c")].diagnostics[0].message.as_str()).starts_with("Not checked yet");
    assert_that!(probe.files.values().all(|p| !p.is_valid())).is_true();
    assert_that!(unprobed(&probe)).is_equal_to(vec!["b", "c"]);
    // Un seul délai dépensé : seule la sonde de `a` est gardée.
    assert_that!(w.1.file_probes.lock().unwrap().len()).is_equal_to(1);
    // Non gardés : sondés aux appels suivants, un par appel.
    assert_that!(unprobed(&w.probe_pack(&pack).unwrap())).is_equal_to(vec!["c"]);
    let probe = w.probe_pack(&pack).unwrap();
    assert_that!(probe.files[&wf("c")].diagnostics[0].message.as_str()).does_not_contain("Not checked yet");
    // Un délai gardé : cassé, plus « non sondé ».
    assert_that!(probe.unprobed.is_empty()).is_true();
}

#[test]
fn probe_pack_with_a_zero_budget_probes_one_file_per_call() {
    let w = worker_without_budget(10_000);
    let pack =
        PackWidgets::new("casses", files(&[("a", "#let a = ("), ("b", "#let b = ("), ("c", "#let c = (")])).unwrap();
    // Le budget épuisé d'emblée n'empêche pas un fichier par appel ; les sondés sont gardés.
    assert_that!(unprobed(&w.probe_pack(&pack).unwrap())).is_equal_to(vec!["b", "c"]);
    assert_that!(unprobed(&w.probe_pack(&pack).unwrap())).is_equal_to(vec!["c"]);
    let probe = w.probe_pack(&pack).unwrap();
    assert_that!(probe.unprobed.is_empty()).is_true();
    assert_that!(probe.files.values().all(|p| !p.is_valid())).is_true();
}

#[test]
fn probe_pack_beyond_max_widgets_does_not_load_the_extra_files() {
    let w = worker_with(10_000, 1);
    let pack = PackWidgets::new("trop", files(&[("a", QUOTE), ("b", QUOTE)])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    assert_that!(probe.files[&wf("b")].diagnostics[0].message.as_str()).contains("more than 1");
}

#[test]
fn probe_pack_with_a_file_over_max_widget_kib_does_not_load_it() {
    let w = worker(10_000);
    let big = format!("{QUOTE}// {}\n", "x".repeat(256 * 1024));
    let pack = PackWidgets::new("lourd", files(&[("quote", &big)])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    assert_that!(probe.files[&wf("quote")].diagnostics[0].message.as_str())
        .is_equal_to("Larger than 256 KiB: not loaded.");
    assert_that!(probe.pack).is_none();
    // Rien n'a été exécuté : aucun travailleur lancé.
    assert_that!(w.link().connected()).is_false();
}

#[test]
fn trial_slot_render_that_loops_times_out_and_accuses_nothing() {
    let w = worker(300);
    let pack = PackWidgets::new("rendu", files(&[("boucle", LOOP_ON_RENDER)])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    let kept = probe.pack.clone().unwrap();
    let base = base_registry(&Fonts::default()).unwrap();
    let mut s = w.trial_slot(&Fonts::default(), &Localizer::new(Some("fr")), base).unwrap();
    s.set_widgets(Some(&kept), probe.registry.clone()).unwrap();
    let deck = looping_deck();
    let err = s.render(&deck, &BTreeMap::new()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Timeout);
    assert_that!(w.is_faulty(&kept)).is_false();
    // Pas d'écartement : l'essai suivant retente (délai, pas `PackDisabled`).
    s.set_widgets(Some(&kept), probe.registry).unwrap();
    assert_that!(s.render(&deck, &BTreeMap::new()).unwrap_err().kind()).is_equal_to(RenderErrorKind::Timeout);
}

#[test]
fn trial_slot_with_a_faulty_pack_renders_it_until_a_restart_then_is_disabled() {
    let w = worker(3_000);
    let pack = PackWidgets::new("rendu", files(&[("boucle", LOOP_ON_RENDER)])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    let kept = probe.pack.clone().unwrap();
    // Le rendu du deck accuse le pack.
    let mut deck_slot = slot(&w);
    deck_slot.set_widgets(Some(&kept), probe.registry.clone()).unwrap();
    assert_that!(deck_slot.render(&looping_deck(), &BTreeMap::new()).unwrap_err().kind())
        .is_equal_to(RenderErrorKind::Timeout);
    assert_that!(w.is_faulty(&kept)).is_true();
    // L'essai pose tout de même le pack fautif, et rend.
    let base = base_registry(&Fonts::default()).unwrap();
    let mut s = w.trial_slot(&Fonts::default(), &Localizer::new(Some("fr")), base).unwrap();
    s.set_widgets(Some(&kept), probe.registry).unwrap();
    assert_that!(s.render(&tiny_deck(), &BTreeMap::new())).is_ok();
    // Après une relance, le rejeu l'a écarté : le dire, pas rendre sans lui.
    force_restart(&w, "boucle-c");
    assert_that!(s.render(&tiny_deck(), &BTreeMap::new()).unwrap_err().kind())
        .is_equal_to(RenderErrorKind::PackDisabled);
}

#[test]
fn probe_pack_whose_link_cannot_start_leaves_its_files_unprobed() {
    let settings = RenderSettings {
        timeout_ms: 10_000,
        probe_budget_ms: 10_000,
        memory_mib: 4096,
        max_frame_mib: 64,
        max_widget_kib: 256,
        max_widgets: 64,
    };
    let w = RenderWorker::new(RenderLaunch::Process { program: "/inexistant/oku-slides".into() }, settings);
    let pack = PackWidgets::new("p", files(&[("quote", QUOTE)])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    // Panne du lien, pas du fichier : non sondé, repris au prochain appel.
    assert_that!(unprobed(&probe)).is_equal_to(vec!["quote"]);
    assert_that!(probe.files[&wf("quote")].is_valid()).is_false();
    assert_that!(w.1.file_probes.lock().unwrap().len()).is_equal_to(0);
}

#[test]
fn probe_pack_limit_diagnostics_speak_the_language_of_the_slots() {
    let w = worker_with(10_000, 2);
    let _fr = slot(&w);
    let big = format!("{QUOTE}// {}\n", "x".repeat(256 * 1024));
    let pack = PackWidgets::new("p", files(&[("a", &big), ("b", QUOTE), ("c", QUOTE)])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    assert_that!(probe.files[&wf("a")].diagnostics[0].message.as_str()).is_equal_to("Plus de 256 Kio : non chargé.");
    assert_that!(probe.files[&wf("c")].diagnostics[0].message.as_str())
        .is_equal_to("Le pack a plus de 2 fichiers de widgets : celui-ci n'est pas chargé.");
}

#[test]
fn probe_pack_unprobed_diagnostic_speaks_the_language_of_the_slots() {
    let w = worker_without_budget(10_000);
    let _fr = slot(&w);
    let pack = PackWidgets::new("casses", files(&[("a", "#let a = ("), ("b", "#let b = (")])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    assert_that!(probe.files[&wf("b")].diagnostics[0].message.as_str()).starts_with("Pas encore vérifié");
}
