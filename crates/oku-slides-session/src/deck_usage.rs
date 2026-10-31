//! The deck's agent usage: turns recorded without making the deck unsaved.
//!
//! A record goes to the deck's own document (the stash while history is
//! open), snapshots the resume at once and waits for the next save; a
//! clean tab that closes writes it alone.

use oku_slides_core::{Inserted, UsageLedger, UsageRecord};
use oku_slides_platform::storage::StorageErrorKind;

use crate::{Document, Mode, Session};

impl Session {
    /// Records a turn's agent usage in the deck; false when nothing was added.
    ///
    /// Only in deck mode or, while history is open, in the deck set aside: a
    /// help, pack or theme tab records nothing (the workspace routes their
    /// turns to their deck tab), nor does `[usage] store_in_deck` off.
    pub fn record_usage(&mut self, record: UsageRecord) -> bool {
        if !self.doc_defaults.store_usage {
            return false;
        }
        let turn = record.turn;
        let Some(doc) = self.usage_document_mut() else { return false };
        // A forged turn may still replace the held record (the shorter line wins).
        let offered = record.clone();
        let (added, changed) = match doc.sources.usage.insert(record) {
            Inserted::Added => (true, true),
            Inserted::Same => (false, false),
            Inserted::Full => {
                if !doc.usage_full_warned {
                    doc.usage_full_warned = true;
                    tracing::warn!(%turn, "agent usage ledger full: later turns are not recorded");
                }
                (false, false)
            }
            Inserted::Forged => {
                tracing::warn!(%turn, "agent usage record differs from the one held for its turn");
                (false, doc.sources.usage.records().any(|r| *r == offered))
            }
            Inserted::Invalid => {
                tracing::warn!(%turn, "agent usage record refused by the ledger");
                (false, false)
            }
        };
        if changed {
            doc.usage_unsaved = true;
            self.persist(true);
        }
        added
    }

    /// Writes a clean closing deck's missing agent usage into its file.
    ///
    /// No render: the deck is clean, so the file on disk already shows it.
    /// Only in deck mode, for a file still there whose bytes are those of the
    /// last save; otherwise the records are dropped with a warning, and an
    /// unsaved deck is left to the close's own question. With history open
    /// nothing is written: the tab counts as unsaved, so a close asks first.
    /// With `[usage] store_in_deck` off the file is left as it is.
    pub(crate) fn save_pending_usage(&mut self) {
        if let Mode::History { stash, .. } = &self.mode {
            if stash.usage_unsaved {
                tracing::warn!("agent usage not saved: history is open, the records wait for a save");
            }
            return;
        }
        if !matches!(self.mode, Mode::Deck) || self.doc.dirty || !self.doc.usage_unsaved {
            return;
        }
        if !self.doc_defaults.store_usage {
            // Off drops the ledger only on a save: a silent write never removes records.
            tracing::info!("agent usage not written: [usage] store_in_deck is off, the file is left as is");
            self.doc.usage_unsaved = false;
            self.persist(true);
            return;
        }
        let Some(path) = self.doc.path.clone() else {
            tracing::warn!("agent usage dropped: the deck has no file");
            return;
        };
        let Some(saved) = self.doc.saved_hash.clone() else {
            tracing::warn!(path = %path.display(), "agent usage dropped: the file is not a deck this app saved");
            return;
        };
        let pdf = match self.storage.read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == StorageErrorKind::NotFound => {
                // Not written again behind the user's back.
                tracing::warn!(path = %path.display(), "agent usage dropped: the file was deleted");
                return;
            }
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %oku_slides_platform::errors::one_line(&e),
                    "agent usage dropped: the file is unreadable"
                );
                return;
            }
        };
        if oku_slides_core::asset_id(&pdf) != saved {
            tracing::warn!(path = %path.display(), "agent usage dropped: the file changed on disk");
            return;
        }
        match self.write_pdf(&path, &pdf) {
            Ok(hash) => {
                self.doc.saved_hash = Some(hash);
                self.doc.usage_unsaved = false;
                // A close refused later keeps a resume that matches the file.
                self.persist(true);
            }
            Err(e) => tracing::warn!(error = %e.one_line(), "agent usage dropped: the file could not be written"),
        }
    }

    /// The deck's usage ledger; `None` in a help, pack or theme tab.
    #[must_use]
    pub fn usage_ledger(&self) -> Option<&UsageLedger> {
        match &self.mode {
            Mode::Deck => Some(&self.doc.sources.usage),
            Mode::History { stash, .. } => Some(&stash.sources.usage),
            Mode::Help { .. } | Mode::Pack { .. } | Mode::Theme { .. } => None,
        }
    }

    /// The document whose ledger takes a turn: the deck, or the deck set aside.
    fn usage_document_mut(&mut self) -> Option<&mut Document> {
        match &mut self.mode {
            Mode::Deck => Some(&mut self.doc),
            Mode::History { stash, .. } => Some(&mut **stash),
            Mode::Help { .. } | Mode::Pack { .. } | Mode::Theme { .. } => None,
        }
    }
}

#[cfg(test)]
#[path = "deck_usage_tests.rs"]
pub(crate) mod deck_usage_tests;
