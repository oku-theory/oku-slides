//! Sorties communes à toute lecture : assets dédupliqués, avertissements, erreur fatale différée.

use std::collections::{BTreeMap, BTreeSet};

use oku_slides_core::asset_id;

use crate::error::ImportError;
use crate::source::{Asset, Warning, WarningKind};

/// Nombre d'avertissements gardés ; au-delà, un seul `Truncated`.
pub(crate) const MAX_WARNINGS: usize = 1000;
const CLIP_CHARS: usize = 200;

/// Caractère de contrôle ou de mise en forme invisible (bidi, largeur nulle, séparateurs de ligne
/// et de paragraphe, annotations interlinéaires).
fn is_unsafe(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{061c}'
                | '\u{180e}'
                | '\u{200b}'..='\u{200f}'
                | '\u{2028}'..='\u{202e}'
                | '\u{2060}'..='\u{2064}'
                | '\u{2066}'..='\u{2069}'
                | '\u{feff}'
                | '\u{fff9}'..='\u{fffb}'
        )
}

/// Chaîne venue du fichier, sans caractère dangereux, `n` caractères au plus.
pub(crate) fn clip_to(s: &str, n: usize) -> String {
    s.chars().filter(|c| !is_unsafe(*c)).take(n).collect()
}

/// Chaîne venue du fichier, rendue sûre pour un message (200 caractères au plus).
pub(crate) fn clip(s: &str) -> String {
    clip_to(s, CLIP_CHARS)
}

/// Accumulateur d'une lecture.
#[derive(Debug, Default)]
pub(crate) struct Sink {
    assets: BTreeMap<String, Asset>,
    by_path: BTreeMap<String, String>,
    /// Chemins dont la lecture a échoué (déjà signalés) : ni relus ni resignalés.
    failed: BTreeSet<String>,
    warnings: Vec<Warning>,
    fatal: Option<ImportError>,
}

impl Sink {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Note un avertissement (borné à `MAX_WARNINGS`, puis un `Truncated`).
    pub(crate) fn warn(&mut self, slide: Option<u32>, item: Option<&str>, kind: WarningKind) {
        match self.warnings.len() {
            n if n < MAX_WARNINGS => self.warnings.push(Warning { slide, item: item.map(clip), kind }),
            MAX_WARNINGS => self.warnings.push(Warning { slide: None, item: None, kind: WarningKind::Truncated }),
            _ => {}
        }
    }

    /// Identifiant de l'asset déjà lu à `path`.
    pub(crate) fn cached_asset(&self, path: &str) -> Option<String> {
        self.by_path.get(path).cloned()
    }

    /// Vrai si la lecture de l'asset `path` a déjà échoué.
    pub(crate) fn asset_failed(&self, path: &str) -> bool {
        self.failed.contains(path)
    }

    /// Note l'échec de la lecture de l'asset `path`.
    pub(crate) fn fail_asset(&mut self, path: &str) {
        self.failed.insert(path.to_owned());
    }

    /// Range un asset lu à `path` ; rend son identifiant.
    pub(crate) fn intern(&mut self, path: &str, bytes: Vec<u8>, mime: &str) -> String {
        let id = asset_id(&bytes);
        self.assets.entry(id.clone()).or_insert_with(|| Asset { mime: mime.to_owned(), bytes });
        self.by_path.insert(path.to_owned(), id.clone());
        id
    }

    /// Garde la première erreur fatale rencontrée.
    pub(crate) fn set_fatal(&mut self, e: ImportError) {
        self.fatal.get_or_insert(e);
    }

    /// Une erreur fatale est-elle notée ?
    pub(crate) fn has_fatal(&self) -> bool {
        self.fatal.is_some()
    }

    /// Seule sortie : l'erreur fatale si elle existe, sinon assets et avertissements.
    pub(crate) fn finish(self) -> Result<(BTreeMap<String, Asset>, Vec<Warning>), ImportError> {
        match self.fatal {
            Some(e) => Err(e),
            None => Ok((self.assets, self.warnings)),
        }
    }
}

#[cfg(test)]
#[path = "collect_tests.rs"]
mod collect_tests;
