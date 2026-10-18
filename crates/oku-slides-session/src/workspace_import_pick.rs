//! Un `.pptx` reçu de l'OS : la vue demande le dialogue d'import à sa fenêtre.
//!
//! Argument de lancement, seconde instance ou macOS (`RunEvent::Opened`) :
//! le fichier ne s'ouvre pas comme un deck. Il devient l'[`ImportPick`] de la
//! fenêtre active, porté par la vue : une page qui n'écoute pas encore (ou le
//! mode web, sans événements) le trouve à sa première lecture. La page qui a
//! ouvert le dialogue l'acquitte (`ImportPicked`) : un rechargement ne le
//! rouvre pas.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::Workspace;

/// Un `.pptx` reçu de l'OS, à montrer dans le dialogue d'import.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportPick {
    /// Label de la fenêtre qui ouvre le dialogue (la fenêtre active).
    pub window: String,
    /// Le `.pptx` (normalisé à la réception).
    pub path: PathBuf,
    /// Numéro croissant : la page ouvre le dialogue une fois par numéro.
    pub id: u64,
}

/// Une présentation PowerPoint : l'extension `.pptx`, sans casse.
pub(super) fn is_pptx(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pptx"))
}

impl Workspace {
    /// `path` devient le choix d'import de la fenêtre active, à la place du précédent.
    pub(super) fn pick_import(&mut self, path: PathBuf) {
        if let Some(previous) = &self.import_pick {
            tracing::info!(replaced = %previous.path.display(), by = %path.display(), "choix d'import remplacé");
        }
        self.last_pick = self.last_pick.saturating_add(1);
        self.import_pick = Some(ImportPick { window: self.layout.focused.clone(), path, id: self.last_pick });
    }

    /// `ImportPicked` : oublie le choix `id`, montré par sa page.
    ///
    /// Un choix plus récent reste.
    pub(super) fn import_picked(&mut self, id: u64) {
        if self.import_pick.as_ref().is_some_and(|p| p.id == id) {
            self.import_pick = None;
        }
    }
}

#[cfg(test)]
#[path = "workspace_import_pick_tests.rs"]
mod workspace_import_pick_tests;
