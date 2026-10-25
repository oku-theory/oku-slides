//! Un onglet importé : le deck d'un `.pptx`, sans fichier, et l'état de son import.
//!
//! [`Session::load_import`] charge le deck obtenu par [`super::import`] comme
//! un deck neuf non enregistré (titre de la présentation, sinon le nom du
//! `.pptx`), relève ses débordements sur le rendu de la session, et garde
//! l'[`ImportState`](super::ImportState) en mémoire tant que la session vit. Un autre document
//! chargé l'oublie ; la reprise ne l'écrit pas : un onglet repris n'en a pas.

use snafu::ResultExt;

use super::{Imported, note_overflows};
use crate::error::SourcesSnafu;
use crate::{Result, Session};

impl Session {
    /// Loads import `imported` of the `.pptx` named `name` as an unsaved deck.
    ///
    /// The deck keeps the import's history step (undoing it empties the deck);
    /// without a title, it takes `name`. Its widgets are probed again by the
    /// session's render process, which renders it.
    ///
    /// # Errors
    ///
    /// Fails if the title cannot be written to the sources, or if the deck does not load or render.
    pub fn load_import(&mut self, imported: Imported, name: &str) -> Result<()> {
        let Imported { mut sources, mut state, .. } = imported;
        if sources.deck().params.title.is_none() && !name.trim().is_empty() {
            let mut deck = sources.deck().clone();
            deck.params.title = Some(name.to_owned());
            // Un nom, pas une modification : l'étape de l'import reste la seule.
            sources.apply_silent(deck).context(SourcesSnafu)?;
        }
        self.load(sources, None)?;
        if let Some(compiled) = &self.compiled {
            note_overflows(compiled.layout(), state.report_mut());
        }
        self.doc.dirty = true;
        self.import = Some(state);
        self.persist(true);
        Ok(())
    }

    /// L'état de l'import d'où vient le deck, s'il en vient (onglet importé, pas repris).
    #[cfg(test)]
    pub(crate) fn import_state(&self) -> Option<&super::ImportState> {
        self.import.as_ref()
    }
}

#[cfg(test)]
#[path = "tab_tests.rs"]
mod tab_tests;
