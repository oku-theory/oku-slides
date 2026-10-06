//! Repli sur la base (packs qui bouclent, travailleur relancé) et reprise
//! de la sonde des widgets non sondés.

use std::path::Path;
use std::sync::Arc;

use serde_json::json;
use speculoos::prelude::*;

use super::deck_widgets_tests::{
    LOOP_ON_IMPORT, LOOP_ON_RENDER, LOOP_TIMEOUT_MS, QUOTE, add_loop, assert_registry_agrees, data_dir, fixture_path,
    session_in, session_with_timeout, set_templates, user_pack,
};
use crate::{Session, SessionErrorKind};

#[test]
fn apply_with_a_widget_that_loops_falls_back_to_the_base_registry_with_a_notice() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    let looping = user_pack(&s, "Boucle", &[("boucle", LOOP_ON_RENDER)]);
    let base = s.registry.clone();
    // Le pack est installé, puis son widget boucle au rendu : l'élément reste, la base le rend.
    s.apply("Pack", vec![set_templates(Some(&looping)), add_loop()], None).unwrap();
    assert_that!(Arc::ptr_eq(&s.registry, &base)).is_true();
    assert_that!(s.deck().slide("s-constat").is_some_and(|sl| sl.locate("l").is_some())).is_true();
    // L'avis nomme le pack par son nom, pas par son empreinte.
    assert_that!(s.doc.notice.clone().unwrap_or_default().as_str()).contains("Boucle");
    assert_registry_agrees(&s);
}

#[test]
fn apply_set_templates_to_a_pack_that_loops_keeps_the_base_registry_and_sets_a_notice() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    let looping = user_pack(&s, "Boucle", &[("boucle", LOOP_ON_IMPORT)]);
    let base = s.registry.clone();
    s.apply("Pack", vec![set_templates(Some(&looping))], None).unwrap();
    assert_that!(Arc::ptr_eq(&s.registry, &base)).is_true();
    assert_that!(s.doc.notice.is_some()).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn a_pack_that_looped_is_not_retried_in_the_session() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    let looping = user_pack(&s, "Boucle", &[("boucle", LOOP_ON_IMPORT)]);
    s.apply("Pack", vec![set_templates(Some(&looping))], None).unwrap();
    let started = std::time::Instant::now();
    s.apply("Pack", vec![set_templates(None)], None).unwrap();
    s.apply("Pack", vec![set_templates(Some(&looping))], None).unwrap();
    // Une nouvelle sonde coûterait au moins le délai.
    assert_that!(started.elapsed()).is_less_than(std::time::Duration::from_millis(LOOP_TIMEOUT_MS));
}
// ---- Relance du travailleur et repli hors du rendu du deck ----------------------------

/// Relance le travailleur hors du rendu du deck : la sonde d'un fichier qui
/// boucle (neuf à chaque appel, hors cache) dépasse le délai.
fn restart_worker_outside_the_deck(s: &Session) {
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let source = format!("{LOOP_ON_IMPORT}// {n}\n");
    let other = oku_slides_render::PackWidgets::new("autre", crate::test_support::widget_files(&[("boucle", &source)]))
        .unwrap();
    let probe = s.worker.probe_pack(&other).unwrap();
    let boucle = oku_slides_core::WidgetFile::parse("boucle").unwrap();
    assert_that!(probe.files[&boucle].diagnostics[0].message.as_str()).contains("took more than");
}

/// Une session dont le deck a le pack `boucle` (widget qui boucle au rendu)
/// sans élément `boucle` : le rendu du deck réussit.
fn session_on_looping_pack(data: &Path) -> Session {
    let mut s = session_with_timeout(data, LOOP_TIMEOUT_MS);
    let looping = user_pack(&s, "Boucle", &[("boucle", LOOP_ON_RENDER)]);
    s.apply("Pack", vec![set_templates(Some(&looping))], None).unwrap();
    assert_that!(s.pack.is_some()).is_true();
    s
}

/// Rend fautif le pack du deck hors du rendu du deck : un autre emplacement
/// rend un élément `boucle`, sans passer par la session.
fn fault_pack_outside_the_deck(s: &Session) {
    let mut slot =
        crate::new_renderer(&s.worker, s.render_fonts.clone(), &s.localizer, s.pack.as_ref(), &s.registry).unwrap();
    let mut deck = s.deck().clone();
    oku_slides_core::apply_all(&mut deck, vec![add_loop()], &s.registry).unwrap();
    let err = slot.render(&deck, &s.doc.sources.assets).unwrap_err();
    assert_that!(err.kind().is_bounded()).is_true();
}

/// Le repli a eu lieu : la base, l'avis qui nomme le pack (son nom), les registres d'accord.
fn assert_fell_back(s: &Session, base: &Arc<oku_slides_core::WidgetRegistry>) {
    assert_that!(Arc::ptr_eq(&s.registry, base)).is_true();
    assert_that!(s.pack.is_none()).is_true();
    assert_that!(s.doc.notice.clone().unwrap_or_default().as_str()).contains("Boucle");
    assert_registry_agrees(s);
}

#[test]
fn save_after_a_restart_outside_the_deck_render_rerenders_and_writes() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    restart_worker_outside_the_deck(&s);
    let path = data.0.join("apres-relance.oku-slides.pdf");
    s.save(Some(&path)).unwrap();
    assert_that!(path.exists()).is_true();
}

#[test]
fn svg_and_png_after_a_restart_outside_the_deck_render_are_served() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    restart_worker_outside_the_deck(&s);
    // Aucune page n'est en cache : le SVG se demande au travailleur relancé.
    assert_that!(s.svg_cache.is_empty()).is_true();
    assert_that!(s.svg("s-constat").unwrap().starts_with("<svg")).is_true();
    assert_that!(s.png("s-constat").unwrap()[..4].to_vec()).is_equal_to(b"\x89PNG".to_vec());
}

#[test]
fn templates_view_with_a_template_that_loops_falls_back_to_the_base_with_a_notice() {
    let data = data_dir();
    let base = session_in(&data.0).registry.clone();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    // Un modèle du pack porte un élément `boucle` : son aperçu boucle.
    let slide = json!({ "id": "tpl-l", "kind": "content", "params": { "blocks": [
        { "id": "l", "kind": "boucle", "params": { "body": "x" } } ] } });
    let mut m = crate::test_support::manifest(uuid::Uuid::new_v4(), "Boucle", None, "2026-10-01T10:00:00Z");
    m.templates = vec![serde_json::from_value(json!({ "id": "l", "name": "Boucle", "slide": slide })).unwrap()];
    let looping = crate::test_support::add_version(&s.library, &m, &[("boucle", LOOP_ON_RENDER)]);
    s.apply("Pack", vec![set_templates(Some(&looping))], None).unwrap();
    let view = s.templates_view(None).unwrap();
    assert_that!(view.templates.len()).is_equal_to(1);
    assert_fell_back(&s, &base);
}

#[test]
fn variants_with_an_element_that_loops_fall_back_to_the_base_with_a_notice() {
    let data = data_dir();
    let base = session_in(&data.0).registry.clone();
    let mut s = session_on_looping_pack(&data.0);
    let spec: crate::agent::ShowVariants = serde_json::from_value(json!({ "slide": "s-constat", "variants": [
        { "label": "a", "ops": [serde_json::to_value(add_loop()).unwrap()] },
        { "label": "b", "ops": [{ "op": "set_param", "target": { "slide": "s-constat" },
            "param": "title", "value": "B" }] } ] }))
    .unwrap();
    // Refaites avec la base, qui ne connaît pas `boucle` : la variante est refusée, pas gelée.
    let err = s.variants_surface("v-1", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_fell_back(&s, &base);
}

#[test]
fn refresh_fonts_after_the_pack_faulted_elsewhere_falls_back_and_succeeds() {
    let data = data_dir();
    let base = session_in(&data.0).registry.clone();
    let mut s = session_on_looping_pack(&data.0);
    fault_pack_outside_the_deck(&s);
    s.refresh_fonts().unwrap();
    assert_fell_back(&s, &base);
    s.rerender().unwrap();
}

#[test]
fn set_templates_back_to_a_faulty_pack_names_it_by_its_name() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    let looping = user_pack(&s, "Boucle", &[("boucle", LOOP_ON_RENDER)]);
    s.apply("Pack", vec![set_templates(Some(&looping)), add_loop()], None).unwrap();
    s.apply("Pack", vec![set_templates(None)], None).unwrap();
    s.apply("Pack", vec![set_templates(Some(&looping))], None).unwrap();
    let notice = s.doc.notice.clone().unwrap_or_default();
    assert_that!(notice.as_str()).contains("Boucle");
    assert_that!(notice.as_str()).does_not_contain(looping.version.hex());
}
/// Le fichier `name` : cassé dès sa compilation (sonde), sans délai.
fn broken_file(name: &str) -> String {
    format!("#let {} = (\n", name.replace('-', "_"))
}

/// L'avis posé depuis le dernier appel, effacé.
fn take_notice(s: &mut Session) -> String {
    s.doc.notice.take().unwrap_or_default()
}

#[test]
fn widgets_left_unprobed_by_the_budget_are_probed_at_the_next_renders() {
    let data = crate::test_support::fresh_dir("oku-slides-packs-");
    let mut settings = crate::test_support::settings_on(data.path());
    // Budget nul : un seul fichier exécuté par sonde, quelle que soit la durée réelle.
    settings.render.probe_budget_ms = 0;
    let mut s = crate::test_support::session_with(settings);
    s.open(&fixture_path()).unwrap();
    let (a, b, c) = (broken_file("casse-a"), broken_file("casse-b"), broken_file("casse-c"));
    let files = [("casse-a", a.as_str()), ("casse-b", &b), ("casse-c", &c), ("quote", QUOTE)];
    let pack = user_pack(&s, "Cassés", &files);
    s.doc.notice = None;
    // Le budget part dans `casse-a` : les autres ne sont pas sondés, ni dits cassés.
    s.apply("Pack", vec![set_templates(Some(&pack))], None).unwrap();
    assert_that!(s.registry.get("quote")).is_none();
    let notice = take_notice(&mut s);
    assert_that!(notice.as_str()).contains("casse-a");
    assert_that!(notice.contains("casse-b") || notice.contains("quote")).is_false();
    // Chaque rendu sonde de nouveau les restants ; l'avis nomme les nouveaux cassés.
    s.rerender().unwrap();
    let notice = take_notice(&mut s);
    assert_that!(notice.as_str()).contains("casse-b");
    assert_that!(notice.contains("casse-a") || notice.contains("casse-c")).is_false();
    s.rerender().unwrap();
    let notice = take_notice(&mut s);
    assert_that!(notice.as_str()).contains("casse-c");
    assert_that!(notice.contains("casse-b") || notice.contains("quote")).is_false();
    s.rerender().unwrap();
    assert_that!(s.registry.get("quote")).is_some();
    assert_that!(s.doc.notice).is_none();
    assert_that!(s.widgets_unprobed.is_empty()).is_true();
    assert_registry_agrees(&s);
    // Plus rien à sonder : un rendu ne résout plus les widgets, le registre reste le même.
    let registry = s.registry.clone();
    s.rerender().unwrap();
    assert_that!(Arc::ptr_eq(&s.registry, &registry)).is_true();
}

#[test]
fn a_failed_install_while_probing_again_does_not_fail_the_edit() {
    let data = crate::test_support::fresh_dir("oku-slides-packs-");
    let mut s = crate::test_support::opened_on(data.path());
    let pack = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    s.apply("Pack", vec![set_templates(Some(&pack))], None).unwrap();
    let registry = s.registry.clone();
    // Un fichier reste à sonder ; l'installation des widgets sondés échoue.
    s.widgets_unprobed.insert(oku_slides_core::WidgetFile::parse("quote").unwrap());
    s.failpoints.install_widgets = true;
    let theme = oku_slides_core::Op::SetTheme { theme: Some("pro".into()) };
    s.apply("Thème", vec![theme], None).unwrap();
    // Les widgets installés restent ; la sonde sera reprise au rendu suivant.
    assert_that!(Arc::ptr_eq(&s.registry, &registry)).is_true();
    assert_that!(s.widgets_unprobed.is_empty()).is_false();
    assert_registry_agrees(&s);
    s.rerender().unwrap();
    assert_that!(s.widgets_unprobed.is_empty()).is_true();
}
