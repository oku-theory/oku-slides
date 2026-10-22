//! Annuler et rétablir, par le replica Loro du document.
//!
//! Après l'étape : l'édition d'un pack est réalignée, les widgets resynchronisés,
//! et un renommage de widget défait ou refait (un nom retiré et un ajouté dans le
//! même pack) déplace la conversation de l'agent comme `rename_widget` (t-613).

use oku_slides_core::SourceBundle;
use snafu::ResultExt;

use crate::error::SourcesSnafu;
use crate::{Result, Session};

/// Une étape du replica : annuler ou rétablir ; `true` si elle a eu lieu.
type Step = fn(&mut SourceBundle) -> std::result::Result<bool, oku_slides_core::SourcesError>;

impl Session {
    /// Annule par le replica ; un rendu raté garde l'état et remonte l'erreur.
    pub fn undo(&mut self) -> Result<()> {
        self.replay(SourceBundle::undo)
    }

    /// Rétablit par le replica ; un rendu raté garde l'état et remonte l'erreur.
    pub fn redo(&mut self) -> Result<()> {
        self.replay(SourceBundle::redo)
    }

    fn replay(&mut self, step: Step) -> Result<()> {
        self.end_widget_session();
        self.ensure_editable()?;
        let before = self.widgets_before();
        if step(&mut self.doc.sources).context(SourcesSnafu)? {
            self.doc.dirty = true;
            self.realign_pack()?;
            self.sync_widgets()?;
            self.follow_widget_rename(before);
            self.rerender()?;
        }
        Ok(())
    }
}
