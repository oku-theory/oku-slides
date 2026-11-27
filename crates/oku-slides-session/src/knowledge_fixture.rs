//! A deck knowledge base and its approvals, for this and other crates' tests.
//!
//! The deck's knowledge is set here silently, as a received deck brings it: nothing is approved.
#![expect(clippy::panic, reason = "a test-support fixture: a setup that fails is the calling test failing")]

use std::path::Path;

pub use oku_slides_core::knowledge::unstamped;
use oku_slides_core::{KnowledgeFile, asset_id};

use crate::Session;

/// Writes `knowledge-approvals.json` in `data`, approving `texts`.
///
/// Approvals are read when a session is built: call it before `Session::new` on `data`.
///
/// # Panics
///
/// If the file cannot be written: a broken test setup.
pub fn approve_knowledge(data: &Path, texts: &[&str]) {
    let hashes: Vec<String> = texts.iter().map(|t| asset_id(t.as_bytes())).collect();
    let json = serde_json::to_vec(&hashes).unwrap_or_else(|e| panic!("approvals json: {e}"));
    std::fs::write(data.join("knowledge-approvals.json"), json)
        .unwrap_or_else(|e| panic!("write knowledge approvals: {e}"));
}

/// Adds `assets` to the deck of `s`, then sets its knowledge to `files` (path, text), silently.
///
/// # Panics
///
/// If the deck refuses the knowledge: a broken test setup.
pub fn seed_knowledge(s: &mut Session, files: &[(&str, &str)], assets: Vec<Vec<u8>>) {
    for bytes in assets {
        s.doc.sources.add_asset(bytes);
    }
    let mut deck = s.doc.sources.deck().clone();
    deck.knowledge.clear();
    for (n, (path, text)) in files.iter().enumerate() {
        deck.knowledge
            .insert(format!("{n:032x}"), KnowledgeFile { path: (*path).to_owned(), text: (*text).to_owned() });
    }
    s.doc.sources.apply_silent(deck).unwrap_or_else(|e| panic!("seed knowledge: {e}"));
}
