use speculoos::prelude::*;

use super::*;
use oku_slides_core::{Deck, Op, SourceBundle};
use oku_slides_platform::storage::StorageErrorKind;

fn temp_dir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "oku-slides-resume-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn resume() -> Resume {
    Resume::new(Storage::local().unwrap(), temp_dir())
}

fn deck() -> Deck {
    Deck::from_json(r#"{"theme":"papier","slides":[]}"#).unwrap()
}

fn document() -> Document {
    let mut sources = SourceBundle::new(deck());
    sources.add_asset(b"image".to_vec());
    sources.fonts.insert("Inter.ttf".into(), b"police".to_vec());
    let mut doc = Document::new(sources, Some(PathBuf::from("/tmp/essai.oku-slides.pdf")), 10);
    doc.history.commit(&mut doc.sources.deck, "Thème", vec![Op::SetTheme { theme: Some("pro".into()) }]).unwrap();
    doc.dirty = true;
    doc.saved_hash = Some("sha256:00".into());
    doc
}

#[test]
fn write_doc_then_read_doc_round_trips_sources_history_and_meta() {
    let r = resume();
    let doc = document();
    r.write_doc(DECK_DIR, &doc).unwrap();
    let back = r.read_doc(DECK_DIR, 10).unwrap();
    assert_that!(back.sources).is_equal_to(&doc.sources);
    assert_that!(back.history.undo_label()).is_equal_to(Some("Thème"));
    assert_that!(back.path).is_equal_to(&doc.path);
    assert_that!(back.dirty).is_true();
    assert_that!(back.saved_hash).is_equal_to(&doc.saved_hash);
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn write_doc_removes_orphan_assets() {
    let r = resume();
    let mut doc = document();
    r.write_doc(DECK_DIR, &doc).unwrap();
    doc.sources.assets.clear();
    r.write_doc(DECK_DIR, &doc).unwrap();
    assert_that!(r.read_doc(DECK_DIR, 10).unwrap().sources.assets.len()).is_equal_to(0);
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn read_doc_with_corrupt_history_keeps_deck_with_empty_history() {
    let r = resume();
    r.write_doc(DECK_DIR, &document()).unwrap();
    let path = r.dir.join(DECK_DIR).join("document.json");
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    v["history"] = serde_json::json!("illisible");
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    let back = r.read_doc(DECK_DIR, 10).unwrap();
    assert_that!(back.sources.deck.theme.as_deref()).is_equal_to(Some("pro"));
    assert_that!(back.history.can_undo()).is_false();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn read_state_missing_returns_none() {
    let r = resume();
    assert_that!(r.read_state().unwrap()).is_none();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn write_state_then_read_state_round_trips_mode_and_ui() {
    let r = resume();
    let edit = TemplateMode { set: "essentiel".into(), name: "Essentiel".into(), builtin: true };
    let ui = UiState { slide: Some("s-1".into()), grid: false };
    r.write_state(&State { version: VERSION, mode: SavedMode::Templates(edit), ui: ui.clone() }).unwrap();
    let back = r.read_state().unwrap().unwrap();
    assert_that!(matches!(back.mode, SavedMode::Templates(ref m) if m.set == "essentiel")).is_true();
    assert_that!(back.ui).is_equal_to(ui);
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn read_state_unknown_version_returns_error() {
    let r = resume();
    std::fs::create_dir_all(&r.dir).unwrap();
    std::fs::write(r.dir.join("state.json"), br#"{"version":99,"mode":"deck"}"#).unwrap();
    assert_that!(matches!(r.read_state(), Err(ResumeError::Version { version: 99, .. }))).is_true();
    std::fs::remove_dir_all(&r.dir).ok();
}

/// Un `state.json` valide mais démesuré n'est pas lu en entier : illisible.
#[test]
fn read_state_oversized_returns_access_error_too_large() {
    let r = resume();
    let mut bytes = br#"{"version":1,"mode":"deck"}"#.to_vec();
    bytes.resize(usize::try_from(MAX_JSON_BYTES).unwrap() + 1, b' ');
    std::fs::create_dir_all(&r.dir).unwrap();
    std::fs::write(r.dir.join("state.json"), bytes).unwrap();
    let err = r.read_state().unwrap_err();
    let too_large = matches!(&err, ResumeError::Access { source, .. } if source.kind() == StorageErrorKind::TooLarge);
    assert_that!(too_large).is_true();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn read_state_future_version_with_other_shape_returns_version_error() {
    let r = resume();
    std::fs::create_dir_all(&r.dir).unwrap();
    std::fs::write(r.dir.join("state.json"), br#"{"version":2,"mode":{"nouveau":1}}"#).unwrap();
    assert_that!(matches!(r.read_state(), Err(ResumeError::Version { version: 2, .. }))).is_true();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn write_doc_font_changed_under_same_name_reads_new_bytes() {
    let r = resume();
    let mut doc = document();
    r.write_doc(DECK_DIR, &doc).unwrap();
    doc.sources.fonts.insert("Inter.ttf".into(), b"nouvelle police".to_vec());
    r.write_doc(DECK_DIR, &doc).unwrap();
    let back = r.read_doc(DECK_DIR, 10).unwrap();
    assert_that!(back.sources.fonts.get("Inter.ttf").cloned()).is_equal_to(Some(b"nouvelle police".to_vec()));
    let fonts_dir = r.dir.join(DECK_DIR).join("fonts");
    assert_that!(std::fs::read_dir(&fonts_dir).unwrap().count()).is_equal_to(1);
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn write_doc_unsafe_names_writes_nothing_outside_dir() {
    let r = resume();
    let mut doc = document();
    doc.sources.assets.insert("sha256:../x".into(), b"evil".to_vec());
    doc.sources.fonts.insert("../evil.ttf".into(), b"evil".to_vec());
    doc.sources.fonts.insert(".hidden.ttf".into(), b"evil".to_vec());
    r.write_doc(DECK_DIR, &doc).unwrap();
    assert_that!(r.dir.join(DECK_DIR).join("assets").join("x").exists()).is_false();
    assert_that!(r.dir.join(DECK_DIR).join("x").exists()).is_false();
    assert_that!(r.dir.parent().unwrap().join("x").exists()).is_false();
    assert_that!(r.dir.join(DECK_DIR).join("fonts").join("evil.ttf").exists()).is_false();
    assert_that!(r.dir.join(DECK_DIR).join("evil.ttf").exists()).is_false();
    assert_that!(r.dir.parent().unwrap().join("evil.ttf").exists()).is_false();
    let back = r.read_doc(DECK_DIR, 10).unwrap();
    assert_that!(back.sources.assets.contains_key("sha256:../x")).is_false();
    assert_that!(back.sources.fonts.contains_key("evil.ttf")).is_false();
    assert_that!(back.sources.fonts.contains_key(".hidden.ttf")).is_false();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn read_doc_with_stored_path_not_oku_slides_drops_path() {
    let r = resume();
    let mut doc = document();
    doc.path = Some(PathBuf::from("/tmp/notes.txt"));
    r.write_doc(DECK_DIR, &doc).unwrap();
    assert_that!(r.read_doc(DECK_DIR, 10).unwrap().path).is_none();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn read_doc_with_stored_relative_path_drops_path() {
    let r = resume();
    let mut doc = document();
    doc.path = Some(PathBuf::from("essai.oku-slides.pdf"));
    r.write_doc(DECK_DIR, &doc).unwrap();
    assert_that!(r.read_doc(DECK_DIR, 10).unwrap().path).is_none();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn write_doc_then_read_doc_keeps_the_blank_flag() {
    let r = resume();
    let mut doc = document();
    doc.blank = true;
    r.write_doc(DECK_DIR, &doc).unwrap();
    assert_that!(r.read_doc(DECK_DIR, 10).unwrap().blank).is_true();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn write_doc_then_read_doc_keeps_a_non_blank_flag() {
    let r = resume();
    let doc = document();
    assert_that!(doc.blank).is_false();
    r.write_doc(DECK_DIR, &doc).unwrap();
    assert_that!(r.read_doc(DECK_DIR, 10).unwrap().blank).is_false();
    std::fs::remove_dir_all(&r.dir).ok();
}
