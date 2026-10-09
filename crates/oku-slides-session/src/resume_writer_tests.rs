use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::agent::{AgentScope, SavedConversation};

use oku_slides_core::{Deck, SourceBundle};
use speculoos::prelude::*;

use super::*;
use crate::DocDefaults;
use crate::Document;
use crate::resume::{DECK_DIR, DocSnapshot, SavedMode, State, VERSION};
use crate::test_support::registry;
use crate::views::UiState;

/// Réglages de test : 10 étapes d'annulation, sans auteur réglé.
fn defaults() -> DocDefaults {
    DocDefaults { undo_limit: 10, author: None }
}

fn temp_dir() -> PathBuf {
    crate::test_support::fresh_dir("oku-slides-writer-").keep()
}

fn resume(dir: &std::path::Path) -> Resume {
    Resume::new(oku_slides_platform::storage::Storage::local().unwrap(), dir.to_owned())
}

fn document(theme: &str) -> Document {
    let mut deck = Deck::from_json(r#"{"slides":[]}"#).unwrap();
    deck.theme = crate::test_support::theme_ref("papier");
    let mut doc = Document::new(SourceBundle::new(deck.clone(), registry()).unwrap(), None, &defaults());
    let mut next = deck;
    next.theme = crate::test_support::theme_ref(theme);
    doc.sources.apply(next, "Thème", None).unwrap();
    doc
}

fn state(slide: &str) -> State {
    State {
        version: VERSION,
        mode: SavedMode::Deck,
        ui: UiState { slide: Some(slide.into()), grid: true, ghosts: false, ..UiState::default() },
        help_theme: None,
        theme_edit: None,
    }
}

fn job(theme: Option<&str>, slide: &str) -> Job {
    let docs = theme.map(|t| vec![(DECK_DIR, DocSnapshot::of(&document(t), &mut Vec::new()).unwrap())]);
    Job { docs: docs.unwrap_or_default(), state: state(slide), agent: BTreeMap::new() }
}

/// Après `flush`, tout ce qui a été envoyé est sur le disque.
#[test]
fn writer_flush_waits_for_the_sent_job() {
    let dir = temp_dir();
    let w = Writer::new(resume(&dir));
    w.send(job(Some("pro"), "s-1"));
    w.flush();
    let doc = resume(&dir).read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).unwrap();
    assert_that!(doc.sources.deck().theme.as_ref().and_then(crate::test_support::theme_name)).is_equal_to(Some("pro"));
    std::fs::remove_dir_all(&dir).ok();
}

/// Des envois rapprochés se fondent : le dernier état l'emporte, et un
/// envoi sans documents n'efface pas les documents d'un envoi précédent.
#[test]
fn writer_merged_jobs_keep_the_latest_documents_and_state() {
    let dir = temp_dir();
    let w = Writer::new(resume(&dir));
    for i in 0..50 {
        w.send(job(Some(if i % 2 == 0 { "pro" } else { "papier" }), "s-0"));
    }
    w.send(job(Some("pro"), "s-1"));
    w.send(job(None, "s-2"));
    w.flush();
    let r = resume(&dir);
    assert_that!(
        r.read_doc(DECK_DIR, &defaults(), 1 << 24, &registry())
            .unwrap()
            .sources
            .deck()
            .theme
            .as_ref()
            .and_then(crate::test_support::theme_name)
    )
    .is_equal_to(Some("pro"));
    assert_that!(r.read_state().unwrap().unwrap().ui.slide.as_deref()).is_equal_to(Some("s-2"));
    std::fs::remove_dir_all(&dir).ok();
}

/// Rendre le fil (fermeture de l'onglet, fin de la session) écrit d'abord
/// ce qui reste : rien n'est perdu, et plus rien n'est écrit ensuite.
#[test]
fn writer_drop_writes_pending_jobs_first() {
    let dir = temp_dir();
    let w = Writer::new(resume(&dir));
    w.send(job(Some("pro"), "s-9"));
    drop(w);
    assert_that!(resume(&dir).read_state().unwrap().unwrap().ui.slide.as_deref()).is_equal_to(Some("s-9"));
    std::fs::remove_dir_all(&dir).ok();
}

/// Images et polices inchangées : partagées d'un instantané au suivant,
/// jamais recopiées ; une police changée sous le même nom est recopiée.
#[test]
fn doc_snapshot_shares_unchanged_blobs_and_copies_changed_fonts() {
    let mut doc = document("pro");
    doc.sources.add_asset(vec![1; 1000]);
    doc.sources.fonts.insert("A.ttf".into(), vec![2; 1000]);
    let mut cache = Vec::new();
    let first = DocSnapshot::of(&doc, &mut cache).unwrap();
    let second = DocSnapshot::of(&doc, &mut cache).unwrap();
    assert_that!(std::sync::Arc::ptr_eq(first.blobs(), second.blobs())).is_true();
    doc.sources.fonts.insert("A.ttf".into(), vec![3; 1000]);
    let third = DocSnapshot::of(&doc, &mut cache).unwrap();
    assert_that!(std::sync::Arc::ptr_eq(second.blobs(), third.blobs())).is_false();
    assert_that!(third.blobs().fonts["A.ttf"][0]).is_equal_to(3);
}

/// La reprise garde le deck et son identité, pas la pile d'annulation.
#[test]
fn read_doc_after_write_keeps_deck_and_uid_but_not_undo() {
    let dir = temp_dir();
    let r = resume(&dir);
    let doc = document("pro");
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    let back = r.read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).unwrap();
    assert_that!(back.sources.deck()).is_equal_to(doc.sources.deck());
    assert_that!(back.sources.replica().deck_uid()).is_equal_to(doc.sources.replica().deck_uid());
    assert_that!(back.sources.replica().can_undo()).is_false();
    std::fs::remove_dir_all(&dir).ok();
}

/// Un document de reprise à l'ancien format (sans `deck.loro`) ne se relit pas.
#[test]
fn read_doc_without_deck_loro_returns_error() {
    let dir = temp_dir();
    let sub = dir.join(DECK_DIR);
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(
        sub.join("document.json"),
        r#"{"path":null,"dirty":false,"saved_hash":null,"blank":false,"deck":{"slides":[]},"history":{}}"#,
    )
    .unwrap();
    assert_that!(resume(&dir).read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).is_err()).is_true();
    std::fs::remove_dir_all(&dir).ok();
}

fn conv(scope: AgentScope) -> Option<SavedConversation> {
    Some(SavedConversation { scope, agent: None, session: Some("s".into()), thread: Vec::new() })
}

#[test]
fn merge_with_documents_keeps_the_previous_conversations() {
    let mut prev = job(None, "s-1");
    prev.agent.insert(AgentScope::Slides, conv(AgentScope::Slides));
    let atelier = AgentScope::Atelier { pack: uuid::Uuid::nil() };
    let mut next = job(Some("pro"), "s-2");
    next.agent.insert(atelier.clone(), conv(atelier.clone()));
    let merged = merge(prev, next);
    assert_that!(merged.agent.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec![AgentScope::Slides, atelier]);
}

#[test]
fn merge_later_delete_wins_over_earlier_write() {
    let mut prev = job(None, "s-1");
    prev.agent.insert(AgentScope::Slides, conv(AgentScope::Slides));
    let mut next = job(None, "s-2");
    next.agent.insert(AgentScope::Slides, None);
    assert_that!(merge(prev, next).agent.get(&AgentScope::Slides)).is_equal_to(Some(&None));
}

#[test]
fn merge_later_write_wins_over_earlier_delete() {
    let mut prev = job(None, "s-1");
    prev.agent.insert(AgentScope::Slides, None);
    let mut next = job(None, "s-2");
    next.agent.insert(AgentScope::Slides, conv(AgentScope::Slides));
    assert_that!(merge(prev, next).agent.get(&AgentScope::Slides)).is_equal_to(Some(&conv(AgentScope::Slides)));
}

#[test]
fn merge_without_documents_keeps_the_previous_documents() {
    let merged = merge(job(Some("pro"), "s-1"), job(None, "s-2"));
    assert_that!(merged.docs.is_empty()).is_false();
    assert_that!(merged.state.ui.slide.as_deref()).is_equal_to(Some("s-2"));
}
