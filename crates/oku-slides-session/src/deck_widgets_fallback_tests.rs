//! Repli par pack (packs qui bouclent, travailleur relancé, dichotomie) et
//! reprise de la sonde des widgets non sondés.

use std::path::Path;
use std::sync::Arc;

use serde_json::json;
use speculoos::prelude::*;

use super::deck_widgets_tests::{
    LOOP_ON_IMPORT, LOOP_ON_RENDER, LOOP_TIMEOUT_MS, QUOTE, add_base_loop, add_loop, assert_registry_agrees, data_dir,
    edited_base_pack, fixture_path, kind_of, namespace_of_pack, op, pin_pack, session_with_timeout, unpin_pack,
    user_pack,
};
use oku_slides_core::kinds;
use oku_slides_render::is_trusted_pack;

use crate::test_support::ns;
use crate::{Session, SessionErrorKind};

#[test]
fn apply_with_a_widget_that_loops_falls_back_to_the_base_registry_with_a_notice() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    let looping = user_pack(&s, "Boucle", &[("boucle", LOOP_ON_RENDER)]);
    let base = s.registry.clone();
    // Le pack est installé, puis son widget boucle au rendu : l'élément reste, la base le rend.
    s.apply("Pack", vec![pin_pack(&looping), add_loop(&looping)], None).unwrap();
    assert_that!(Arc::ptr_eq(&s.registry, &base)).is_true();
    assert_that!(s.deck().slide("s-constat").is_some_and(|sl| sl.locate("l").is_some())).is_true();
    // L'avis nomme le pack par son nom, pas par son empreinte.
    assert_that!(s.doc.notice.clone().unwrap_or_default().as_str()).contains("Boucle");
    assert_registry_agrees(&s);
}

#[test]
fn apply_set_pack_to_a_pack_that_loops_keeps_the_base_registry_and_sets_a_notice() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    let looping = user_pack(&s, "Boucle", &[("boucle", LOOP_ON_IMPORT)]);
    let base = s.registry.clone();
    s.apply("Pack", vec![pin_pack(&looping)], None).unwrap();
    assert_that!(Arc::ptr_eq(&s.registry, &base)).is_true();
    assert_that!(s.doc.notice.is_some()).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn a_single_looping_user_pack_beside_the_builtins_is_disabled_and_base_kept() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    let looping = user_pack(&s, "Boucle", &[("boucle", LOOP_ON_RENDER)]);
    s.apply("Pack", vec![pin_pack(&looping), add_loop(&looping)], None).unwrap();
    let disabled: Vec<String> = s.widgets_disabled.keys().map(ToString::to_string).collect();
    assert_that!(disabled).is_equal_to(vec![namespace_of_pack(&looping)]);
    assert_that!(s.registry.get(kinds::base::TEXT).is_some()).is_true();
}

/// t-1053 : un `base` modifié n'est jamais accusé ni écarté (Essentiel et tous les éléments le
/// suivraient) : l'échec borné est celui du deck, l'op est refusée et rien n'est écarté.
#[test]
fn an_edited_base_that_loops_is_never_disabled_and_the_render_error_surfaces() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    // Une version modifiée de `base` (même id, un widget qui boucle) épinglée sous `base`.
    let edited = edited_base_pack(&s, LOOP_ON_RENDER);
    s.apply("Pack", vec![pin_pack(&edited)], None).unwrap();
    let err = s.apply("Boucle", vec![add_base_loop()], None).unwrap_err();
    let kind = err.render_error().map(oku_slides_render::RenderError::kind);
    assert_that!(kind).is_equal_to(Some(oku_slides_render::RenderErrorKind::Timeout));
    assert_that!(s.widgets_disabled.contains_key(&kinds::base::namespace())).is_false();
    assert_that!(s.deck().slide("s-constat").is_some_and(|sl| sl.locate("l").is_some())).is_false();
}

/// Un widget `boucle` qui se rend sans rien faire : un `base` modifié sans faute.
const CALM_LOOP_FILE: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("boucle", params: (body: p.plain()), (args, ctx) => [])
"#;

/// La dichotomie garde un `base` modifié dans le rendu témoin : s'il boucle, personne n'est accusé.
#[test]
fn an_edited_base_that_loops_beside_a_user_pack_disables_neither() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    let edited = edited_base_pack(&s, LOOP_ON_RENDER);
    let quote = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    s.apply("Packs", vec![pin_pack(&edited), pin_pack(&quote)], None).unwrap();
    let err = s.apply("Boucle", vec![add_base_loop()], None).unwrap_err();
    let kind = err.render_error().map(oku_slides_render::RenderError::kind);
    assert_that!((kind, s.widgets_disabled.is_empty()))
        .is_equal_to((Some(oku_slides_render::RenderErrorKind::Timeout), true));
}

/// À côté d'un `base` modifié, un pack de l'utilisateur qui boucle est trouvé et écarté seul.
#[test]
fn a_looping_user_pack_beside_an_edited_base_is_disabled_and_base_kept() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    let edited = edited_base_pack(&s, CALM_LOOP_FILE);
    let looping = user_pack(&s, "Boucle", &[("boucle", LOOP_ON_RENDER)]);
    s.apply("Base", vec![pin_pack(&edited)], None).unwrap();
    s.apply("Pack", vec![pin_pack(&looping), add_loop(&looping)], None).unwrap();
    let disabled: Vec<String> = s.widgets_disabled.keys().map(ToString::to_string).collect();
    assert_that!(disabled).is_equal_to(vec![namespace_of_pack(&looping)]);
}

#[test]
fn a_pack_that_looped_is_not_retried_in_the_session() {
    let data = data_dir();
    let mut settings = crate::test_support::settings_on(&data.0);
    settings.render.timeout_ms = LOOP_TIMEOUT_MS;
    // Budget nul : un seul fichier exécuté par sonde, quelle que soit la durée réelle.
    settings.render.probe_budget_ms = 0;
    let mut s = crate::test_support::session_with(settings);
    s.open(&fixture_path()).unwrap();
    let looping = user_pack(&s, "Boucle", &[("boucle", LOOP_ON_IMPORT), ("quote", QUOTE)]);
    // `boucle` dépasse le délai ; `quote`, après lui, n'est pas sondé.
    s.apply("Pack", vec![pin_pack(&looping)], None).unwrap();
    let unprobed: Vec<&str> = s.widgets_unprobed.values().flatten().map(oku_slides_core::WidgetFile::as_str).collect();
    assert_that!(unprobed).is_equal_to(vec!["quote"]);
    s.apply("Pack", vec![unpin_pack(&looping)], None).unwrap();
    s.apply("Pack", vec![pin_pack(&looping)], None).unwrap();
    // `boucle` n'est pas sondé de nouveau : la seule sonde de l'appel va à `quote`.
    assert_that!(s.widgets_unprobed.is_empty()).is_true();
    assert_that!(s.registry.get(&kind_of(&looping, "quote"))).is_some();
    assert_registry_agrees(&s);
}
// ---- Relance du travailleur et repli hors du rendu du deck ----------------------------

/// Relance le travailleur hors du rendu du deck : la sonde d'un fichier qui
/// boucle (neuf à chaque appel, hors cache) dépasse le délai.
fn restart_worker_outside_the_deck(s: &Session) {
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let source = format!("{LOOP_ON_IMPORT}// {n}\n");
    let files = crate::test_support::widget_files(&[("boucle", &source)]);
    let other =
        oku_slides_render::PackWidgets::new("autre", ns("autre"), oku_slides_core::PackKind::Widgets, files).unwrap();
    let probe = s.worker.probe_pack(&other).unwrap();
    let boucle = oku_slides_core::WidgetFile::parse("boucle").unwrap();
    assert_that!(probe.files[&boucle].diagnostics[0].message.as_str()).contains("took more than");
}

/// Une session dont le deck a le pack `boucle` (widget qui boucle au rendu)
/// sans élément `boucle` : le rendu du deck réussit. Rend aussi son épinglage.
fn session_on_looping_pack(data: &Path) -> (Session, oku_slides_core::PackPin) {
    let mut s = session_with_timeout(data, LOOP_TIMEOUT_MS);
    let looping = user_pack(&s, "Boucle", &[("boucle", LOOP_ON_RENDER)]);
    s.apply("Pack", vec![pin_pack(&looping)], None).unwrap();
    assert_that!(s.packs.is_empty()).is_false();
    (s, looping)
}

/// Rend fautif le pack `looping` du deck hors du rendu du deck, comme un autre
/// onglet dont le rendu l'aurait accusé : le processus de rendu le tient fautif,
/// sans que la session le sache.
fn fault_pack_outside_the_deck(s: &Session, looping: &oku_slides_core::PackPin) {
    let pack = s.packs.packs().iter().find(|p| p.id() == looping.version.hex()).unwrap();
    s.worker.mark_faulty(pack);
    assert_that!(s.worker.is_faulty(pack)).is_true();
}

/// Le repli a eu lieu : seuls les packs de confiance restent, sans les widgets de `looping` ;
/// l'avis nomme le pack (son nom), les registres sont d'accord.
/// Le layout `boucle` : boucle à chaque rendu d'une slide qui l'emploie.
const LOOP_SLIDE_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("boucle", category: "slide", params: (body: p.plain()),
  (args, ctx) => { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; [#n] })
"#;

fn assert_fell_back(s: &Session, looping: &oku_slides_core::PackPin) {
    assert_that!(s.packs.packs().iter().all(is_trusted_pack)).is_true();
    assert_that!(s.registry.get(&kind_of(looping, "boucle"))).is_none();
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
fn layouts_view_with_a_layout_that_loops_leaves_its_pack_out_with_a_notice() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    // Un pack de layouts dont le layout `boucle` boucle au rendu : son aperçu boucle.
    let m = crate::test_support::manifest(uuid::Uuid::new_v4(), "Boucle", None, "2026-10-01T10:00:00Z");
    let kind = oku_slides_core::PackKind::Layouts;
    let looping = crate::test_support::add_version_of(&s.library, kind, &m, &[("boucle", LOOP_SLIDE_ON_RENDER)]);
    let op = oku_slides_core::Op::AddPack { kind, namespace: m.namespace.clone(), pin: looping.clone() };
    s.apply("Pack", vec![op], None).unwrap();
    let view = s.layouts_view().unwrap();
    let shown: Vec<&str> = view.packs.iter().map(|p| p.namespace.as_str()).collect();
    assert_that!(shown).is_equal_to(vec!["essentiel"]);
    assert_fell_back(&s, &looping);
}

#[test]
fn variants_with_an_element_that_loops_fall_back_to_the_base_with_a_notice() {
    let data = data_dir();
    let (mut s, looping) = session_on_looping_pack(&data.0);
    let spec: crate::agent::ShowVariants = serde_json::from_value(json!({ "slide": "s-constat", "variants": [
        { "label": "a", "ops": [serde_json::to_value(add_loop(&looping)).unwrap()] },
        { "label": "b", "ops": [{ "op": "set_param", "target": { "slide": "s-constat" },
            "param": "title", "value": "B" }] } ] }))
    .unwrap();
    // Refaites avec la base, qui ne connaît pas `boucle` : la variante est refusée, pas gelée.
    let err = s.variants_surface("v-1", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_fell_back(&s, &looping);
}

#[test]
fn refresh_fonts_after_the_pack_faulted_elsewhere_falls_back_and_succeeds() {
    let data = data_dir();
    let (mut s, looping) = session_on_looping_pack(&data.0);
    fault_pack_outside_the_deck(&s, &looping);
    s.refresh_fonts().unwrap();
    assert_fell_back(&s, &looping);
    s.rerender().unwrap();
}

#[test]
fn fall_back_on_a_pack_disabled_elsewhere_names_the_pack_once() {
    let data = data_dir();
    let (mut s, looping) = session_on_looping_pack(&data.0);
    let id = s.packs.packs().first().map(|p| p.id().to_owned()).unwrap_or_default();
    fault_pack_outside_the_deck(&s, &looping);
    s.refresh_fonts().unwrap();
    // La cause `PackDisabled` redirait le pack, par son empreinte : l'avis le nomme une fois, par son nom.
    let notice = s.doc.notice.clone().unwrap_or_default();
    assert_that!(notice.matches("Boucle").count()).is_equal_to(1);
    assert_that!(notice.contains(&id)).is_false();
}

#[test]
fn set_pack_back_to_a_faulty_pack_names_it_by_its_name() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    let looping = user_pack(&s, "Boucle", &[("boucle", LOOP_ON_RENDER)]);
    s.apply("Pack", vec![pin_pack(&looping), add_loop(&looping)], None).unwrap();
    let remove = op(json!({ "op": "remove_element", "slide": "s-constat", "element": "l" }));
    s.apply("Pack", vec![remove, unpin_pack(&looping)], None).unwrap();
    s.apply("Pack", vec![pin_pack(&looping)], None).unwrap();
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
    s.apply("Pack", vec![pin_pack(&pack)], None).unwrap();
    assert_that!(s.registry.get(&kind_of(&pack, "quote"))).is_none();
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
    assert_that!(s.registry.get(&kind_of(&pack, "quote"))).is_some();
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
    s.apply("Pack", vec![pin_pack(&pack)], None).unwrap();
    let registry = s.registry.clone();
    // Un fichier reste à sonder ; l'installation des widgets sondés échoue.
    let quote = std::collections::BTreeSet::from([oku_slides_core::WidgetFile::parse("quote").unwrap()]);
    s.widgets_unprobed.insert(crate::test_support::namespace_of(pack.id), quote);
    s.failpoints.install_widgets = true;
    let theme = crate::test_support::set_theme_op("pro");
    s.apply("Thème", vec![theme], None).unwrap();
    // Les widgets installés restent ; la sonde sera reprise au rendu suivant.
    assert_that!(Arc::ptr_eq(&s.registry, &registry)).is_true();
    assert_that!(s.widgets_unprobed.is_empty()).is_false();
    assert_registry_agrees(&s);
    s.rerender().unwrap();
    assert_that!(s.widgets_unprobed.is_empty()).is_true();
}

// ---- Repli et retour arrière, repli qui échoue, repli en historique ------------------

#[test]
fn apply_that_falls_back_then_fails_to_commit_restores_the_widgets_without_the_notice() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    let looping = user_pack(&s, "Boucle", &[("boucle", LOOP_ON_RENDER)]);
    let (registry, pins) = (s.registry.clone(), s.widgets_pins.clone());
    s.doc.notice = None;
    // Le rendu boucle (pack écarté, avec son avis), puis l'inscription dans les sources échoue.
    s.failpoints.sources_apply = true;
    assert_that!(s.apply("Pack", vec![pin_pack(&looping), add_loop(&looping)], None).is_err()).is_true();
    assert_that!(Arc::ptr_eq(&s.registry, &registry)).is_true();
    assert_that!(s.widgets_pins).is_equal_to(pins);
    assert_that!(s.doc.notice).is_none();
    assert_registry_agrees(&s);
}

#[test]
fn render_whose_fall_back_fails_returns_the_render_error() {
    let data = data_dir();
    let (mut s, looping) = session_on_looping_pack(&data.0);
    // Le rendu boucle, puis l'installation sans le pack fautif échoue.
    s.failpoints.install_widgets = true;
    let err = s.apply("Boucle", vec![add_loop(&looping)], None).unwrap_err();
    let kind = err.render_error().map(oku_slides_render::RenderError::kind);
    assert_that!(kind).is_equal_to(Some(oku_slides_render::RenderErrorKind::Timeout));
}

#[test]
fn history_open_after_the_pack_faulted_elsewhere_falls_back_and_shows_the_notice() {
    let data = data_dir();
    let (mut s, looping) = session_on_looping_pack(&data.0);
    fault_pack_outside_the_deck(&s, &looping);
    s.doc.notice = None;
    s.history_open().unwrap();
    // L'avis est sur le document montré, pas sur le deck mis de côté.
    assert_fell_back(&s, &looping);
    s.history_close().unwrap();
    assert_that!(s.packs.packs().iter().all(is_trusted_pack)).is_true();
    assert_registry_agrees(&s);
}

/// La version montrée utilise le pack devenu fautif (la dernière, celle du
/// pack `Boucle`) : son rendu se replie, l'avis suit le document montré.
///
/// t-355 : l'aperçu se rend avec le pack de sa propre version ; la version 0
/// (pack par défaut) n'utilise pas `Boucle` et ne peut plus servir ici (voir
/// le test suivant).
#[test]
fn history_seek_after_the_pack_faulted_elsewhere_keeps_the_notice() {
    let data = data_dir();
    let (mut s, looping) = session_on_looping_pack(&data.0);
    s.history_open().unwrap();
    fault_pack_outside_the_deck(&s, &looping);
    s.doc.notice = None;
    let last = s.history_versions().unwrap().len() - 1;
    s.history_seek(last).unwrap();
    assert_fell_back(&s, &looping);
}

/// t-355 : une version d'un autre pack se montre sans l'avis du pack fautif,
/// qu'elle n'utilise pas ; en sortant, le deck (qui l'utilise) se replie et le dit.
#[test]
fn history_seek_to_a_version_without_the_faulted_pack_then_close_says_it_on_the_deck() {
    let data = data_dir();
    let (mut s, looping) = session_on_looping_pack(&data.0);
    s.history_open().unwrap();
    fault_pack_outside_the_deck(&s, &looping);
    s.doc.notice = None;
    s.history_seek(0).unwrap();
    assert_that!(s.packs.packs().iter().all(is_trusted_pack)).is_true();
    assert_that!(s.doc.notice.clone().unwrap_or_default().as_str()).does_not_contain("Boucle");
    s.history_close().unwrap();
    assert_fell_back(&s, &looping);
}

// ---- Repli par pack : un pack fautif est écarté seul ---------------------------------

/// Le widget d'élément `note` : valide.
const GOOD_NOTE: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("note", params: (body: p.plain()), (args, ctx) => [#args.body])
"#;

/// Le widget d'élément `note` qui boucle au rendu.
fn looping_note() -> String {
    LOOP_ON_RENDER.replace("\"boucle\"", "\"note\"")
}

/// Une session au délai court dont le deck épingle, pour chaque `(espace de noms,
/// besoins, source)`, un pack de la bibliothèque au widget `note`, et porte un
/// élément `<namespace>/note` de chacun sur `s-constat`.
fn session_with_packs(data: &Path, packs: &[(&str, &[&str], &str)]) -> Session {
    let mut s = session_with_timeout(data, LOOP_TIMEOUT_MS);
    let mut ops = Vec::new();
    for (namespace, requires, source) in packs {
        let m = crate::test_support::manifest_ns(uuid::Uuid::new_v4(), namespace, requires);
        let pin = crate::test_support::add_version(&s.library, &m, &[("note", source)]);
        ops.push(crate::test_support::add_pack(&m, &pin));
    }
    for (namespace, ..) in packs {
        ops.push(op(json!({
            "op": "add_element",
            "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
            "element": { "id": format!("n-{namespace}"), "kind": format!("{namespace}/note"), "params": { "body": "x" } }
        })));
    }
    s.doc.notice = None;
    s.apply("Packs", ops, None).unwrap();
    s
}

/// Les widgets `note` au registre de `s`, par espace de noms.
fn notes_of(s: &Session, namespaces: &[&str]) -> Vec<bool> {
    namespaces.iter().map(|n| s.registry.get(&format!("{n}/note")).is_some()).collect()
}

#[test]
fn faulty_pack_is_disabled_alone() {
    let data = data_dir();
    let looping = looping_note();
    let s = session_with_packs(&data.0, &[("good", &[], GOOD_NOTE), ("bad", &[], &looping)]);
    assert_that!(notes_of(&s, &["good", "bad"])).is_equal_to(vec![true, false]);
    // Les épinglages restent ; l'élément du pack écarté reste aussi.
    assert_that!(s.deck().widgets.len()).is_equal_to(3);
    assert_that!(s.deck().slide("s-constat").is_some_and(|sl| sl.locate("n-bad").is_some())).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn pack_requiring_a_faulty_pack_is_disabled_too() {
    let data = data_dir();
    let looping = looping_note();
    let packs: &[(&str, &[&str], &str)] =
        &[("good", &[], GOOD_NOTE), ("bad", &[], &looping), ("top", &["bad"], GOOD_NOTE)];
    let s = session_with_packs(&data.0, packs);
    assert_that!(notes_of(&s, &["good", "bad", "top"])).is_equal_to(vec![true, false, false]);
    assert_registry_agrees(&s);
}

#[test]
fn disabled_packs_notice_names_each_pack() {
    let data = data_dir();
    let looping = looping_note();
    let packs: &[(&str, &[&str], &str)] =
        &[("good", &[], GOOD_NOTE), ("bad", &[], &looping), ("top", &["bad"], GOOD_NOTE)];
    let s = session_with_packs(&data.0, packs);
    let expected = s.localizer.tr_with("session-packs-disabled", &[("packs", "bad, top".into())]);
    let notice = s.doc.notice.clone().unwrap_or_default();
    assert_that!(notice.as_str()).contains(expected.as_str());
    assert_that!(notice.as_str()).does_not_contain("good");
}

#[test]
fn faulty_pack_found_by_bisection_is_disabled_in_the_following_resolutions() {
    let data = data_dir();
    let looping = looping_note();
    let mut s = session_with_packs(&data.0, &[("bad", &[], &looping), ("good", &[], GOOD_NOTE)]);
    // Annuler puis rétablir résout de nouveau : le pack fautif reste écarté, sans nouveau rendu en échec.
    s.undo().unwrap();
    s.redo().unwrap();
    assert_that!(notes_of(&s, &["good", "bad"])).is_equal_to(vec![true, false]);
    assert_registry_agrees(&s);
}

/// Le rendu témoin sans pack échoue aussi (simulé : aucun deck du noyau seul ne dépasse le
/// délai dans les tests) : l'échec est propre au deck, aucun pack n'est accusé ni écarté.
#[test]
fn bounded_failure_without_any_pack_blames_none() {
    let data = data_dir();
    let looping = looping_note();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    let mut ops = Vec::new();
    let mut packs = Vec::new();
    for (namespace, source) in [("good", GOOD_NOTE), ("bad", looping.as_str())] {
        let m = crate::test_support::manifest_ns(uuid::Uuid::new_v4(), namespace, &[]);
        let pin = crate::test_support::add_version(&s.library, &m, &[("note", source)]);
        ops.push(crate::test_support::add_pack(&m, &pin));
        let files = crate::test_support::widget_files(&[("note", source)]);
        let kind = oku_slides_core::PackKind::Widgets;
        packs.push(oku_slides_render::PackWidgets::new(pin.version.hex(), m.namespace.clone(), kind, files).unwrap());
    }
    ops.push(op(json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "n-bad", "kind": "bad/note", "params": { "body": "x" } }
    })));
    s.failpoints.control_render = true;
    let err = s.apply("Packs", ops, None).unwrap_err();
    assert_that!(err.render_error().is_some_and(|e| e.kind().is_bounded())).is_true();
    assert_that!(s.failpoints.control_render).is_false();
    assert_that!(packs.iter().any(|p| s.worker.is_faulty(p))).is_false();
    assert_that!(s.widgets_disabled.is_empty()).is_true();
}
