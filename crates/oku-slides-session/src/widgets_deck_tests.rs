use oku_slides_core::{Op, WidgetFile};
use serde_json::json;
use speculoos::prelude::*;

use crate::SessionErrorKind;
use crate::pack::{DEFAULT_PACK, PackManifest};
use crate::test_support::{fresh_dir, library_of, opened_on};

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

const QUOTE_V2: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", strong(args.body)))
"#;

fn quote() -> WidgetFile {
    WidgetFile::parse("quote").unwrap()
}

#[test]
fn set_widget_twice_in_deck_mode_makes_one_current_version_and_one_undo_step() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let before = s.deck().templates.clone().unwrap();
    s.set_widget("quote", QUOTE.into()).unwrap();
    s.set_widget("quote", QUOTE_V2.into()).unwrap();
    let after = s.deck().templates.clone().unwrap();
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
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(before));
    assert_that!(library_of(&s).version(after.id, &after.version).is_some()).is_true();
}

#[test]
fn a_non_widget_operation_ends_the_widget_session() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    s.set_widget("quote", QUOTE.into()).unwrap();
    let first = s.deck().templates.clone().unwrap();
    s.apply("Thème", vec![Op::SetTheme { theme: Some("pro".into()) }], None).unwrap();
    s.set_widget("quote", QUOTE_V2.into()).unwrap();
    let second = s.deck().templates.clone().unwrap();
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
    s.handle(crate::Request::SetWidget { name: "quote".into(), source: QUOTE.into() }).unwrap();
    let first = s.deck().templates.clone().unwrap();
    s.handle(crate::Request::SetWidget { name: "quote".into(), source: QUOTE_V2.into() }).unwrap();
    // Même séance : la version est remplacée, de même parente.
    let replaced = s.deck().templates.clone().unwrap();
    assert_that!(library_of(&s).version(first.id, &first.version).is_some()).is_false();
    s.handle(crate::Request::Undo).unwrap();
    s.handle(crate::Request::Redo).unwrap();
    s.set_widget("quote", QUOTE.into()).unwrap();
    let manifest = PackManifest::from_files(&library_of(&s).current_version(replaced.id).unwrap().files).unwrap();
    assert_that!(manifest.parent).is_equal_to(Some(replaced.version));
}

#[test]
fn a_widget_set_in_deck_mode_can_be_added_to_the_deck() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    s.set_widget("quote", QUOTE.into()).unwrap();
    let add: Op = serde_json::from_value(json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "q", "kind": "quote", "params": { "body": [{ "runs": [{ "t": "Tout coule." }] }] } }
    }))
    .unwrap();
    s.apply("Citation", vec![add], None).unwrap();
    assert_that!(s.deck().slide("s-constat").is_some_and(|sl| sl.locate("q").is_some())).is_true();
}

#[test]
fn set_widget_that_breaks_in_deck_mode_posts_no_broken_widgets_notice() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let change = s.set_widget("quote", "#let widget = (".into()).unwrap();
    assert_that!(change.status.is_valid()).is_false();
    assert_that!(s.doc.notice).is_none();
}

#[test]
fn widget_reads_on_a_deck_without_pack_use_the_default_pack() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let mut deck = s.deck().clone();
    deck.templates = None;
    s.doc.sources.apply_silent(deck).unwrap();
    assert_that!(s.widget_status()).is_ok().is_empty();
    let err = s.widget_source("quote").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWidget);
    // Une écriture fait passer le deck à une version du pack par défaut.
    s.set_widget("quote", QUOTE.into()).unwrap();
    let r = s.deck().templates.clone().unwrap();
    assert_that!(r.id).is_equal_to(DEFAULT_PACK);
    assert_that!(s.widget_source("quote").unwrap()).is_equal_to(QUOTE.to_owned());
}

/// Le corps échoue au rendu ; le schéma, lui, se lit.
const QUOTE_FAILING: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => panic("corps fautif"))
"#;

/// Une session dont le deck a un élément `quote`, en séance de widgets.
fn deck_using_quote(data: &std::path::Path) -> crate::Session {
    let mut s = opened_on(data);
    s.set_widget("quote", QUOTE.into()).unwrap();
    let add: Op = serde_json::from_value(json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "q", "kind": "quote", "params": { "body": [{ "runs": [{ "t": "Tout coule." }] }] } }
    }))
    .unwrap();
    s.apply("Citation", vec![add], None).unwrap();
    s
}

#[test]
fn set_widget_whose_body_fails_to_render_changes_nothing_and_returns_diagnostics() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = deck_using_quote(data.path());
    s.set_widget("quote", QUOTE_V2.into()).unwrap();
    let r = s.deck().templates.clone().unwrap();
    let versions = library_of(&s).versions(r.id).len();
    let packs = s.doc.sources.packs.len();
    let change = s.set_widget("quote", QUOTE_FAILING.into()).unwrap();
    assert_that!(change.status.is_valid()).is_false();
    assert_that!(change.status.diagnostics.iter().any(|d| d.error && d.message.contains("corps fautif"))).is_true();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(r.clone()));
    let lib = library_of(&s);
    assert_that!(lib.versions(r.id).len()).is_equal_to(versions);
    assert_that!(lib.current(r.id)).is_equal_to(Some(r.version.clone()));
    assert_that!(s.doc.sources.packs.len()).is_equal_to(packs);
    assert_that!(s.widget_source("quote").unwrap()).is_equal_to(QUOTE_V2.to_owned());
    // La séance continue : la prochaine écriture remplace sa version.
    s.set_widget("quote", QUOTE.into()).unwrap();
    assert_that!(library_of(&s).version(r.id, &r.version).is_some()).is_false();
}

#[test]
fn set_widget_with_the_same_source_in_deck_mode_commits_nothing() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    s.set_widget("quote", QUOTE.into()).unwrap();
    let r = s.deck().templates.clone().unwrap();
    let versions = library_of(&s).versions(r.id).len();
    s.doc.dirty = false;
    let change = s.set_widget("quote", QUOTE.into()).unwrap();
    assert_that!(change.status.is_valid()).is_true();
    assert_that!(s.doc.dirty).is_false();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(r.clone()));
    assert_that!(library_of(&s).versions(r.id).len()).is_equal_to(versions);
}

#[test]
fn successive_writes_in_deck_mode_keep_only_the_session_version_in_the_sources() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let before = s.deck().templates.clone().unwrap();
    let packs = s.doc.sources.packs.len();
    s.set_widget("quote", QUOTE.into()).unwrap();
    s.set_widget("quote", QUOTE_V2.into()).unwrap();
    s.set_widget("quote", QUOTE.into()).unwrap();
    // La parente et la version de la séance, pas les intermédiaires.
    assert_that!(s.doc.sources.packs.len()).is_equal_to(packs + 1);
    s.undo().unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(before));
    s.redo().unwrap();
    assert_that!(s.widget_source("quote").unwrap()).is_equal_to(QUOTE.to_owned());
}

#[test]
fn undo_after_a_write_in_an_agent_turn_then_one_outside_finds_the_turn_version() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    s.begin_turn("crée une citation");
    s.set_widget("quote", QUOTE.into()).unwrap();
    s.end_turn();
    let turn = s.deck().templates.clone().unwrap();
    s.set_widget("quote", QUOTE_V2.into()).unwrap();
    assert_that!(s.deck().templates.clone()).is_not_equal_to(Some(turn.clone()));
    s.undo().unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(turn.clone()));
    assert_that!(s.doc.sources.packs.contains_key(&turn.version)).is_true();
    assert_that!(library_of(&s).version(turn.id, &turn.version).is_some()).is_true();
    assert_that!(s.registry.get("quote").is_some()).is_true();
    assert_that!(s.doc.notice).is_none();
}

#[test]
fn two_writes_in_one_agent_turn_keep_only_the_last_version() {
    let data = fresh_dir("oku-slides-widgets-deck-");
    let mut s = opened_on(data.path());
    let before = s.deck().templates.clone().unwrap();
    let packs = s.doc.sources.packs.len();
    s.begin_turn("crée une citation");
    s.set_widget("quote", QUOTE.into()).unwrap();
    s.set_widget("quote", QUOTE_V2.into()).unwrap();
    s.end_turn();
    // Même étape (le groupe du tour) : l'intermédiaire part.
    assert_that!(s.doc.sources.packs.len()).is_equal_to(packs + 1);
    s.undo().unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(before));
}
