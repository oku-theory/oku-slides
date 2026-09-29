use std::path::{Path, PathBuf};
use std::sync::Arc;

use oku_slides_core::Op;
use serde_json::json;
use speculoos::prelude::*;

use crate::{Fonts, Platform, Request, Session, SessionErrorKind, Settings};

/// Le pack `quote` du plan 1 : un widget d'élément, avec un corps riche.
const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let quote = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
#let registry = (quote,)
"#;

/// Un pack qui ne compile pas.
const BROKEN: &str = "#let registry = (";

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

fn fixture_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

/// Écrit le pack `id` de l'utilisateur, sans modèles, avec `widgets` pour `widgets.typ`.
fn write_pack(data: &Path, id: &str, widgets: &str) {
    let dir = data.join("templates").join(id);
    std::fs::create_dir_all(&dir).unwrap();
    let set = json!({ "id": id, "name": format!("Pack {id}"), "templates": [] });
    std::fs::write(dir.join("pack.json"), serde_json::to_vec(&set).unwrap()).unwrap();
    std::fs::write(dir.join("widgets.typ"), widgets).unwrap();
}

fn op(v: serde_json::Value) -> Op {
    serde_json::from_value(v).unwrap()
}

fn set_templates(id: Option<&str>) -> Op {
    op(json!({ "op": "set_templates", "templates": id }))
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
    write_pack(&data.0, "citations", QUOTE);
    let mut s = session_in(&data.0);
    assert_that!(widgets_have_quote(&mut s)).is_false();
    s.apply("Pack", vec![set_templates(Some("citations"))], None).unwrap();
    assert_that!(widgets_have_quote(&mut s)).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn apply_set_templates_then_pack_element_in_one_transaction_succeeds() {
    let data = data_dir();
    write_pack(&data.0, "citations", QUOTE);
    let mut s = session_in(&data.0);
    s.apply("Citation", vec![set_templates(Some("citations")), add_quote()], None).unwrap();
    assert_that!(has_quote(&s)).is_true();
    let page = s.slide("s-constat").unwrap();
    assert_that!(page.targets.iter().any(|t| t.id == "q")).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn apply_pack_element_without_its_pack_is_refused() {
    let data = data_dir();
    write_pack(&data.0, "citations", QUOTE);
    let mut s = session_in(&data.0);
    let err = s.apply("Citation", vec![add_quote()], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Op);
    assert_that!(has_quote(&s)).is_false();
}

#[test]
fn apply_set_templates_with_render_failure_keeps_the_previous_registry() {
    let data = data_dir();
    write_pack(&data.0, "citations", QUOTE);
    let mut s = session_in(&data.0);
    let base = s.registry.clone();
    let bad = op(json!({ "op": "set_token", "path": ["color", "accent"], "value": 5 }));
    let err = s.apply("Pack", vec![set_templates(Some("citations")), bad], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Render);
    assert_that!(s.deck().templates).is_none();
    assert_that!(Arc::ptr_eq(&s.registry, &base)).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn apply_set_templates_with_sources_failure_keeps_the_previous_registry() {
    let data = data_dir();
    write_pack(&data.0, "citations", QUOTE);
    let mut s = session_in(&data.0);
    let base = s.registry.clone();
    s.failpoints.sources_apply = true;
    assert_that!(s.apply("Pack", vec![set_templates(Some("citations"))], None).is_err()).is_true();
    assert_that!(s.deck().templates).is_none();
    assert_that!(Arc::ptr_eq(&s.registry, &base)).is_true();
    assert_that!(widgets_have_quote(&mut s)).is_false();
    assert_registry_agrees(&s);
}

#[test]
fn undo_and_redo_of_set_templates_follow_the_deck_pack() {
    let data = data_dir();
    write_pack(&data.0, "citations", QUOTE);
    let mut s = session_in(&data.0);
    s.apply("Citation", vec![set_templates(Some("citations")), add_quote()], None).unwrap();
    s.undo().unwrap();
    assert_that!(s.deck().templates).is_none();
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
    write_pack(&data.0, "citations", QUOTE);
    let mut s = session_in(&data.0);
    let ours = data.0.join("nous.oku-slides.pdf");
    s.save(Some(&ours)).unwrap();
    // Un autre pair du même deck choisit le pack et y ajoute une citation.
    let mut other = session_in(&data.0);
    other.open(&ours).unwrap();
    other.apply("Citation", vec![set_templates(Some("citations")), add_quote()], None).unwrap();
    let theirs = data.0.join("eux.oku-slides.pdf");
    other.save(Some(&theirs)).unwrap();
    s.merge_file(&theirs).unwrap();
    assert_that!(s.deck().templates.as_deref()).is_equal_to(Some("citations"));
    assert_that!(has_quote(&s)).is_true();
    assert_that!(widgets_have_quote(&mut s)).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn history_restore_before_set_templates_returns_to_the_base_registry() {
    let data = data_dir();
    write_pack(&data.0, "citations", QUOTE);
    let mut s = session_in(&data.0);
    s.apply("Pack", vec![set_templates(Some("citations"))], None).unwrap();
    s.history_open().unwrap();
    let before = s.history_versions().unwrap().len() - 2;
    s.history_seek(before).unwrap();
    s.history_restore().unwrap();
    assert_that!(s.deck().templates).is_none();
    assert_that!(widgets_have_quote(&mut s)).is_false();
    assert_registry_agrees(&s);
}

#[test]
fn open_deck_with_a_broken_pack_keeps_the_base_registry_and_sets_a_notice() {
    let data = data_dir();
    write_pack(&data.0, "casse", BROKEN);
    let mut s = session_in(&data.0);
    let base = s.registry.clone();
    let mut deck: serde_json::Value = serde_json::from_slice(&std::fs::read(fixture_path()).unwrap()).unwrap();
    deck["templates"] = json!("casse");
    let path = data.0.join("casse.json");
    std::fs::write(&path, serde_json::to_vec(&deck).unwrap()).unwrap();
    s.open(&path).unwrap();
    assert_that!(s.deck().templates.as_deref()).is_equal_to(Some("casse"));
    assert_that!(Arc::ptr_eq(&s.registry, &base)).is_true();
    let notice = s.doc.notice.clone().unwrap_or_default();
    assert_that!(notice.as_str()).contains(s.localizer.tr("registry-compile").as_str());
    assert_registry_agrees(&s);
}

#[test]
fn apply_set_templates_to_a_broken_pack_keeps_the_base_registry_and_sets_a_notice() {
    let data = data_dir();
    write_pack(&data.0, "casse", BROKEN);
    let mut s = session_in(&data.0);
    let base = s.registry.clone();
    s.apply("Pack", vec![set_templates(Some("casse"))], None).unwrap();
    assert_that!(s.deck().templates.as_deref()).is_equal_to(Some("casse"));
    assert_that!(Arc::ptr_eq(&s.registry, &base)).is_true();
    assert_that!(s.doc.notice.is_some()).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn load_with_a_failed_registry_install_keeps_the_previous_document() {
    let data = data_dir();
    write_pack(&data.0, "citations", QUOTE);
    let mut s = session_in(&data.0);
    s.apply("Citation", vec![set_templates(Some("citations")), add_quote()], None).unwrap();
    let before = s.deck().clone();
    s.failpoints.install_widgets = true;
    // La fixture n'a pas de pack : son registre, la base, ne s'installe pas.
    assert_that!(s.open(&fixture_path()).is_err()).is_true();
    assert_that!(s.deck()).is_equal_to(&before);
    assert_that!(widgets_have_quote(&mut s)).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn edit_templates_of_a_pack_with_widgets_uses_them_then_close_returns_to_the_deck_registry() {
    let data = data_dir();
    write_pack(&data.0, "citations", QUOTE);
    let mut s = session_in(&data.0);
    s.edit_templates("citations").unwrap();
    assert_that!(widgets_have_quote(&mut s)).is_true();
    assert_registry_agrees(&s);
    s.close_templates().unwrap();
    assert_that!(widgets_have_quote(&mut s)).is_false();
    assert_registry_agrees(&s);
}

#[test]
fn save_templates_as_a_copy_takes_the_pack_widgets_along() {
    let data = data_dir();
    write_pack(&data.0, "citations", QUOTE);
    let mut s = session_in(&data.0);
    s.edit_templates("citations").unwrap();
    s.save_templates(true).unwrap();
    let copy = s.snapshot().editing.unwrap().set.clone();
    assert_that!(copy.as_str()).is_not_equal_to("citations");
    assert_that!(s.library.widgets(&copy)).is_equal_to(Some(QUOTE.to_owned()));
    assert_that!(widgets_have_quote(&mut s)).is_true();
    assert_registry_agrees(&s);
}
