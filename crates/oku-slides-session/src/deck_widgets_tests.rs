use std::path::{Path, PathBuf};
use std::sync::Arc;

use oku_slides_core::{Op, PackRef, SourceBundle};
use serde_json::json;
use speculoos::prelude::*;

use crate::{Fonts, Platform, Request, Session, SessionErrorKind, Settings};

/// Le fichier `quote` : un widget d'élément, avec un corps riche.
const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

/// Un fichier `casse` qui ne compile pas.
const BROKEN: &str = "#let widget = (";

/// Le fichier `boucle` : boucle au rendu, hors des gardes de Typst.
///
/// Boucles imbriquées : `range(1e12)` alloue au lieu de tourner.
const LOOP_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("boucle", params: (body: p.plain()),
  (args, ctx) => { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; [#n] })
"#;

/// Le fichier `boucle` : boucle dès son chargement (sonde).
const LOOP_ON_IMPORT: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let _spin = { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; n }
#let widget = widget("boucle", params: (body: p.plain()), (args, ctx) => [])
"#;

/// Délai de rendu des tests de packs qui bouclent : bien plus qu'un rendu
/// normal de la fixture, pour que la base ne dépasse jamais.
const LOOP_TIMEOUT_MS: u64 = 2000;

/// Un dossier de données neuf, supprimé à la fin du test.
struct Data(PathBuf);

impl Drop for Data {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn data_dir() -> Data {
    Data(crate::test_support::fresh_dir("oku-slides-packs-").keep())
}

/// Une session sur la fixture ; ses modèles sont dans `data/templates`.
fn session_in(data: &Path) -> Session {
    let mut settings = Settings::defaults().unwrap();
    settings.paths.data = Some(data.to_owned());
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    let mut s = Session::new(Fonts::default(), &Platform::with_settings(settings).unwrap()).unwrap();
    s.open(&fixture_path()).unwrap();
    s
}

/// Comme `session_in`, avec un délai de rendu de `timeout_ms`.
fn session_with_timeout(data: &Path, timeout_ms: u64) -> Session {
    let mut settings = Settings::defaults().unwrap();
    settings.paths.data = Some(data.to_owned());
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    settings.render.timeout_ms = timeout_ms;
    let mut s = Session::new(Fonts::default(), &Platform::with_settings(settings).unwrap()).unwrap();
    s.open(&fixture_path()).unwrap();
    s
}

fn fixture_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

/// Ajoute à la bibliothèque de `s` un pack de l'utilisateur, avec ses fichiers `widgets` ; sa référence.
fn user_pack(s: &Session, name: &str, widgets: &[(&str, &str)]) -> PackRef {
    let m = crate::test_support::manifest(uuid::Uuid::new_v4(), name, None, "2026-10-01T10:00:00Z");
    crate::test_support::add_version(&s.library, &m, widgets)
}

fn op(v: serde_json::Value) -> Op {
    serde_json::from_value(v).unwrap()
}

fn set_templates(r: Option<&PackRef>) -> Op {
    Op::SetTemplates { templates: r.cloned() }
}

/// Une citation ajoutée aux blocs de `s-constat`.
fn add_quote() -> Op {
    op(json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": {
            "id": "q",
            "kind": "quote",
            "params": { "body": [{ "runs": [{ "t": "Tout coule." }] }], "source": "Héraclite" }
        }
    }))
}

/// Un élément `boucle` ajouté aux blocs de `s-constat`.
fn add_loop() -> Op {
    op(json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "l", "kind": "boucle", "params": { "body": "x" } }
    }))
}

/// La requête `widgets` connaît-elle le widget `quote` ? (La base a une
/// mise en page `quote`, pas de widget de ce nom.)
fn widgets_have_quote(s: &mut Session) -> bool {
    let reply: serde_json::Value = serde_json::from_str(&s.handle(Request::Widgets).unwrap()).unwrap();
    reply["widgets"].get("quote").is_some()
}

/// Le registre du deck, celui de ses sources et celui du rendu sont le même.
fn assert_registry_agrees(s: &Session) {
    assert_that!(std::ptr::eq(s.doc.sources.registry(), &*s.registry)).is_true();
    assert_that!(Arc::ptr_eq(s.renderer.registry(), &s.registry)).is_true();
}

fn has_quote(s: &Session) -> bool {
    s.deck().slide("s-constat").is_some_and(|slide| slide.locate("q").is_some())
}

#[test]
fn session_with_pack_widgets_uses_the_pack_registry() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let quote = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    assert_that!(widgets_have_quote(&mut s)).is_false();
    s.apply("Pack", vec![set_templates(Some(&quote))], None).unwrap();
    assert_that!(widgets_have_quote(&mut s)).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn apply_set_templates_then_pack_element_in_one_transaction_succeeds() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let quote = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    s.apply("Citation", vec![set_templates(Some(&quote)), add_quote()], None).unwrap();
    assert_that!(has_quote(&s)).is_true();
    let page = s.slide("s-constat").unwrap();
    assert_that!(page.targets.iter().any(|t| t.id == "q")).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn apply_pack_element_without_its_pack_is_refused() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    user_pack(&s, "Citation", &[("quote", QUOTE)]);
    let err = s.apply("Citation", vec![add_quote()], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Op);
    assert_that!(has_quote(&s)).is_false();
}

#[test]
fn apply_set_templates_with_render_failure_keeps_the_previous_registry() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let quote = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    let (base, before) = (s.registry.clone(), s.deck().templates.clone());
    let bad = op(json!({ "op": "set_token", "path": ["color", "accent"], "value": 5 }));
    let err = s.apply("Pack", vec![set_templates(Some(&quote)), bad], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Render);
    assert_that!(s.deck().templates.clone()).is_equal_to(before);
    assert_that!(Arc::ptr_eq(&s.registry, &base)).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn apply_set_templates_with_sources_failure_keeps_the_previous_registry() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let quote = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    let (base, before) = (s.registry.clone(), s.deck().templates.clone());
    s.failpoints.sources_apply = true;
    assert_that!(s.apply("Pack", vec![set_templates(Some(&quote))], None).is_err()).is_true();
    assert_that!(s.deck().templates.clone()).is_equal_to(before);
    assert_that!(Arc::ptr_eq(&s.registry, &base)).is_true();
    assert_that!(widgets_have_quote(&mut s)).is_false();
    assert_registry_agrees(&s);
}

#[test]
fn undo_and_redo_of_set_templates_follow_the_deck_pack() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let quote = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    let before = s.deck().templates.clone();
    s.apply("Citation", vec![set_templates(Some(&quote)), add_quote()], None).unwrap();
    s.undo().unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(before);
    assert_that!(widgets_have_quote(&mut s)).is_false();
    assert_registry_agrees(&s);
    s.redo().unwrap();
    assert_that!(has_quote(&s)).is_true();
    assert_that!(widgets_have_quote(&mut s)).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn merge_file_that_brings_a_pack_uses_its_registry() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let quote = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    let ours = data.0.join("nous.oku-slides.pdf");
    s.save(Some(&ours)).unwrap();
    // Un autre pair du même deck choisit le pack et y ajoute une citation.
    let mut other = session_in(&data.0);
    other.open(&ours).unwrap();
    other.apply("Citation", vec![set_templates(Some(&quote)), add_quote()], None).unwrap();
    let theirs = data.0.join("eux.oku-slides.pdf");
    other.save(Some(&theirs)).unwrap();
    s.merge_file(&theirs).unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(quote.clone()));
    assert_that!(s.doc.sources.packs.contains_key(&quote.version)).is_true();
    assert_that!(has_quote(&s)).is_true();
    assert_that!(widgets_have_quote(&mut s)).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn history_restore_before_set_templates_returns_to_the_base_registry() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let quote = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    let before_pack = s.deck().templates.clone();
    s.apply("Pack", vec![set_templates(Some(&quote))], None).unwrap();
    s.history_open().unwrap();
    let before = s.history_versions().unwrap().len() - 2;
    s.history_seek(before).unwrap();
    s.history_restore().unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(before_pack);
    assert_that!(widgets_have_quote(&mut s)).is_false();
    assert_registry_agrees(&s);
}

/// Les sources de la fixture, qui embarquent un pack `name` de widgets `widgets` et y renvoient.
fn bundle_with_pack(s: &Session, name: &str, widgets: &[(&str, &str)]) -> (SourceBundle, PackRef) {
    let m = crate::test_support::manifest(uuid::Uuid::new_v4(), name, None, "2026-10-01T10:00:00Z");
    let files = m.to_files(crate::test_support::widget_files(widgets)).unwrap();
    let r = PackRef { id: m.id, version: files.version() };
    let mut deck = s.deck().clone();
    deck.templates = Some(r.clone());
    let mut sources = SourceBundle::new(deck, s.registry.clone()).unwrap();
    sources.add_pack(files);
    (sources, r)
}

#[test]
fn load_sources_with_a_broken_pack_keeps_the_base_registry_and_sets_a_notice() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let base = s.registry.clone();
    let (sources, broken) = bundle_with_pack(&s, "Cassé", &[("casse", BROKEN)]);
    s.load(sources, None).unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(broken));
    assert_that!(Arc::ptr_eq(&s.registry, &base)).is_true();
    let notice = s.doc.notice.clone().unwrap_or_default();
    assert_that!(notice.as_str()).contains("casse");
    assert_registry_agrees(&s);
}

/// Le pack embarqué fait foi, même absent de la bibliothèque.
#[test]
fn load_sources_with_an_embedded_pack_unknown_to_the_library_uses_its_widgets() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let (sources, quote) = bundle_with_pack(&s, "Citation", &[("quote", QUOTE)]);
    s.load(sources, None).unwrap();
    assert_that!(s.library.current(quote.id)).is_none();
    assert_that!(widgets_have_quote(&mut s)).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn apply_set_templates_to_a_broken_pack_keeps_the_base_registry_and_sets_a_notice() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let broken = user_pack(&s, "Cassé", &[("casse", BROKEN)]);
    let base = s.registry.clone();
    s.apply("Pack", vec![set_templates(Some(&broken))], None).unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(broken));
    assert_that!(Arc::ptr_eq(&s.registry, &base)).is_true();
    assert_that!(s.doc.notice.is_some()).is_true();
    assert_registry_agrees(&s);
}

/// t-329 : un widget cassé est écarté, les autres du même pack restent.
#[test]
fn apply_set_templates_to_a_pack_with_one_broken_widget_keeps_the_others() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let mixed = user_pack(&s, "Mixte", &[("quote", QUOTE), ("casse", BROKEN)]);
    s.apply("Pack", vec![set_templates(Some(&mixed)), add_quote()], None).unwrap();
    assert_that!(widgets_have_quote(&mut s)).is_true();
    assert_that!(has_quote(&s)).is_true();
    assert_that!(s.doc.notice.clone().unwrap_or_default().as_str()).contains("casse");
    assert_registry_agrees(&s);
}

#[test]
fn load_with_a_failed_registry_install_keeps_the_previous_document() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let quote = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    s.apply("Citation", vec![set_templates(Some(&quote)), add_quote()], None).unwrap();
    let before = s.deck().clone();
    s.failpoints.install_widgets = true;
    // La fixture prend le pack par défaut, sans widgets : son registre, la base, ne s'installe pas.
    assert_that!(s.open(&fixture_path()).is_err()).is_true();
    assert_that!(s.deck()).is_equal_to(&before);
    assert_that!(widgets_have_quote(&mut s)).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn edit_templates_of_a_pack_with_widgets_uses_them_then_close_returns_to_the_deck_registry() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let quote = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    s.edit_templates(quote.id).unwrap();
    assert_that!(widgets_have_quote(&mut s)).is_true();
    assert_registry_agrees(&s);
    s.close_templates().unwrap();
    assert_that!(widgets_have_quote(&mut s)).is_false();
    assert_registry_agrees(&s);
}

#[test]
fn save_templates_as_a_copy_takes_the_pack_widgets_along() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let quote = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    s.edit_templates(quote.id).unwrap();
    s.save_templates(true).unwrap();
    let copy = s.snapshot().editing.unwrap().set;
    assert_that!(copy).is_not_equal_to(quote.id);
    let stored = s.library.current_version(copy).unwrap();
    assert_that!(stored.files.widget(&oku_slides_core::WidgetFile::parse("quote").unwrap())).is_equal_to(Some(QUOTE));
    assert_that!(widgets_have_quote(&mut s)).is_true();
    assert_registry_agrees(&s);
}

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

/// Le cache fait foi : une autre version courante ne change pas le rendu.
#[test]
fn render_uses_the_embedded_version_even_when_the_library_has_another() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let quote = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    s.apply("Pack", vec![set_templates(Some(&quote)), add_quote()], None).unwrap();
    let sans = crate::test_support::manifest(quote.id, "Citation", Some(&quote.version), "2026-10-02T10:00:00Z");
    let plain = crate::test_support::add_version(&s.library, &sans, &[]);
    s.library.set_current(quote.id, &plain.version).unwrap();
    let path = data.0.join("decks").join("cache.oku-slides.pdf");
    s.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap();
    let mut other = session_in(&data.0);
    other.open(&path).unwrap();
    assert_that!(widgets_have_quote(&mut other)).is_true();
    assert_that!(other.deck().templates.clone()).is_equal_to(Some(quote));
}

#[test]
fn version_absent_from_the_sources_renders_base_widgets_with_a_notice() {
    let data = data_dir();
    let path = data.0.join("absent.json");
    let deck = json!({ "version": 3, "templates": { "id": uuid::Uuid::new_v4(), "version": format!("sha256:{}", "0".repeat(64)) },
        "slides": [{ "id": "s", "kind": "title" }] });
    std::fs::write(&path, serde_json::to_vec(&deck).unwrap()).unwrap();
    let mut s = session_in(&data.0);
    s.open(&path).unwrap();
    let snap: serde_json::Value = serde_json::from_str(&s.handle(Request::Snapshot).unwrap()).unwrap();
    assert_that!(snap["notice"].as_str().unwrap_or_default()).contains("manque dans le fichier");
    assert_that!(widgets_have_quote(&mut s)).is_false();
}

#[test]
fn pack_without_widgets_renders_base_widgets_without_a_notice() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let m = crate::test_support::manifest(uuid::Uuid::new_v4(), "Sans", None, "2026-10-01T10:00:00Z");
    let r = crate::test_support::add_version(&s.library, &m, &[]);
    s.handle(Request::Snapshot).unwrap();
    s.apply("Pack", vec![set_templates(Some(&r))], None).unwrap();
    let snap: serde_json::Value = serde_json::from_str(&s.handle(Request::Snapshot).unwrap()).unwrap();
    assert_that!(snap["notice"].is_null()).is_true();
}

/// L'historique restaure aussi le pack d'une version passée.
#[test]
fn history_restore_brings_back_the_pack_of_that_version() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let quote = user_pack(&s, "Citation", &[("quote", QUOTE)]);
    s.apply("Pack", vec![set_templates(Some(&quote)), add_quote()], None).unwrap();
    let plain = crate::test_support::add_version(
        &s.library,
        &crate::test_support::manifest(uuid::Uuid::new_v4(), "Sans", None, "2026-10-01T10:00:00Z"),
        &[],
    );
    s.apply("Autre", vec![set_templates(Some(&plain))], None).unwrap();
    assert_that!(widgets_have_quote(&mut s)).is_false();
    s.history_open().unwrap();
    let count = s.history_versions().unwrap().len();
    s.history_seek(count - 2).unwrap();
    s.history_restore().unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(quote));
    assert_that!(widgets_have_quote(&mut s)).is_true();
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

#[test]
fn broken_widgets_notice_lists_the_widgets_but_not_common() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let framed = r#"#import "/oku-slides/lib.typ": widget, params as p
#import "_commun.typ": cadre
#let widget = widget("cadre", params: (body: p.plain()), (args, ctx) => cadre(args.body))
"#;
    let broken = user_pack(&s, "Commun", &[("_commun", "#let cadre = ("), ("cadre", framed), ("quote", QUOTE)]);
    s.apply("Pack", vec![set_templates(Some(&broken))], None).unwrap();
    let notice = s.doc.notice.clone().unwrap_or_default();
    assert_that!(notice.as_str()).contains("cadre");
    assert_that!(notice.as_str()).does_not_contain("_commun");
}
