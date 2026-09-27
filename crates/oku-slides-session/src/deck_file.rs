//! Le fichier d'un deck qui n'en a pas encore : son nom, tiré du titre, et
//! une place libre dans le dossier des decks (`Budget.oku-slides.pdf`,
//! sinon `Budget 2.oku-slides.pdf`…).

use std::path::{Path, PathBuf};

use crate::files::EXTENSION;

/// Au-delà, le nom est coupé : les systèmes de fichiers limitent à 255 octets.
const MAX_STEM_CHARS: usize = 120;

/// Un nom de fichier sûr tiré de `title` ; `fallback` s'il n'en reste rien.
pub(crate) fn safe_stem(title: &str, fallback: &str) -> String {
    let cleaned: String = title
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '-' } else { c })
        .take(MAX_STEM_CHARS)
        .collect();
    // Pas de fichier caché ni de nom fait de points.
    let stem = cleaned.trim().trim_start_matches('.').trim();
    if stem.is_empty() { fallback.to_owned() } else { stem.to_owned() }
}

/// Le premier `dir/stem[ n].oku-slides.pdf` libre selon `taken`.
pub(crate) fn free_path(dir: &Path, stem: &str, taken: impl Fn(&Path) -> bool) -> PathBuf {
    let first = dir.join(format!("{stem}{EXTENSION}"));
    if !taken(&first) {
        return first;
    }
    (2..).map(|n| dir.join(format!("{stem} {n}{EXTENSION}"))).find(|p| !taken(p)).unwrap_or(first)
}

#[cfg(test)]
#[path = "deck_file_tests.rs"]
mod deck_file_tests;
