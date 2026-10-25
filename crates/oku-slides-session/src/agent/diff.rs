//! Différences mot à mot d'une reformulation, pour la surface `ask_rewrite`.

use oku_slides_a2ui::{DiffKind, DiffSegment};
use similar::{ChangeTag, TextDiff};

/// Segments gardés, retirés, ajoutés ; deux voisins de même nature sont fusionnés.
#[must_use]
pub fn diff_words(old: &str, new: &str) -> Vec<DiffSegment> {
    let mut out: Vec<DiffSegment> = Vec::new();
    for change in TextDiff::from_words(old, new).iter_all_changes() {
        let kind = match change.tag() {
            ChangeTag::Equal => DiffKind::Keep,
            ChangeTag::Delete => DiffKind::Del,
            ChangeTag::Insert => DiffKind::Ins,
        };
        match out.last_mut() {
            Some(last) if last.kind == kind => last.text.push_str(change.value()),
            _ => out.push(DiffSegment { kind, text: change.value().to_owned() }),
        }
    }
    out
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod diff_tests;
