//! Knowledge ops: write, edit, move and delete a file of the deck knowledge bundle.
//!
//! Applying is deterministic: the id of a new file comes from the op, never from here. Every
//! write re-checks the path, the frontmatter and the bundle ceilings, and leaves the deck as it
//! was when it refuses.

use crate::error::{KnowledgeReason, KnowledgeSnafu, OpError};
use crate::knowledge::limits::{MAX_FILE_BYTES, MAX_FILES, MAX_TOTAL_BYTES};
use crate::knowledge::{KnowledgeFile, KnowledgePath, check_text, is_file_id};
use crate::model::Deck;

/// A refusal of the op on `path`.
fn refuse(path: &str, reason: KnowledgeReason) -> OpError {
    KnowledgeSnafu { path, reason }.build().into()
}

/// The file at `path` with its id, if the bundle has one there.
fn find_mut<'a>(deck: &'a mut Deck, path: &str) -> Option<(&'a String, &'a mut KnowledgeFile)> {
    deck.knowledge.iter_mut().find(|(_, file)| file.path == path)
}

/// Is there a file at `path`?
fn exists(deck: &Deck, path: &str) -> bool {
    deck.knowledge.values().any(|file| file.path == path)
}

/// The bytes of the whole bundle.
fn total_bytes(deck: &Deck) -> usize {
    deck.knowledge.values().map(|file| file.text.len()).sum()
}

/// Checks `text` as the content of the file at `path`, and the size ceilings given `others`,
/// the bytes of the rest of the bundle.
fn check_content(path: &KnowledgePath, text: &str, others: usize) -> Result<(), OpError> {
    let at = path.as_str();
    if text.len() > MAX_FILE_BYTES {
        return Err(refuse(at, KnowledgeReason::FileTooLarge));
    }
    check_text(path, text).map_err(|reason| refuse(at, KnowledgeReason::Frontmatter(reason)))?;
    if others + text.len() > MAX_TOTAL_BYTES {
        return Err(refuse(at, KnowledgeReason::TotalTooLarge));
    }
    Ok(())
}

/// Creates the file, or replaces its text keeping its id.
pub(super) fn write(deck: &mut Deck, path: &str, text: String, id: Option<String>) -> Result<(), OpError> {
    let parsed = KnowledgePath::parse_writable(path).map_err(|reason| refuse(path, KnowledgeReason::Path(reason)))?;
    let total = total_bytes(deck);
    if let Some((existing, file)) = find_mut(deck, path) {
        if id.is_some_and(|id| id != *existing) {
            return Err(refuse(path, KnowledgeReason::IdMismatch));
        }
        check_content(&parsed, &text, total - file.text.len())?;
        file.text = text;
        return Ok(());
    }
    let id = id.ok_or_else(|| refuse(path, KnowledgeReason::IdMissing))?;
    if !is_file_id(&id) {
        return Err(refuse(path, KnowledgeReason::IdInvalid));
    }
    if deck.knowledge.contains_key(&id) {
        return Err(refuse(path, KnowledgeReason::IdTaken));
    }
    if deck.knowledge.len() >= MAX_FILES {
        return Err(refuse(path, KnowledgeReason::TooManyFiles));
    }
    check_content(&parsed, &text, total)?;
    deck.knowledge.insert(id, KnowledgeFile { path: path.to_owned(), text });
    Ok(())
}

/// Replaces the single occurrence of `old` with `new`, byte for byte.
///
/// Overlapping occurrences count: `aa` in `aaa` is ambiguous.
pub(super) fn edit(deck: &mut Deck, path: &str, old: &str, new: &str) -> Result<(), OpError> {
    let parsed = KnowledgePath::parse_writable(path).map_err(|reason| refuse(path, KnowledgeReason::Path(reason)))?;
    let total = total_bytes(deck);
    let Some((_, file)) = find_mut(deck, path) else { return Err(refuse(path, KnowledgeReason::NotFound)) };
    if old.is_empty() {
        return Err(refuse(path, KnowledgeReason::EditEmptyOld));
    }
    let Some(at) = file.text.find(old) else { return Err(refuse(path, KnowledgeReason::EditNoMatch)) };
    // Search again one character after the first hit, so an overlapping one is found too.
    let next = file.text[at..].char_indices().nth(1).map_or(file.text.len(), |(i, _)| at + i);
    if file.text[next..].contains(old) {
        return Err(refuse(path, KnowledgeReason::EditAmbiguous));
    }
    let text = [&file.text[..at], new, &file.text[at + old.len()..]].concat();
    check_content(&parsed, &text, total - file.text.len())?;
    file.text = text;
    Ok(())
}

/// Moves the file to a free path, keeping its id and text; the text is re-checked at the target.
pub(super) fn relocate(deck: &mut Deck, from: &str, to: &str) -> Result<(), OpError> {
    let taken = exists(deck, to);
    let Some((_, file)) = find_mut(deck, from) else { return Err(refuse(from, KnowledgeReason::NotFound)) };
    let target = KnowledgePath::parse_writable(to).map_err(|reason| refuse(to, KnowledgeReason::Path(reason)))?;
    if taken {
        return Err(refuse(to, KnowledgeReason::Exists));
    }
    check_text(&target, &file.text).map_err(|reason| refuse(to, KnowledgeReason::Frontmatter(reason)))?;
    to.clone_into(&mut file.path);
    Ok(())
}

/// Deletes the file.
pub(super) fn delete(deck: &mut Deck, path: &str) -> Result<(), OpError> {
    let Some((id, _)) = find_mut(deck, path) else { return Err(refuse(path, KnowledgeReason::NotFound)) };
    let id = id.clone();
    deck.knowledge.remove(&id);
    Ok(())
}

#[cfg(test)]
#[path = "knowledge_tests.rs"]
mod knowledge_tests;
