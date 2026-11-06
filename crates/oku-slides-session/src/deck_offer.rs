//! The opener's offer: a deck takes the pack version its pack tab wrote.
//!
//! When a pack tab closes, the deck it was opened from may pin an older
//! version of the pack. It is offered the version the tab wrote, only while
//! that version is the library's current one, newer than the deck's, and its
//! requirements are pinned by the deck. Taking it pins exactly that version,
//! rechecked at that moment (`use_pack_version`), in one undo step.
//!
//! A tab that saved a copy of the deck's pack offers the switch to the copy
//! (`use_pack_copy`): its pin replaces the original's and the slides follow
//! its namespace. A theme tab offers the version it wrote of the deck's active
//! theme (`use_theme_version`). Each offer is rechecked when taken (t-1207).

use serde::Serialize;
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{Namespace, Op, PackFiles, PackKind, PackPin, PackVersion, ThemeManifest, remove_pack_refusal};
use oku_slides_i18n::Localize;

use crate::agent::bounds::label;
use crate::deck_closure::{closure_with_files, combined_pins};
use crate::deck_packs::Drift;
use crate::error::{PackManifestSnafu, PackOfferExpiredSnafu, PackVersionMissingSnafu, SourcesSnafu, ThemeJsonSnafu};
use crate::namespace_slug::rename_namespace;
use crate::pack::PackManifest;
use crate::pack_mode::{embed_packs, pin_deck};
use crate::sources_budget::new_packs_bytes;
use crate::theme_closure::{closure_with, library_theme, pin_of};
use crate::{Mode, Result, Session};

/// A checked switch from pack `from` to its copy: what `use_pack_copy` writes.
struct CopySwitch {
    /// The original's namespace, which the slides leave.
    from_ns: Namespace,
    /// The copy's pin, under its own namespace.
    pin: PackPin,
    /// The copy's namespace.
    to_ns: Namespace,
    /// The copy's files.
    files: PackFiles,
    /// The copy's name.
    name: String,
}

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

    /// This deck's offer to switch from `from` to `version` of its copy, if due.
    ///
    /// Due in deck mode when the deck pins `from`, `version` is the copy's
    /// current library version, of the same kind, its namespace free in the
    /// deck (but for `from`'s own) and its requirements pinned. Logged and
    /// `None` when the copy cannot be read.
    ///
    /// # Blocking
    ///
    /// Reads the library index and the copy's manifest.
    pub(crate) fn opener_copy_offer(&self, from: Uuid, copy: Uuid, version: &PackVersion) -> Option<DeckOffer> {
        self.copy_switch(from, copy, version)?;
        Some(DeckOffer {
            title: label(&self.document_name(&self.doc)),
            deck_uid: self.doc.sources.replica().deck_uid(),
        })
    }

    /// The switch from `from` to `version` of `copy`, when `opener_copy_offer` would offer it.
    fn copy_switch(&self, from: Uuid, copy: Uuid, version: &PackVersion) -> Option<CopySwitch> {
        if !matches!(self.mode, Mode::Deck) || from == copy || self.library.current(copy).as_ref() != Some(version) {
            return None;
        }
        let (kind, from_ns, _) = self.pack_pin(from)?;
        let Some(files) = self.library.version(copy, version) else {
            tracing::warn!(pack = %copy, version = %version, "offered copy unreadable: no offer");
            return None;
        };
        let manifest = match PackManifest::from_files(&files) {
            Ok(manifest) => manifest,
            Err(e) => {
                tracing::warn!(pack = %copy, version = %version, error = %e, "offered copy manifest unreadable: no offer");
                return None;
            }
        };
        let deck = self.doc.sources.deck();
        let to_ns = manifest.namespace.clone();
        let other = match kind {
            PackKind::Widgets => PackKind::Layouts,
            PackKind::Layouts => PackKind::Widgets,
        };
        let free =
            to_ns == from_ns || (!deck.pins(kind).contains_key(&to_ns) && !deck.pins(other).contains_key(&to_ns));
        // The rule of `Op::AddPack`, the original's namespace leaving the deck.
        let met = manifest
            .requires
            .iter()
            .all(|r| (kind == PackKind::Widgets && *r == to_ns) || (*r != from_ns && deck.widgets.contains_key(r)));
        // The original leaves only as `Op::RemovePack` lets it, its slides renamed first.
        let leaves = to_ns == from_ns || {
            let mut renamed = deck.clone();
            for slide in &mut renamed.slides {
                rename_namespace(slide, &from_ns, &to_ns);
            }
            remove_pack_refusal(&renamed, kind, &from_ns).is_none()
        };
        (files.kind() == kind && manifest.id == copy && free && met && leaves).then(|| CopySwitch {
            from_ns,
            pin: PackPin { id: copy, version: version.clone(), requires: manifest.requires.clone() },
            to_ns,
            files,
            name: manifest.name,
        })
    }

    /// The deck switches from `from` to `version` of its copy, if the offer holds.
    ///
    /// Rechecked as `opener_copy_offer`, in the tab still holding deck
    /// `deck_uid`; the original's pin goes, the copy's comes with its
    /// requirements, and every slide and element of `from`'s namespace takes
    /// the copy's, in one undo step. A render that fails afterwards is a
    /// notice, as for a received deck-pack save: the step stays undoable.
    ///
    /// # Errors
    ///
    /// `PackOfferExpired`; `ReadOnly`, `EditingPack`, `InHistory` (`ensure_deck_mode`);
    /// `UndoInPackTab` while a deck-pack tab edits a moved pack; the library's and sources' errors.
    ///
    /// # Blocking
    ///
    /// Reads the library.
    pub(crate) fn use_pack_copy(
        &mut self,
        from: Uuid,
        copy: Uuid,
        version: &PackVersion,
        deck_uid: &str,
    ) -> Result<()> {
        self.ensure_deck_mode()?;
        ensure!(self.doc.sources.replica().deck_uid() == deck_uid, PackOfferExpiredSnafu);
        let switch = self.copy_switch(from, copy, version).context(PackOfferExpiredSnafu)?;
        let mut deck = self.doc.sources.deck().clone();
        deck.widgets.retain(|_, pin| pin.id != from);
        deck.layouts.retain(|_, pin| pin.id != from);
        if switch.to_ns != switch.from_ns {
            for slide in &mut deck.slides {
                rename_namespace(slide, &switch.from_ns, &switch.to_ns);
            }
        }
        let edited = (switch.to_ns.clone(), switch.pin, switch.files);
        let packs = closure_with_files(&self.library, &deck, edited, &self.doc.sources.packs)?;
        pin_deck(&mut deck, &packs)?;
        self.refuse_linked_pins(&combined_pins(&deck))?;
        self.ensure_sources_room(new_packs_bytes(&self.doc.sources.packs, &packs))?;
        self.end_widget_session();
        embed_packs(&mut self.doc.sources, packs);
        let label_text = self.localizer.tr_with("pack-adopt-label", &[("name", switch.name.as_str().into())]);
        self.doc.sources.apply(deck, &label_text, None).context(SourcesSnafu)?;
        self.mark_dirty();
        self.sync_widgets()?;
        self.refresh_pack_updates();
        match self.rerender() {
            Err(e) if e.render_error().is_some() => {
                tracing::warn!(error = %e.one_line(), "copy pinned, render failed");
                self.set_notice(e.localize(&self.localizer));
                Ok(())
            }
            other => other,
        }
    }

    /// This deck's offer to take `version` of theme `theme`, its active one, if due.
    ///
    /// Due in deck mode when the deck's active theme is `theme`, at a version
    /// older than `version`, which is the library's current one.
    ///
    /// # Blocking
    ///
    /// Reads the theme library.
    pub(crate) fn opener_theme_offer(&self, theme: Uuid, version: &PackVersion) -> Option<DeckOffer> {
        self.theme_due(theme, version).then(|| DeckOffer {
            title: label(&self.document_name(&self.doc)),
            deck_uid: self.doc.sources.replica().deck_uid(),
        })
    }

    /// True when `opener_theme_offer` would offer `version` of `theme`.
    fn theme_due(&self, theme: Uuid, version: &PackVersion) -> bool {
        matches!(self.mode, Mode::Deck)
            && self.deck_theme_pin().is_some_and(|pin| pin.id == theme)
            && self.themes.current(theme).as_ref() == Some(version)
            && self.theme_relation() == Some(Drift::Older)
    }

    /// The deck takes `version` of its active theme `theme`, if the offer still holds.
    ///
    /// Rechecked as `opener_theme_offer`, in the tab still holding deck
    /// `deck_uid`; then exactly `version` is pinned ([`Self::pin_theme_version`]),
    /// even if another tab saved a newer one since the check (t-1378).
    ///
    /// # Errors
    ///
    /// `PackOfferExpired`, and those of `pin_theme_version`.
    pub(crate) fn use_theme_version(&mut self, theme: Uuid, version: &PackVersion, deck_uid: &str) -> Result<()> {
        self.ensure_deck_mode()?;
        let same_deck = self.doc.sources.replica().deck_uid() == deck_uid;
        ensure!(same_deck && self.theme_due(theme, version), PackOfferExpiredSnafu);
        self.pin_theme_version(theme, version)
    }

    /// Pins exactly `version` of the active theme, the rest of its closure current.
    ///
    /// One undo step. The packs panel's theme update reads the library's current version when the op applies;
    /// an offer names its version, which a save in between must not replace.
    ///
    /// # Errors
    ///
    /// `PackOfferExpired` when `version` is gone or is not the active theme's; `ThemeJson` for an
    /// unreadable manifest; the closure's and `apply`'s errors.
    ///
    /// # Blocking
    ///
    /// Reads the theme library.
    pub(crate) fn pin_theme_version(&mut self, theme: Uuid, version: &PackVersion) -> Result<()> {
        let ns = self.active_theme()?;
        let files = self.themes.version(theme, version).context(PackOfferExpiredSnafu)?;
        let manifest = ThemeManifest::from_files(&files).context(ThemeJsonSnafu { id: theme })?;
        ensure!(manifest.id == theme && manifest.namespace == ns, PackOfferExpiredSnafu);
        let offered = pin_of(&manifest, version.clone());
        let (pins, _) = closure_with(&ns, |n| {
            if *n == ns { Some((offered.clone(), files.clone())) } else { library_theme(&self.themes, n) }
        })?;
        let name = label(&manifest.name);
        let label_text = self.localizer.tr_with("packs-undo-theme-update", &[("name", name.as_str().into())]);
        self.apply(&label_text, vec![Op::SetTheme { theme: Some(ns), themes: pins }], None)
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

#[cfg(test)]
#[path = "deck_offer_switch_tests.rs"]
mod deck_offer_switch_tests;
