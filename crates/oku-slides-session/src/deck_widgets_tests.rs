use std::path::{Path, PathBuf};
use std::sync::Arc;

use oku_slides_core::{Op, PackRef, SourceBundle};
use serde_json::json;
use speculoos::prelude::*;

use crate::{Fonts, Platform, Request, Session, SessionErrorKind, Settings};

/// Le fichier `quote` : un widget d'élément, avec un corps riche.
pub(super) const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

/// Un fichier `casse` qui ne compile pas.
pub(super) const BROKEN: &str = "#let widget = (";

/// Le fichier `boucle` : boucle au rendu, hors des gardes de Typst.
///
/// Boucles imbriquées : `range(1e12)` alloue au lieu de tourner.
pub(super) const LOOP_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("boucle", params: (body: p.plain()),
  (args, ctx) => { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; [#n] })
"#;

/// Le fichier `boucle` : boucle dès son chargement (sonde).
pub(super) const LOOP_ON_IMPORT: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let _spin = { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; n }
#let widget = widget("boucle", params: (body: p.plain()), (args, ctx) => [])
"#;

/// Délai de rendu des tests de packs qui bouclent : bien plus qu'un rendu
/// normal de la fixture, pour que la base ne dépasse jamais.
pub(super) const LOOP_TIMEOUT_MS: u64 = 2000;

/// Un dossier de données neuf, supprimé à la fin du test.
pub(super) struct Data(pub(super) PathBuf);

impl Drop for Data {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

pub(super) fn data_dir() -> Data {
    Data(crate::test_support::fresh_dir("oku-slides-packs-").keep())
}

/// Une session sur la fixture ; ses modèles sont dans `data/templates`.
pub(super) fn session_in(data: &Path) -> Session {
    let mut settings = Settings::defaults().unwrap();
    settings.paths.data = Some(data.to_owned());
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    let mut s = Session::new(Fonts::default(), &Platform::with_settings(settings).unwrap()).unwrap();
    s.open(&fixture_path()).unwrap();
    s
}

/// Comme `session_in`, avec un délai de rendu de `timeout_ms`.
pub(super) fn session_with_timeout(data: &Path, timeout_ms: u64) -> Session {
    let mut settings = Settings::defaults().unwrap();
    settings.paths.data = Some(data.to_owned());
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    settings.render.timeout_ms = timeout_ms;
    settings.render.probe_budget_ms = timeout_ms;
    let mut s = Session::new(Fonts::default(), &Platform::with_settings(settings).unwrap()).unwrap();
    s.open(&fixture_path()).unwrap();
    s
}

pub(super) fn fixture_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

/// Ajoute à la bibliothèque de `s` un pack de l'utilisateur, avec ses fichiers `widgets` ; sa référence.
pub(super) fn user_pack(s: &Session, name: &str, widgets: &[(&str, &str)]) -> PackRef {
    let m = crate::test_support::manifest(uuid::Uuid::new_v4(), name, None, "2026-10-01T10:00:00Z");
    crate::test_support::add_version(&s.library, &m, widgets)
}

pub(super) fn op(v: serde_json::Value) -> Op {
    serde_json::from_value(v).unwrap()
}

pub(super) fn set_templates(r: Option<&PackRef>) -> Op {
    Op::SetTemplates { templates: r.cloned() }
}

/// Une citation ajoutée aux blocs de `s-constat`.
pub(super) fn add_quote() -> Op {
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
pub(super) fn add_loop() -> Op {
    op(json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "l", "kind": "boucle", "params": { "body": "x" } }
    }))
}

/// La requête `widgets` connaît-elle le widget `quote` ? (La base a une
/// mise en page `quote`, pas de widget de ce nom.)
pub(super) fn widgets_have_quote(s: &mut Session) -> bool {
    let reply: serde_json::Value = serde_json::from_str(&s.handle(Request::Widgets).unwrap()).unwrap();
    reply["widgets"].get("quote").is_some()
}

/// Le registre du deck, celui de ses sources et celui du rendu sont le même.
pub(super) fn assert_registry_agrees(s: &Session) {
    assert_that!(std::ptr::eq(s.doc.sources.registry(), &*s.registry)).is_true();
    // Un emplacement pas encore ouvert le sera avec les widgets installés.
    assert_that!(s.renderer.as_ref().is_none_or(|r| Arc::ptr_eq(r.registry(), &s.registry))).is_true();
}

pub(super) fn has_quote(s: &Session) -> bool {
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
    let bad = op(json!({ "op": "set_token", "path": ["size", "body"], "value": true }));
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
    let deck = json!({ "version": 4, "templates": { "id": uuid::Uuid::new_v4(), "version": format!("sha256:{}", "0".repeat(64)) },
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
