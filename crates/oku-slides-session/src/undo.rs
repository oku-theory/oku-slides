//! Undo and redo, through the document's Loro replica.
//!
//! After the step: a pack edit is realigned, the widgets resynchronized, and a
//! widget rename undone or redone (one name removed and one added in the same
//! pack) moves the agent's conversation like `rename_widget` (t-613). A step
//! that would change the version an owner deck pins for a pack its deck-pack
//! tab edits is taken back by the opposite step and refused (`UndoInPackTab`).

use std::collections::BTreeMap;

use snafu::ResultExt;
use uuid::Uuid;

use oku_slides_core::SourceBundle;

use crate::deck_pack_link::LinkedPins;
use crate::error::{SourcesSnafu, UndoCompensationFailedSnafu};
use crate::{Result, Session, SessionError};

/// One replica step: undo or redo; `true` if it took place.
type Step = fn(&mut SourceBundle) -> std::result::Result<bool, oku_slides_core::SourcesError>;

#[cfg(test)]
thread_local! {
    /// Tests only: the opposite step of a refused undo or redo fails, as if Loro refused it.
    ///
    /// Thread-local, so safe under any runner; nextest also runs one process per test.
    pub(crate) static FAIL_OPPOSITE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

impl Session {
    /// Undoes through the replica; a failed render keeps the state and returns the error.
    ///
    /// # Errors
    ///
    /// Fails if the session is not editable, if the edited pack's lease is
    /// held elsewhere (`PackOpenInTab`), the pack is gone (`UnknownPack`) or
    /// its base missing (`PackVersionMissing`), the library write refused
    /// (`LeaseMissing`), or if rendering the undone state fails. A step that
    /// would change a pack a deck-pack tab of this deck edits is refused
    /// (`UndoInPackTab`), the deck unchanged; `UndoCompensationFailed` if it
    /// could not be taken back.
    pub fn undo(&mut self) -> Result<()> {
        self.replay(SourceBundle::undo, SourceBundle::redo)
    }

    /// Redoes through the replica; a failed render keeps the state and returns the error.
    ///
    /// # Errors
    ///
    /// Fails like [`Self::undo`]: not editable, `PackOpenInTab`, `UnknownPack`,
    /// `LeaseMissing`, `PackVersionMissing`, `UndoInPackTab`,
    /// `UndoCompensationFailed`, or a failed render.
    pub fn redo(&mut self) -> Result<()> {
        self.replay(SourceBundle::redo, SourceBundle::undo)
    }

    /// Plays `do_step`; takes it back with `opposite` when it moved a linked pack's pins.
    fn replay(&mut self, do_step: Step, opposite: Step) -> Result<()> {
        // A step may bring a session version back into the library: the editor needs its lease.
        self.ensure_pack_lease()?;
        self.end_widget_session();
        self.ensure_document_editable()?;
        let before = self.linked_pins();
        if do_step(&mut self.doc.sources).context(SourcesSnafu)? {
            if let Err(refused) = self.stop_at_linked_pins(&before) {
                return Err(self.take_back(opposite, &before, refused));
            }
            self.doc.dirty = true;
            self.realign_pack()?;
            self.realign_theme_edit();
            self.sync_widgets()?;
            self.refresh_pack_updates_if_pins_changed();
            self.rerender()?;
        }
        Ok(())
    }

    /// Runs `opposite` after a refused step: `refused` once the pins are `before` again.
    ///
    /// Nothing rerenders and `dirty` stays as it was. A failed opposite step,
    /// or pins it does not bring back, is `UndoCompensationFailed`, its cause
    /// logged: the deck is then marked dirty, its widgets synced and rerendered.
    fn take_back(
        &mut self,
        opposite: Step,
        before: &BTreeMap<Uuid, LinkedPins>,
        refused: SessionError,
    ) -> SessionError {
        #[cfg(test)]
        let failing = FAIL_OPPOSITE.with(std::cell::Cell::get);
        #[cfg(not(test))]
        let failing = false;
        let back = if failing { Ok(false) } else { opposite(&mut self.doc.sources) };
        match back {
            Ok(true) if self.linked_pins() == *before => return refused,
            Ok(done) => tracing::error!(done, "refused undo or redo not taken back: the linked pins moved"),
            Err(e) => tracing::error!(error = %e, "refused undo or redo not taken back"),
        }
        // The deck stays moved: it is dirty, and its widgets and view follow it.
        self.doc.dirty = true;
        if let Err(e) = self.sync_widgets().and_then(|()| self.rerender()) {
            tracing::error!(error = %e.one_line(), "deck not shown again after a failed take-back");
        }
        UndoCompensationFailedSnafu.build().into()
    }
}
