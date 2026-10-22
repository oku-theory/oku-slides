use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{PackKind, WidgetRegistry, kinds::essentiel};
use oku_slides_i18n::Localizer;
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::{BuiltinPack, Fonts, PackSet, PackWidgets, RenderErrorKind};
use speculoos::prelude::*;

use super::*;
use crate::client::wire;
use crate::link::fingerprint_hex;
use crate::serve::pack_of;
use crate::test_packs::{LOOP_ON_IMPORT, QUOTE, deck_of, files, looping_deck, looping_layouts, ns};

fn worker() -> RenderWorker {
    worker_with(10_000, 10_000)
}

fn worker_with(timeout_ms: u64, probe_budget_ms: u64) -> RenderWorker {
    RenderWorker::new(
        RenderLaunch::InProcess,
        RenderSettings {
            timeout_ms,
            probe_budget_ms,
            memory_mib: 4096,
            max_frame_mib: 64,
            max_widget_kib: 256,
            max_widgets: 64,
        },
    )
}

/// Un pack d'un seul widget `quote` sous l'espace de noms `namespace`.
fn test_pack(namespace: &str) -> PackWidgets {
    PackWidgets::new(namespace, ns(namespace), PackKind::Widgets, files(&[("quote", QUOTE)])).unwrap()
}

/// Le registre d'un emplacement sans pack.
fn empty() -> Arc<WidgetRegistry> {
    Arc::new(WidgetRegistry::default())
}

/// Un deck d'une slide `essentiel/content` : un bloc `<ns>/quote` d'identifiant `e-<ns>` par espace de noms.
fn deck_with(namespaces: &[&str]) -> oku_slides_core::Deck {
    let blocks: Vec<_> = namespaces
        .iter()
        .map(|n| serde_json::json!({ "id": format!("e-{n}"), "kind": format!("{n}/quote"), "params": { "body": "x" } }))
        .collect();
    deck_of(serde_json::json!({ "id": "s", "kind": essentiel::CONTENT, "params": { "blocks": blocks } }))
}

#[test]
fn set_widgets_two_packs_renders_both_namespaces() {
    let w = worker();
    // La slide qui porte les blocs vient d'`essentiel`.
    let set =
        PackSet::new(vec![test_pack("beta"), test_pack("acme"), BuiltinPack::Essentiel.widgets().unwrap()]).unwrap();
    let registry = w.registry_of(&set).unwrap();
    let mut slot = w.slot(&Fonts::default(), &Localizer::new(Some("fr")), empty()).unwrap();
    slot.set_widgets(&set, registry).unwrap();
    let compiled = slot.render(&deck_with(&["acme", "beta"]), &BTreeMap::new()).unwrap();
    let mut ids: Vec<String> = compiled.layout().boxes.iter().map(|b| b.id.clone()).collect();
    ids.sort();
    assert_that!(ids).is_equal_to(vec!["e-acme".to_owned(), "e-beta".to_owned()]);
}

#[test]
fn probe_pack_reports_namespace_qualified_registry() {
    let probe = worker().probe_pack(&test_pack("acme")).unwrap();
    assert_that!(probe.registry.get("acme/quote").is_some()).is_true();
    assert_that!(probe.registry.get("quote").is_some()).is_false();
}

#[test]
fn registry_of_same_set_returns_the_same_arc_and_lists_every_pack() {
    let w = worker();
    let set = PackSet::new(vec![test_pack("acme"), test_pack("beta")]).unwrap();
    let (a, b) = (w.registry_of(&set).unwrap(), w.registry_of(&set).unwrap());
    assert_that!(Arc::ptr_eq(&a, &b)).is_true();
    assert_that!((a.get("acme/quote").is_some(), a.get("beta/quote").is_some())).is_equal_to((true, true));
}

#[test]
fn registry_of_empty_set_is_empty() {
    let registry = worker().registry_of(&PackSet::default()).unwrap();
    assert_that!(registry.is_empty()).is_true();
}

#[test]
fn registry_of_builtins_equals_the_builtin_registry() {
    let set = oku_slides_render::builtin_pack_set().unwrap();
    let registry = worker().registry_of(&set).unwrap();
    assert_that!(&*registry).is_equal_to(&*oku_slides_render::builtin_registry(&Fonts::default()).unwrap());
}

#[test]
fn probe_pack_of_a_layouts_file_declaring_element_drops_it() {
    let src = "#import \"/oku-slides/lib.typ\": widget\n#let widget = widget(\"w\", params: (:), (args, ctx) => [])\n";
    let pack = PackWidgets::new("acme", ns("acme"), PackKind::Layouts, files(&[("w", src)])).unwrap();
    let probe = worker().probe_pack(&pack).unwrap();
    let w = oku_slides_core::WidgetFile::parse("w").unwrap();
    assert_that!((probe.files[&w].is_valid(), probe.registry.get("acme/w").is_none())).is_equal_to((false, true));
}

#[test]
fn registry_of_keeps_a_bounded_number_of_registries() {
    let w = worker();
    let first = w.registry_of(&PackSet::new(vec![test_pack("p0")]).unwrap()).unwrap();
    for i in 1..=MAX_ASSEMBLED {
        w.registry_of(&PackSet::new(vec![test_pack(&format!("p{i}"))]).unwrap()).unwrap();
    }
    let again = w.registry_of(&PackSet::new(vec![test_pack("p0")]).unwrap()).unwrap();
    assert_that!(Arc::ptr_eq(&first, &again)).is_false();
}

#[test]
fn wire_pack_round_trip_keeps_namespace_kind_and_fingerprint() {
    let p = BuiltinPack::Essentiel.widgets().unwrap();
    let back = pack_of(wire(&p)).unwrap();
    assert_that!((back.namespace().clone(), back.kind(), back.fingerprint())).is_equal_to((
        p.namespace().clone(),
        p.kind(),
        p.fingerprint(),
    ));
}

#[test]
fn fingerprint_hex_matches_pack_fingerprint_with_namespace() {
    let p = test_pack("acme");
    assert_that!(fingerprint_hex(&wire(&p))).is_equal_to(hex::encode(p.fingerprint()));
    // L'espace de noms entre dans l'empreinte.
    let other = PackWidgets::new("acme", ns("beta"), PackKind::Widgets, p.files().clone()).unwrap();
    assert_that!(fingerprint_hex(&wire(&other))).is_not_equal_to(fingerprint_hex(&wire(&p)));
    // La sorte aussi.
    let layouts = PackWidgets::new("acme", ns("acme"), PackKind::Layouts, p.files().clone()).unwrap();
    assert_that!(fingerprint_hex(&wire(&layouts))).is_equal_to(hex::encode(layouts.fingerprint()));
    assert_that!(fingerprint_hex(&wire(&layouts))).is_not_equal_to(fingerprint_hex(&wire(&p)));
}

#[test]
fn registry_of_with_unprobed_files_is_not_cached() {
    // Budget nul : un seul fichier exécuté par appel, les autres restent non sondés.
    let w = worker_with(300, 0);
    let looping = |n: &str| LOOP_ON_IMPORT.replace("widget(\"boucle\"", &format!("widget(\"{n}\""));
    let (a, b) = (looping("a"), looping("b"));
    let pack = PackWidgets::new("boucles", ns("boucles"), PackKind::Widgets, files(&[("a", &a), ("b", &b)])).unwrap();
    let set = PackSet::new(vec![pack]).unwrap();
    let (first, second) = (w.registry_of(&set).unwrap(), w.registry_of(&set).unwrap());
    assert_that!(Arc::ptr_eq(&first, &second)).is_false();
}

/// Rend `looping_deck` avec ces packs ; l'échec de rendu et l'état fautif de `looping` et `acme`.
fn render_looping(packs: Vec<PackWidgets>) -> (RenderErrorKind, bool, bool) {
    let w = worker_with(300, 300);
    let looping = looping_layouts();
    let acme = test_pack("acme");
    let set = PackSet::new(packs.into_iter().chain([looping.clone()]).collect()).unwrap();
    let registry = w.registry_of(&set).unwrap();
    let mut slot = w.slot(&Fonts::default(), &Localizer::new(Some("fr")), empty()).unwrap();
    slot.set_widgets(&set, registry).unwrap();
    let kind = slot.render(&looping_deck(), &BTreeMap::new()).unwrap_err().kind();
    (kind, w.is_faulty(&looping), w.is_faulty(&acme))
}

#[test]
fn bounded_failure_with_two_packs_marks_none_faulty() {
    let (kind, looping, acme) = render_looping(vec![test_pack("acme")]);
    assert_that!((kind, looping, acme)).is_equal_to((RenderErrorKind::Timeout, false, false));
}

#[test]
fn bounded_failure_with_one_pack_marks_it_faulty() {
    let (kind, looping, acme) = render_looping(vec![]);
    assert_that!((kind, looping, acme)).is_equal_to((RenderErrorKind::Timeout, true, false));
}

/// Le widget `holder` : un slot qui n'accepte que des textes du pack `base`.
const HOLDER_OF_BASE_TEXT: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("holder", params: (kids: p.slot(accepts: ("base/text",))), (args, ctx) => [])
"#;

#[test]
fn probe_pack_with_a_slot_accepting_base_text_keeps_the_widget_valid() {
    let w = worker();
    let pack =
        PackWidgets::new("acme", ns("acme"), PackKind::Widgets, files(&[("holder", HOLDER_OF_BASE_TEXT)])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    let holder = oku_slides_core::WidgetFile::parse("holder").unwrap();
    assert_that!((probe.files[&holder].is_valid(), probe.registry.get("acme/holder").is_some()))
        .is_equal_to((true, true));
}
