//! Versions a deck pins are frozen in the library.
//!
//! A deck in deck mode adopts every library version it pins, packs and
//! themes, so the library never replaces it: whenever its pack updates are
//! refreshed (`refresh_pack_updates`) and when a widget session ends. A pack
//! tab's working deck, a preview deck and a running widget session's draft
//! adopt nothing; deck-only versions never enter the library.

use oku_slides_core::{PackFiles, PackVersion};
use uuid::Uuid;

use crate::library::draft_lock;
use crate::pack_receive::is_deck_only;
use crate::{Mode, Session};

impl Session {
    /// The embedded files of `version` of `pin_id` when adopting it is due.
    ///
    /// `None` otherwise. Due: not the widget session's draft (the session
    /// still replaces it), listed by the library, not yet frozen and not
    /// deck-only. An unlisted version (received, or a draft replaced
    /// meanwhile) stays the deck's, keeping its `received` mark; the index
    /// suffices, no loading under the lock.
    pub(crate) fn adoptable(&self, pin_id: Uuid, version: &PackVersion) -> Option<&PackFiles> {
        if self.is_widget_draft(pin_id, version) {
            return None;
        }
        if self.library.is_frozen(pin_id, version) || !self.library.lists(pin_id, version) {
            return None;
        }
        self.doc.sources.packs.get(version).filter(|f| !is_deck_only(f))
    }

    /// Freezes every library version the deck pins, in deck mode only.
    ///
    /// A failure is logged: the deck embeds the version, nothing is lost;
    /// only a later save could replace it.
    pub(crate) fn adopt_pins(&self) {
        if !matches!(self.mode, Mode::Deck) {
            return;
        }
        let lock = draft_lock();
        let blobs = &self.doc.sources;
        let deck = blobs.deck();
        for (_, _, pin) in deck.all_pins() {
            let Some(files) = self.adoptable(pin.id, &pin.version) else { continue };
            if let Err(e) = self.library.adopt(&lock, pin.id, files) {
                tracing::warn!(pack = %pin.id, error = %e.one_line(), "version épinglée non figée");
            }
        }
        for pin in deck.themes.values() {
            if self.themes.is_frozen(pin.id, &pin.version) || !self.themes.lists(pin.id, &pin.version) {
                continue;
            }
            let Some(files) = blobs.themes.get(&pin.version) else { continue };
            if let Err(e) = self.themes.adopt(&lock, pin.id, files) {
                tracing::warn!(theme = %pin.id, error = %e.one_line(), "thème épinglé non figé");
            }
        }
    }
}

#[cfg(test)]
#[path = "deck_adopt_tests.rs"]
mod deck_adopt_tests;
