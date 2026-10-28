use oku_slides_core::{Namespace, Op, PackKind, WidgetFile};
use serde_json::json;
use speculoos::prelude::*;

use crate::SessionErrorKind;
use crate::library::draft_lock;
use crate::pack::{PackManifest, PackScope};
use crate::test_support::{
    add_pack, add_version, adopt_deck_only_pack, fresh_dir, library_of, manifest, namespace_of, ns, opened_on, pin,
    rename_title, touch_elsewhere, widget_files,
};

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

const QUOTE_V2: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", strong(args.body)))
"#;

/// Un widget de slide des tests : un titre seul.
const SLIDE_WIDGET_SRC: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("cover", category: "slide", params: (title: p.plain()),
  (args, ctx) => field(ctx, "title", args.title))
"#;

fn quote() -> WidgetFile {
    WidgetFile::parse("quote").unwrap()
}

/// L'espace de noms du pack de widgets intégré, que ces tests modifient en mode deck.
fn ess() -> oku_slides_core::Namespace {
    ns("base")
}

#[test]
fn set_widget_twice_in_deck_mode_makes_one_current_version_and_one_undo_step() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let before = pin(&s, "base").unwrap();
    let change = s.set_widget(Some(&ess()), "quote", QUOTE.into()).unwrap();
    assert_that!(serde_json::to_value(&change).unwrap()["saved"].clone()).is_equal_to(json!(true));
    s.set_widget(Some(&ess()), "quote", QUOTE_V2.into()).unwrap();
    let after = pin(&s, "base").unwrap();
    assert_that!(after.id).is_equal_to(before.id);
    let lib = library_of(&s);
    assert_that!(lib.current(after.id)).is_equal_to(Some(after.version.clone()));
    let files = lib.version(after.id, &after.version).unwrap();
    assert_that!(files.widget(&quote())).is_equal_to(Some(QUOTE_V2));
    let manifest = PackManifest::from_files(&files).unwrap();
    assert_that!(manifest.parent).is_equal_to(Some(before.version.clone()));
    // Une seule étape : annuler rend la version d'avant, la bibliothèque garde la nouvelle.
    assert_that!(s.snapshot().undo_label.unwrap_or_default().as_str()).starts_with("Widgets");
    s.undo().unwrap();
    assert_that!(pin(&s, "base")).is_equal_to(Some(before));
    assert_that!(library_of(&s).version(after.id, &after.version).is_some()).is_true();
}

#[test]
fn a_non_widget_operation_ends_the_widget_session() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    s.set_widget(Some(&ess()), "quote", QUOTE.into()).unwrap();
    let first = pin(&s, "base").unwrap();
    s.apply("Thème", vec![crate::test_support::set_theme_op("pro")], None).unwrap();
    s.set_widget(Some(&ess()), "quote", QUOTE_V2.into()).unwrap();
    let second = pin(&s, "base").unwrap();
    assert_that!(second.version).is_not_equal_to(first.version.clone());
    let lib = library_of(&s);
    let manifest = PackManifest::from_files(&lib.version(second.id, &second.version).unwrap()).unwrap();
    assert_that!(manifest.parent).is_equal_to(Some(first.version.clone()));
    assert_that!(lib.version(first.id, &first.version).is_some()).is_true();
}

#[test]
fn a_non_widget_request_ends_the_widget_session_but_a_widget_request_does_not() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    s.handle(crate::Request::SetWidget { pack: Some(ess()), name: "quote".into(), source: QUOTE.into() }).unwrap();
    let first = pin(&s, "base").unwrap();
    s.handle(crate::Request::SetWidget { pack: Some(ess()), name: "quote".into(), source: QUOTE_V2.into() }).unwrap();
    // Même séance : la version est remplacée, de même parente.
    let replaced = pin(&s, "base").unwrap();
    assert_that!(library_of(&s).version(first.id, &first.version).is_some()).is_false();
    s.handle(crate::Request::Undo).unwrap();
    s.handle(crate::Request::Redo).unwrap();
    s.set_widget(Some(&ess()), "quote", QUOTE.into()).unwrap();
    let manifest = PackManifest::from_files(&library_of(&s).current_version(replaced.id).unwrap().files).unwrap();
    assert_that!(manifest.parent).is_equal_to(Some(replaced.version));
}

#[test]
fn a_widget_set_in_deck_mode_can_be_added_to_the_deck() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    s.set_widget(Some(&ess()), "quote", QUOTE.into()).unwrap();
    let add: Op = serde_json::from_value(json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "q", "kind": "base/quote", "params": { "body": [{ "runs": [{ "t": "Tout coule." }] }] } }
    }))
    .unwrap();
    s.apply("Citation", vec![add], None).unwrap();
    assert_that!(s.deck().slide("s-constat").is_some_and(|sl| sl.locate("q").is_some())).is_true();
}

#[test]
fn set_widget_that_breaks_in_deck_mode_posts_no_broken_widgets_notice() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let change = s.set_widget(Some(&ess()), "quote", "#let widget = (".into()).unwrap();
    assert_that!(change.status.is_valid()).is_false();
    assert_that!(s.doc.notice).is_none();
}

#[test]
fn set_widget_without_pack_creates_the_deck_pack() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let base = pin(&s, "base");
    // Sans pack `deck` : aucun widget à lire, aucun pack créé.
    assert_that!(s.widget_status(None)).is_ok().is_empty();
    let err = s.widget_source(None, "quote").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWidget);
    assert_that!(pin(&s, "deck")).is_none();
    // La première écriture crée le pack propre du deck, du deck seul, sans parente.
    s.set_widget(None, "note", QUOTE.replace("\"quote\"", "\"note\"")).unwrap();
    let created = pin(&s, "deck").unwrap();
    assert_that!(s.registry.get("deck/note").is_some()).is_true();
    let manifest = PackManifest::from_files(&s.doc.sources.packs[&created.version]).unwrap();
    assert_that!((manifest.scope, manifest.parent)).is_equal_to((PackScope::Deck, None));
    assert_that!(library_of(&s).versions(created.id).is_empty()).is_true();
    // Le pack intégré du deck ne change pas.
    assert_that!(pin(&s, "base")).is_equal_to(base);
    // Annuler retire le pack créé.
    s.undo().unwrap();
    assert_that!(pin(&s, "deck")).is_none();
}

#[test]
fn set_widget_into_named_pack_makes_a_new_version_of_it() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let before = pin(&s, "base").unwrap().version;
    s.set_widget(Some(&ess()), "other", QUOTE.replace("\"quote\"", "\"other\"")).unwrap();
    assert_that!(pin(&s, "base").unwrap().version).is_not_equal_to(before);
    assert_that!(pin(&s, "deck")).is_none();
}

#[test]
fn set_widget_into_a_namespace_the_deck_does_not_pin_is_refused() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let err = s.set_widget(Some(&ns("absent")), "quote", QUOTE.into()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownNamespace);
}

/// Le corps échoue au rendu ; le schéma, lui, se lit.
const QUOTE_FAILING: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => panic("corps fautif"))
"#;

/// Une session dont le deck a un élément `quote`, en séance de widgets.
fn deck_using_quote(data: &std::path::Path) -> crate::Session {
    let mut s = opened_on(data);
    s.set_widget(Some(&ess()), "quote", QUOTE.into()).unwrap();
    let add: Op = serde_json::from_value(json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "q", "kind": "base/quote", "params": { "body": [{ "runs": [{ "t": "Tout coule." }] }] } }
    }))
    .unwrap();
    s.apply("Citation", vec![add], None).unwrap();
    s
}

#[test]
fn set_widget_whose_body_fails_to_render_changes_nothing_and_returns_diagnostics() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = deck_using_quote(data.path());
    s.set_widget(Some(&ess()), "quote", QUOTE_V2.into()).unwrap();
    let r = pin(&s, "base").unwrap();
    let versions = library_of(&s).versions(r.id).len();
    let packs = s.doc.sources.packs.len();
    let change = s.set_widget(Some(&ess()), "quote", QUOTE_FAILING.into()).unwrap();
    assert_that!(change.saved).is_false();
    assert_that!(serde_json::to_value(&change).unwrap()["saved"].clone()).is_equal_to(json!(false));
    assert_that!(change.status.is_valid()).is_false();
    assert_that!(change.status.diagnostics.iter().any(|d| d.error && d.message.contains("corps fautif"))).is_true();
    assert_that!(pin(&s, "base")).is_equal_to(Some(r.clone()));
    let lib = library_of(&s);
    assert_that!(lib.versions(r.id).len()).is_equal_to(versions);
    assert_that!(lib.current(r.id)).is_equal_to(Some(r.version.clone()));
    assert_that!(s.doc.sources.packs.len()).is_equal_to(packs);
    assert_that!(s.widget_source(Some(&ess()), "quote").unwrap()).is_equal_to(QUOTE_V2.to_owned());
    // La séance continue : la prochaine écriture remplace sa version.
    s.set_widget(Some(&ess()), "quote", QUOTE.into()).unwrap();
    assert_that!(library_of(&s).version(r.id, &r.version).is_some()).is_false();
}

#[test]
fn set_widget_with_the_same_source_in_deck_mode_commits_nothing() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    s.set_widget(Some(&ess()), "quote", QUOTE.into()).unwrap();
    let r = pin(&s, "base").unwrap();
    let versions = library_of(&s).versions(r.id).len();
    s.doc.dirty = false;
    let change = s.set_widget(Some(&ess()), "quote", QUOTE.into()).unwrap();
    assert_that!(change.status.is_valid()).is_true();
    assert_that!(s.doc.dirty).is_false();
    assert_that!(pin(&s, "base")).is_equal_to(Some(r.clone()));
    assert_that!(library_of(&s).versions(r.id).len()).is_equal_to(versions);
}

#[test]
fn successive_writes_in_deck_mode_keep_only_the_session_version_in_the_sources() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let before = pin(&s, "base").unwrap();
    let packs = s.doc.sources.packs.len();
    s.set_widget(Some(&ess()), "quote", QUOTE.into()).unwrap();
    s.set_widget(Some(&ess()), "quote", QUOTE_V2.into()).unwrap();
    s.set_widget(Some(&ess()), "quote", QUOTE.into()).unwrap();
    // La parente et la version de la séance, pas les intermédiaires.
    assert_that!(s.doc.sources.packs.len()).is_equal_to(packs + 1);
    s.undo().unwrap();
    assert_that!(pin(&s, "base")).is_equal_to(Some(before));
    s.redo().unwrap();
    assert_that!(s.widget_source(Some(&ess()), "quote").unwrap()).is_equal_to(QUOTE.to_owned());
}

#[test]
fn undo_after_a_write_in_an_agent_turn_then_one_outside_finds_the_turn_version() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    s.begin_turn(1, "crée une citation");
    s.set_widget(Some(&ess()), "quote", QUOTE.into()).unwrap();
    s.end_turn();
    let turn = pin(&s, "base").unwrap();
    s.set_widget(Some(&ess()), "quote", QUOTE_V2.into()).unwrap();
    assert_that!(pin(&s, "base")).is_not_equal_to(Some(turn.clone()));
    s.undo().unwrap();
    assert_that!(pin(&s, "base")).is_equal_to(Some(turn.clone()));
    assert_that!(s.doc.sources.packs.contains_key(&turn.version)).is_true();
    assert_that!(library_of(&s).version(turn.id, &turn.version).is_some()).is_true();
    assert_that!(s.registry.get("base/quote").is_some()).is_true();
    assert_that!(s.doc.notice).is_none();
}

#[test]
fn two_writes_in_one_agent_turn_keep_only_the_last_version() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let before = pin(&s, "base").unwrap();
    let packs = s.doc.sources.packs.len();
    s.begin_turn(1, "crée une citation");
    s.set_widget(Some(&ess()), "quote", QUOTE.into()).unwrap();
    s.set_widget(Some(&ess()), "quote", QUOTE_V2.into()).unwrap();
    s.end_turn();
    // Même étape (le groupe du tour) : l'intermédiaire part.
    assert_that!(s.doc.sources.packs.len()).is_equal_to(packs + 1);
    s.undo().unwrap();
    assert_that!(pin(&s, "base")).is_equal_to(Some(before));
}

#[test]
fn set_widget_in_deck_mode_counts_no_pack_update() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    s.set_widget(Some(&ess()), "quote", QUOTE.into()).unwrap();
    let r = pin(&s, "base").unwrap();
    assert_that!(library_of(&s).current(r.id)).is_equal_to(Some(r.version.clone()));
    assert_that!(s.snapshot().pack_updates).is_equal_to(0);
    s.set_widget(Some(&ess()), "quote", QUOTE_V2.into()).unwrap();
    assert_that!(s.snapshot().pack_updates).is_equal_to(0);
}

/// `Save` de l'interface (sans `overwrite`), dans `path`.
fn save(path: &std::path::Path) -> crate::Request {
    crate::Request::Save { path: Some(path.to_owned()), overwrite: false }
}

/// `SetWidget` de l'interface, dans le pack `essentiel`.
fn set(source: &str) -> crate::Request {
    crate::Request::SetWidget { pack: Some(ess()), name: "quote".into(), source: source.into() }
}

#[test]
fn a_save_between_two_widget_writes_keeps_the_widget_session() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let path = data.path().join("deck.oku-slides.pdf");
    s.handle(save(&path)).unwrap();
    let before = pin(&s, "base").unwrap();
    let versions = library_of(&s).versions(before.id).len();
    s.handle(set(QUOTE)).unwrap();
    s.handle(save(&path)).unwrap();
    s.handle(set(QUOTE_V2)).unwrap();
    s.handle(save(&path)).unwrap();
    // Une seule version nouvelle (l'intermédiaire remplacée), une seule étape.
    assert_that!(library_of(&s).versions(before.id).len()).is_equal_to(versions + 1);
    s.handle(crate::Request::Undo).unwrap();
    assert_that!(pin(&s, "base")).is_equal_to(Some(before));
}

#[test]
fn a_save_that_merges_a_change_on_disk_ends_the_widget_session() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let path = data.path().join("deck.oku-slides.pdf");
    s.handle(save(&path)).unwrap();
    s.handle(set(QUOTE)).unwrap();
    let first = pin(&s, "base").unwrap();
    touch_elsewhere(&path);
    s.handle(save(&path)).unwrap();
    s.handle(set(QUOTE_V2)).unwrap();
    let second = pin(&s, "base").unwrap();
    let lib = library_of(&s);
    assert_that!(lib.version(first.id, &first.version).is_some()).is_true();
    let manifest = PackManifest::from_files(&lib.version(second.id, &second.version).unwrap()).unwrap();
    assert_that!(manifest.parent).is_equal_to(Some(first.version));
}

#[cfg(unix)]
#[test]
fn a_library_write_failure_after_the_render_posts_a_notice_and_keeps_the_deck_version() {
    use std::os::unix::fs::PermissionsExt;

    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    // Bibliothèque en lecture seule : la version ne peut pas y être écrite.
    let templates = data.path().join("lecture-seule");
    std::fs::create_dir(&templates).unwrap();
    s.set_pack_dirs(&templates, data.path().join("layouts")).unwrap();
    std::fs::set_permissions(&templates, std::fs::Permissions::from_mode(0o555)).unwrap();
    let change = s.set_widget(Some(&ess()), "quote", QUOTE.into());
    std::fs::set_permissions(&templates, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_that!(change.unwrap().status.is_valid()).is_true();
    let r = pin(&s, "base").unwrap();
    assert_that!(s.doc.sources.packs.contains_key(&r.version)).is_true();
    assert_that!(library_of(&s).version(r.id, &r.version)).is_none();
    assert_that!(s.doc.notice.as_deref().unwrap_or_default()).contains("bibliothèque");
}

const KPI: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (value: p.plain()), (args, ctx) => field(ctx, "value", args.value))
"#;

#[test]
fn undo_and_redo_of_a_widget_session_in_deck_mode_follow_the_registry() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    assert_that!(s.registry.get("base/kpi")).is_none();
    s.set_widget(Some(&ess()), "kpi", KPI.into()).unwrap();
    assert_that!(s.registry.get("base/kpi")).is_some();
    s.undo().unwrap();
    assert_that!(s.registry.get("base/kpi")).is_none();
    assert_that!(std::ptr::eq(s.doc.sources.registry(), &raw const *s.registry)).is_true();
    s.redo().unwrap();
    assert_that!(s.registry.get("base/kpi")).is_some();
    assert_that!(std::ptr::eq(s.doc.sources.registry(), &raw const *s.registry)).is_true();
}

#[test]
fn rename_and_remove_requests_in_deck_mode_make_one_version_and_one_undo_step() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let before = pin(&s, "base").unwrap();
    let versions = library_of(&s).versions(before.id).len();
    s.handle(crate::Request::SetWidget { pack: Some(ess()), name: "kpi".into(), source: KPI.into() }).unwrap();
    s.handle(crate::Request::RenameWidget { pack: Some(ess()), from: "kpi".into(), to: "indicateur".into() }).unwrap();
    assert_that!(s.widget_source(Some(&ess()), "indicateur").unwrap().as_str()).contains("widget(\"indicateur\"");
    assert_that!(s.registry.get("base/indicateur")).is_some();
    s.handle(crate::Request::RemoveWidget { pack: Some(ess()), name: "indicateur".into() }).unwrap();
    assert_that!(s.widget_status(Some(&ess())).unwrap().iter().any(|w| w.name.as_str() == "indicateur")).is_false();
    // Une seule version de séance, de parente la version d'avant.
    let r = pin(&s, "base").unwrap();
    let lib = library_of(&s);
    assert_that!(lib.versions(before.id).len()).is_equal_to(versions + 1);
    let manifest = PackManifest::from_files(&lib.version(r.id, &r.version).unwrap()).unwrap();
    assert_that!(manifest.parent).is_equal_to(Some(before.version.clone()));
    // Une seule étape : annuler rend la version d'avant.
    s.handle(crate::Request::Undo).unwrap();
    assert_that!(pin(&s, "base")).is_equal_to(Some(before));
}

#[test]
fn set_widget_on_a_deck_only_pack_embeds_its_versions_and_leaves_the_library_unchanged() {
    let data = fresh_dir("oku-slides-widgets-deck-only-");
    let mut s = opened_on(data.path());
    let before = adopt_deck_only_pack(&mut s);
    let packs = library_of(&s).packs(PackKind::Widgets);
    s.set_widget(None, "quote", QUOTE.into()).unwrap();
    let change = s.set_widget(None, "quote", QUOTE_V2.into()).unwrap();
    assert_that!(change.saved).is_true();
    let after = pin(&s, "deck").unwrap();
    assert_that!(after.id).is_equal_to(before.id);
    assert_that!(library_of(&s).packs(PackKind::Widgets)).is_equal_to(&packs);
    assert_that!(library_of(&s).versions(after.id).is_empty()).is_true();
    let files = &s.doc.sources.packs[&after.version];
    assert_that!(files.widget(&quote())).is_equal_to(Some(QUOTE_V2));
    let manifest = PackManifest::from_files(files).unwrap();
    assert_that!(manifest.scope).is_equal_to(PackScope::Deck);
    assert_that!(manifest.parent).is_equal_to(Some(before.version.clone()));
    assert_that!(s.doc.notice).is_none();
    assert_that!(s.pack_updates()).is_equal_to(0);
    s.undo().unwrap();
    assert_that!(pin(&s, "deck")).is_equal_to(Some(before));
}

#[test]
fn remove_widget_in_use_names_the_qualified_kind() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = deck_using_quote(data.path());
    let err = s.remove_widget(Some(&ess()), "quote").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::WidgetInUse);
    let shown = oku_slides_i18n::Localize::localize(&err, &s.localizer);
    assert_that!(shown.as_str()).contains("base/quote");
    assert_that!(s.widget_source(Some(&ess()), "quote").unwrap()).is_equal_to(QUOTE.to_owned());
}

#[test]
fn remove_widget_used_by_a_deck_slide_names_the_slide_in_neutral_words() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = deck_using_quote(data.path());
    let slide = s.doc.sources.deck().slides.iter().find(|sl| sl.id == "s-constat").unwrap();
    let shown = slide.id.clone();
    let err = s.remove_widget(Some(&ess()), "quote").unwrap_err();
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    let expected =
        fr.tr_with("session-widget-in-use", &[("name", "base/quote".into()), ("uses", shown.as_str().into())]);
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr)).is_equal_to(expected);
    let en = oku_slides_i18n::Localizer::new(Some("en"));
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr))
        .is_not_equal_to(oku_slides_i18n::Localize::localize(&err, &en));
}

#[test]
fn set_widget_without_pack_on_a_library_pack_pinned_under_deck_is_refused() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    // Un pack de la bibliothèque sous `deck`, tel qu'un deck reçu pourrait l'embarquer.
    let m = crate::test_support::manifest_ns(uuid::Uuid::new_v4(), "deck", &[]);
    let files = crate::test_support::files_of(&m);
    let library_pin = oku_slides_core::PackPin { id: m.id, version: files.version(), requires: Vec::new() };
    s.doc.sources.add_pack(files);
    s.apply("Pack", vec![crate::test_support::add_pack(&m, &library_pin)], None).unwrap();
    let packs = library_of(&s).packs(PackKind::Widgets);
    let err = s.set_widget(None, "quote", QUOTE.into()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NamespaceReserved);
    assert_that!(pin(&s, "deck")).is_equal_to(Some(library_pin));
    assert_that!(library_of(&s).packs(PackKind::Widgets)).is_equal_to(&packs);
}

#[test]
fn deck_widget_write_of_a_slide_widget_creates_deck_layouts() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    s.set_widget(None, "cover", SLIDE_WIDGET_SRC.into()).unwrap();
    let pinned =
        (s.deck().layouts.contains_key(&Namespace::deck_layouts()), s.deck().widgets.contains_key(&Namespace::deck()));
    assert_that!(pinned).is_equal_to((true, false));
    assert_that!(s.registry.get("deck-layouts/cover").map(|w| w.category))
        .is_equal_to(Some(oku_slides_core::Category::Slide));
}

#[test]
fn deck_widget_write_of_a_slide_widget_into_a_widget_pack_is_refused() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let before = pin(&s, "base");
    let err = s.set_widget(Some(&ess()), "cover", SLIDE_WIDGET_SRC.into()).unwrap_err();
    let fault = err.registry_error().and_then(|e| e.widget_error()).map(oku_slides_core::WidgetError::kind);
    assert_that!(fault)
        .is_equal_to(Some(oku_slides_core::WidgetErrorKind::BadSchema(oku_slides_core::SchemaFault::PackCategory)));
    assert_that!(pin(&s, "base")).is_equal_to(before);
}

#[test]
fn two_widget_writes_in_one_session_keep_one_version_that_stays_a_draft() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let id = pin(&s, "base").unwrap().id;
    s.set_widget(Some(&ess()), "quote", QUOTE.into()).unwrap();
    let first = pin(&s, "base").unwrap().version;
    let count = library_of(&s).versions(id).len();
    s.set_widget(Some(&ess()), "quote", QUOTE_V2.into()).unwrap();
    let second = pin(&s, "base").unwrap().version;
    assert_that!(library_of(&s).versions(id).len()).is_equal_to(count);
    assert_that!(library_of(&s).version(id, &first).is_none()).is_true();
    assert_that!(library_of(&s).is_frozen(id, &second)).is_false();
}

#[test]
fn ending_the_widget_session_freezes_the_version_the_deck_pins() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    s.set_widget(Some(&ess()), "quote", QUOTE.into()).unwrap();
    let pinned = pin(&s, "base").unwrap();
    assert_that!(library_of(&s).is_frozen(pinned.id, &pinned.version)).is_false();
    s.apply("Nom", vec![rename_title("Un")], None).unwrap();
    assert_that!(library_of(&s).is_frozen(pinned.id, &pinned.version)).is_true();
}

#[test]
fn a_version_adopted_by_another_deck_mid_session_gets_a_child_not_a_replacement() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    s.set_widget(Some(&ess()), "quote", QUOTE.into()).unwrap();
    let first = pin(&s, "base").unwrap();
    let files = library_of(&s).version(first.id, &first.version).unwrap();
    library_of(&s).adopt(&draft_lock(), first.id, &files).unwrap();
    s.set_widget(Some(&ess()), "quote", QUOTE_V2.into()).unwrap();
    let second = pin(&s, "base").unwrap();
    assert_that!(library_of(&s).version(first.id, &first.version).is_some()).is_true();
    let written = library_of(&s).version(second.id, &second.version).unwrap();
    assert_that!(PackManifest::from_files(&written).unwrap().parent).is_equal_to(Some(first.version.clone()));
    assert_that!(library_of(&s).current(second.id)).is_equal_to(Some(second.version));
}

/// A draft `v1` the deck pins without `adopt_pins` having run is frozen by the deck-mode widget write on top of it.
///
/// `adopt_pins` is the first safety (every public pin path runs it); this covers its failure (t-1224).
#[test]
fn deck_mode_widget_write_freezes_an_unfrozen_parent() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let id = uuid::Uuid::new_v4();
    let m0 = manifest(id, "P", None, "2026-10-24T10:00:00Z");
    let pin0 = add_version(&s.library, &m0, &[("quote", QUOTE)]);
    s.apply("Pack", vec![add_pack(&m0, &pin0)], None).unwrap();
    // A draft `v1` the library lists, current and not frozen.
    let m1 = manifest(id, "P", Some(&pin0.version), "2026-10-24T11:00:00Z");
    let files1 = m1.to_files(oku_slides_core::PackKind::Widgets, widget_files(&[("quote", QUOTE_V2)])).unwrap();
    let v1 = files1.version();
    s.library.save_draft(&draft_lock(), id, &crate::library::DraftWrite::New, &files1).unwrap();
    // The deck pins it through its sources: no `refresh_pack_updates`, so no `adopt_pins`.
    s.doc.sources.add_pack(files1);
    let mut deck = s.doc.sources.deck().clone();
    deck.widgets.insert(namespace_of(id), oku_slides_core::PackPin { id, version: v1.clone(), requires: vec![] });
    s.doc.sources.apply_silent(deck).unwrap();
    assert_that!(library_of(&s).is_frozen(id, &v1)).is_false();
    s.set_widget(Some(&namespace_of(id)), "quote", QUOTE.into()).unwrap();
    assert_that!(library_of(&s).is_frozen(id, &v1)).is_true();
    let written = pin(&s, namespace_of(id).as_str()).unwrap();
    assert_that!(library_of(&s).is_frozen(id, &written.version)).is_false();
}
