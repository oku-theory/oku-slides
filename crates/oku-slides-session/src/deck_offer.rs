//! The opener's offer: a deck takes the pack version its pack tab wrote.
//!
//! When a pack tab closes, the deck it was opened from may pin an older
//! version of the pack. It is offered the version the tab wrote, only while
//! that version is the library's current one, newer than the deck's, and its
//! requirements are pinned by the deck. Taking it pins exactly that version,
//! rechecked at that moment (`use_pack_version`), in one undo step.

use serde::Serialize;
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{Namespace, Op, PackKind, PackPin, PackVersion};

use crate::agent::bounds::label;
use crate::deck_packs::Drift;
use crate::error::{PackManifestSnafu, PackOfferExpiredSnafu, PackVersionMissingSnafu};
use crate::pack::PackManifest;
use crate::{Mode, Result, Session};

/// A deck's offer to take a pack version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct DeckOffer {
    /// The deck title, as a label.
    pub title: String,
    /// The deck's identity.
    pub deck_uid: String,
}

impl Session {
    /// The deck's first pin of `pack`, with its kind and namespace.
    fn pack_pin(&self, pack: Uuid) -> Option<(PackKind, Namespace, PackPin)> {
        let deck = self.doc.sources.deck();
        deck.all_pins().find(|(_, _, pin)| pin.id == pack).map(|(kind, ns, pin)| (kind, ns.clone(), pin.clone()))
    }

    /// This deck's offer to take `version` of `pack`, if still due.
    ///
    /// Due in deck mode when `version` is the library's current one, the
    /// deck's first pin of `pack` is older (`Drift::Older`), and the deck pins
    /// what `version` requires (as `Op::AddPack` demands). A version that
    /// cannot be read gives no offer, logged.
    ///
    /// # Blocking
    ///
    /// Reads the library index and the version's manifest.
    pub(crate) fn opener_offer(&self, pack: Uuid, version: &PackVersion) -> Option<DeckOffer> {
        if !matches!(self.mode, Mode::Deck) || self.library.current(pack).as_ref() != Some(version) {
            return None;
        }
        let (kind, ns, pin) = self.pack_pin(pack)?;
        if self.pin_relation(&pin) != Some(Drift::Older) {
            return None;
        }
        let Some(files) = self.library.version(pack, version) else {
            tracing::warn!(pack = %pack, version = %version, "offered version unreadable: no offer");
            return None;
        };
        let manifest = match PackManifest::from_files(&files) {
            Ok(manifest) => manifest,
            Err(e) => {
                tracing::warn!(pack = %pack, version = %version, error = %e, "offered manifest unreadable: no offer");
                return None;
            }
        };
        let deck = self.doc.sources.deck();
        // The rule of `Op::AddPack`: a widget pack may require its own namespace.
        let met =
            manifest.requires.iter().all(|r| (kind == PackKind::Widgets && *r == ns) || deck.widgets.contains_key(r));
        met.then(|| DeckOffer {
            title: label(&self.document_name(&self.doc)),
            deck_uid: self.doc.sources.replica().deck_uid(),
        })
    }

    /// The deck pins exactly `version` of `pack`, if the offer still holds.
    ///
    /// The tab must still hold the deck `deck_uid`, `version` must still be
    /// current and newer than the deck's pin; then the widget session ends
    /// and the pin moves in one undo step, with that version's `requires`.
    ///
    /// # Errors
    ///
    /// `PackOfferExpired`; `ReadOnly`, `EditingPack`, `InHistory` (`ensure_deck_mode`);
    /// `PackVersionMissing`, `InvalidDocument` (an unreadable manifest), `Op` for real faults.
    ///
    /// # Blocking
    ///
    /// Reads the library.
    pub(crate) fn use_pack_version(&mut self, pack: Uuid, version: &PackVersion, deck_uid: &str) -> Result<()> {
        self.ensure_deck_mode()?;
        let same_deck = self.doc.sources.replica().deck_uid() == deck_uid;
        ensure!(same_deck && self.library.current(pack).as_ref() == Some(version), PackOfferExpiredSnafu);
        let (kind, namespace, pin) = self.pack_pin(pack).context(PackOfferExpiredSnafu)?;
        ensure!(self.pin_relation(&pin) == Some(Drift::Older), PackOfferExpiredSnafu);
        let unreadable = |e: &crate::SessionError| {
            tracing::error!(pack = %pack, deck = %label(deck_uid), error = %e.one_line(), "offered version unreadable");
        };
        let files = self.library.version(pack, version);
        let files = files
            .context(PackVersionMissingSnafu { id: pack, version: version.clone() })
            .map_err(crate::SessionError::from);
        let files = files.inspect_err(unreadable)?;
        let manifest =
            PackManifest::from_files(&files).context(PackManifestSnafu { id: pack }).map_err(crate::SessionError::from);
        let manifest = manifest.inspect_err(unreadable)?;
        self.end_widget_session();
        let pin = PackPin { id: pack, version: version.clone(), requires: manifest.requires };
        let label_text = self.localizer.tr("packs-undo-use-mine");
        self.apply(&label_text, vec![Op::AddPack { kind, namespace, pin }], None).inspect_err(
            |e| tracing::error!(pack = %pack, deck = %label(deck_uid), error = %e.one_line(), "offered version not pinned"),
        )
    }
}

#[cfg(test)]
#[path = "deck_offer_tests.rs"]
mod deck_offer_tests;
