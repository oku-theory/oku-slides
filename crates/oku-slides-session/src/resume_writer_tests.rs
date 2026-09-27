use std::path::PathBuf;

use oku_slides_core::{Deck, Op, SourceBundle};
use speculoos::prelude::*;

use super::*;
use crate::Document;
use crate::resume::{DECK_DIR, DocSnapshot, SavedMode, State, VERSION};
use crate::views::UiState;

fn temp_dir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "oku-slides-writer-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn resume(dir: &std::path::Path) -> Resume {
    Resume::new(oku_slides_platform::storage::Storage::local().unwrap(), dir.to_owned())
}

fn document(theme: &str) -> Document {
    let deck = Deck::from_json(r#"{"theme":"papier","slides":[]}"#).unwrap();
    let mut doc = Document::new(SourceBundle::new(deck), None, 10);
    doc.history.commit(&mut doc.sources.deck, "Thème", vec![Op::SetTheme { theme: Some(theme.into()) }]).unwrap();
    doc
}

fn state(slide: &str) -> State {
    State { version: VERSION, mode: SavedMode::Deck, ui: UiState { slide: Some(slide.into()), grid: true } }
}

fn job(theme: Option<&str>, slide: &str) -> Job {
    let docs = theme.map(|t| vec![(DECK_DIR, DocSnapshot::of(&document(t), &mut Vec::new()))]);
    Job { docs: docs.unwrap_or_default(), state: state(slide) }
}

/// Après `flush`, tout ce qui a été envoyé est sur le disque.
#[test]
fn writer_flush_waits_for_the_sent_job() {
    let dir = temp_dir();
    let w = Writer::new(resume(&dir));
    w.send(job(Some("pro"), "s-1"));
    w.flush();
    let doc = resume(&dir).read_doc(DECK_DIR, 10).unwrap();
    assert_that!(doc.sources.deck.theme.as_deref()).is_equal_to(Some("pro"));
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
    assert_that!(r.read_doc(DECK_DIR, 10).unwrap().sources.deck.theme.as_deref()).is_equal_to(Some("pro"));
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
    let first = DocSnapshot::of(&doc, &mut cache);
    let second = DocSnapshot::of(&doc, &mut cache);
    assert_that!(std::sync::Arc::ptr_eq(first.blobs(), second.blobs())).is_true();
    doc.sources.fonts.insert("A.ttf".into(), vec![3; 1000]);
    let third = DocSnapshot::of(&doc, &mut cache);
    assert_that!(std::sync::Arc::ptr_eq(second.blobs(), third.blobs())).is_false();
    assert_that!(third.blobs().fonts["A.ttf"][0]).is_equal_to(3);
}
