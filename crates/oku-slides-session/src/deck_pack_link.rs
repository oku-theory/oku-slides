//! Owner side of deck-pack tabs: links, refused pin changes, undo stops.
//!
//! An owner deck keeps one `DeckLink` per deck-pack tab that edits one of its
//! deck-only packs, keyed by the tab's id. `link_deck_pack` records a pending
//! link and seeds the tab; the workspace layout then names its links
//! (`set_deck_links`), which also rebuilds them at resume. While a link stands,
//! pending or not, every change of the version the owner pins for its pack is
//! refused (`refuse_linked`, `refuse_linked_pins`, `PackOpenInTab`), and an undo
//! or redo that would make one is stopped (`linked_pins`, `stop_at_linked_pins`).
//! Receiving the tab's saves lives in `deck_pack_receive`.

use std::collections::BTreeMap;

use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{Deck, Namespace, PackFiles, PackPin, PackVersion, SourceBundle};

use crate::agent::bounds::label;
use crate::deck_closure::combined_pins;
use crate::error::{PackOpenInTabSnafu, SourcesSnafu, UndoInPackTabSnafu, UnknownPackSnafu, WrongDocumentSnafu};
use crate::pack::PackManifest;
use crate::pack_receive::deck_scope;
use crate::pack_tab::PreviewSeed;
use crate::{Mode, Result, Session};

/// What the owner must accept as a linked tab's parent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LinkParent {
    /// Fixed by the link's start or its last applied send.
    Known(Option<PackVersion>),
    /// Rebuilt at resume: the pin itself or its manifest's parent.
    Reseeded {
        /// The version pinned when the link was rebuilt.
        pin: PackVersion,
        /// That version's manifest parent.
        pin_parent: Option<PackVersion>,
    },
}

impl LinkParent {
    /// True when a send built on `parent` fits this link.
    #[must_use]
    pub(crate) fn accepts(&self, parent: Option<&PackVersion>) -> bool {
        match self {
            Self::Known(known) => known.as_ref() == parent,
            Self::Reseeded { pin, pin_parent } => parent == Some(pin) || parent == pin_parent.as_ref(),
        }
    }
}

/// A deck-pack tab linked to this owner deck, keyed by its tab id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeckLink {
    /// The tab's title, cleaned with `label`, for refusals.
    pub title: String,
    /// The pack it edits (a copy's id once accepted).
    pub pack: Uuid,
    /// This deck's `replica().deck_uid()` when the link was made.
    pub deck_uid: String,
    /// What the tab's sends must carry as parent.
    pub parent: LinkParent,
    /// The version this deck must pin for it now.
    pub expect: PackVersion,
    /// The last applied seq; `None` before the first.
    pub last_seq: Option<u64>,
    /// Recorded by `link_deck_pack`, not yet named by the layout.
    pub pending: bool,
}

/// A link as the workspace layout names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkSpec {
    /// The deck-pack tab's id.
    pub tab: String,
    /// Its cleaned title.
    pub title: String,
    /// The pack its cached document names.
    pub pack: Uuid,
}

/// A linked pack's pins in both kinds.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct LinkedPins {
    /// Its widgets pin, if any.
    pub widgets: Option<PackVersion>,
    /// Its layouts pin, if any.
    pub layouts: Option<PackVersion>,
}

/// The pins of `pack` in `deck`, in both kinds.
fn pins_of(deck: &Deck, pack: Uuid) -> LinkedPins {
    let find = |pins: &BTreeMap<Namespace, PackPin>| pins.values().find(|p| p.id == pack).map(|p| p.version.clone());
    LinkedPins { widgets: find(&deck.widgets), layouts: find(&deck.layouts) }
}

/// The entries of combined `pins` that pin `pack`, by namespace.
fn pins_by_id(pins: &BTreeMap<Namespace, PackPin>, pack: Uuid) -> Vec<(&Namespace, &PackPin)> {
    pins.iter().filter(|(_, pin)| pin.id == pack).collect()
}

impl Session {
    /// The sources of the user deck, whatever the mode sets aside.
    fn user_sources(&self) -> &SourceBundle {
        match &self.mode {
            Mode::Pack { preview, .. } => &preview.sources,
            Mode::History { stash, .. } => &stash.sources,
            Mode::Deck | Mode::Help { .. } | Mode::Theme { .. } => &self.doc.sources,
        }
    }

    /// The user deck's id, shared by its copies.
    #[must_use]
    pub(crate) fn user_deck_uid(&self) -> String {
        self.user_sources().replica().deck_uid()
    }

    /// The user deck's pin of deck-only `pack` and its files, when it may be linked.
    ///
    /// Fails closed: an unreadable scope, a missing version or a pack the
    /// library has a current version of gives `None`.
    pub(crate) fn linkable_pin(&self, pack: Uuid) -> Option<(Namespace, PackPin, PackFiles)> {
        let (_, ns, pin) = self.user_deck().all_pins().find(|(_, _, pin)| pin.id == pack)?;
        let files = self.user_sources().packs.get(&pin.version)?;
        let deck_only = deck_scope(files) == Some(true) && self.library.current(pack).is_none();
        deck_only.then(|| (ns.clone(), pin.clone(), files.clone()))
    }

    /// The name of the pack `link` edits, cleaned; its id when unreadable.
    pub(crate) fn linked_name(&self, link: &DeckLink) -> String {
        let manifest = self.user_sources().packs.get(&link.expect).and_then(|f| PackManifest::from_files(f).ok());
        manifest.map_or_else(|| link.pack.to_string(), |m| label(&m.name))
    }

    /// The refusal of a change to `link`'s pack while its tab edits it.
    fn linked_refusal(&self, link: &DeckLink) -> crate::SessionError {
        PackOpenInTabSnafu { pack: self.linked_name(link), tab: link.title.clone() }.build().into()
    }

    /// Starts linking `tab` to deck-only `pack`: records it pending, returns the seed.
    ///
    /// The deck's widget session ends first, as for `take_preview`. Another
    /// link already on `pack` is refused: the workspace focuses that tab first,
    /// this guards a race.
    ///
    /// # Errors
    ///
    /// `WrongDocument` outside deck mode; `UnknownPack` unless the deck pins
    /// `pack` deck-only and the library has no current version of it;
    /// `PackOpenInTab` when another tab already edits it; the `preview_seed` errors.
    ///
    /// # Blocking
    ///
    /// Freezes the widget draft in the library.
    pub(crate) fn link_deck_pack(&mut self, tab: &str, title: &str, pack: Uuid) -> Result<Box<PreviewSeed>> {
        ensure!(matches!(self.mode, Mode::Deck), WrongDocumentSnafu { kind: self.document_kind() });
        let (_, pin, _) = self.linkable_pin(pack).context(UnknownPackSnafu { id: pack })?;
        if let Some(other) = self.deck_links.values().find(|link| link.pack == pack) {
            return Err(self.linked_refusal(other));
        }
        self.end_widget_session();
        let seed = self.preview_seed()?;
        let link = DeckLink {
            title: label(title),
            pack,
            deck_uid: self.user_deck_uid(),
            parent: LinkParent::Known(Some(pin.version.clone())),
            expect: pin.version,
            last_seq: None,
            pending: true,
        };
        self.deck_links.insert(tab.to_owned(), link);
        Ok(seed)
    }

    /// Drops `tab`'s link, pending or not.
    pub(crate) fn unlink_deck_pack(&mut self, tab: &str) {
        self.deck_links.remove(tab);
    }

    /// Takes the layout's links; returns the tabs whose new link was refused.
    ///
    /// A call of an older layout `generation` changes nothing. A named tab
    /// keeps its link whatever pack the layout says, its title taken from the
    /// layout; a pending link stays until named or unlinked; a confirmed one
    /// no longer named goes. A newly named tab gets a link rebuilt from what
    /// this deck pins (`reseed`).
    #[must_use]
    pub(crate) fn set_deck_links(&mut self, generation: u64, links: &[LinkSpec]) -> Vec<String> {
        if generation < self.links_generation {
            return Vec::new();
        }
        self.links_generation = generation;
        self.deck_links.retain(|tab, link| {
            let named = links.iter().any(|spec| spec.tab == *tab);
            link.pending &= !named;
            named || link.pending
        });
        let mut refused = Vec::new();
        for spec in links {
            // A renamed tab: refusals name it as the bar does.
            if let Some(known) = self.deck_links.get_mut(&spec.tab) {
                known.title = label(&spec.title);
                continue;
            }
            if let Some(link) = self.reseed(spec) {
                self.deck_links.insert(spec.tab.clone(), link);
            } else {
                tracing::warn!(tab = %spec.tab, pack = %spec.pack, "deck-pack link not rebuilt: closing its tab");
                refused.push(spec.tab.clone());
            }
        }
        refused
    }

    /// A link rebuilt for `spec` from this deck's pin, or `None` (fail closed).
    fn reseed(&self, spec: &LinkSpec) -> Option<DeckLink> {
        if self.deck_links.values().any(|link| link.pack == spec.pack) {
            return None;
        }
        let (_, pin, files) = self.linkable_pin(spec.pack)?;
        let pin_parent = PackManifest::from_files(&files).ok()?.parent;
        Some(DeckLink {
            title: label(&spec.title),
            pack: spec.pack,
            deck_uid: self.user_deck_uid(),
            parent: LinkParent::Reseeded { pin: pin.version.clone(), pin_parent },
            expect: pin.version,
            last_seq: None,
            pending: false,
        })
    }

    /// Refuses a write of `pack` while a tab of this deck edits it.
    ///
    /// # Errors
    ///
    /// `PackOpenInTab` naming the tab, a pending link included.
    pub(crate) fn refuse_linked(&self, pack: Uuid) -> Result<()> {
        match self.deck_links.values().find(|link| link.pack == pack) {
            Some(link) => Err(self.linked_refusal(link)),
            None => Ok(()),
        }
    }

    /// Refuses combined `pins` when they move a linked pack's pin, by id.
    ///
    /// Compared with the user deck's pins, in deck mode and history only.
    ///
    /// # Errors
    ///
    /// `PackOpenInTab` naming the tab whose pack moved.
    pub(crate) fn refuse_linked_pins(&self, pins: &BTreeMap<Namespace, PackPin>) -> Result<()> {
        self.refuse_linked_pins_except(pins, None)
    }

    /// `refuse_linked_pins`, leaving out the link of tab `except`.
    pub(crate) fn refuse_linked_pins_except(
        &self,
        pins: &BTreeMap<Namespace, PackPin>,
        except: Option<&str>,
    ) -> Result<()> {
        if !matches!(self.mode, Mode::Deck | Mode::History { .. }) || self.deck_links.is_empty() {
            return Ok(());
        }
        let current = combined_pins(self.user_deck());
        let others = self.deck_links.iter().filter(|(tab, _)| Some(tab.as_str()) != except);
        for (_, link) in others {
            if pins_by_id(&current, link.pack) != pins_by_id(pins, link.pack) {
                return Err(self.linked_refusal(link));
            }
        }
        Ok(())
    }

    /// Refuses merging `theirs` when it would move a linked pack's pin.
    ///
    /// The merge is tried on copies of both documents; nothing changes here.
    ///
    /// # Errors
    ///
    /// `PackOpenInTab`, or the trial merge's errors (`MergeUnrelated`, `Sources`).
    pub(crate) fn refuse_linked_merge(&self, theirs: &SourceBundle) -> Result<()> {
        if self.deck_links.is_empty() {
            return Ok(());
        }
        let copy = |s: &SourceBundle| {
            let zip = s.to_zip().context(SourcesSnafu)?;
            let (bundle, _) =
                SourceBundle::from_zip(&zip, self.source_limits, self.registry.clone()).context(SourcesSnafu)?;
            Ok::<_, crate::SessionError>(bundle)
        };
        let mut trial = copy(&self.doc.sources)?;
        trial.merge(copy(theirs)?).map_err(crate::error::merge_error)?;
        let merged = trial.deck();
        for link in self.deck_links.values() {
            if pins_of(merged, link.pack) != pins_of(self.doc.sources.deck(), link.pack) {
                return Err(self.linked_refusal(link));
            }
        }
        self.refuse_linked_pins(&combined_pins(merged))
    }

    /// Each linked pack's pins in both kinds, deck mode only.
    #[must_use]
    pub(crate) fn linked_pins(&self) -> BTreeMap<Uuid, LinkedPins> {
        if !matches!(self.mode, Mode::Deck) {
            return BTreeMap::new();
        }
        let deck = self.doc.sources.deck();
        self.deck_links.values().map(|link| (link.pack, pins_of(deck, link.pack))).collect()
    }

    /// Refuses an undo or redo that moved a linked pack's pins from `before`.
    ///
    /// # Errors
    ///
    /// `UndoInPackTab` naming the first such pack and its tab.
    pub(crate) fn stop_at_linked_pins(&self, before: &BTreeMap<Uuid, LinkedPins>) -> Result<()> {
        let after = self.linked_pins();
        let moved = self.deck_links.values().find(|link| before.get(&link.pack) != after.get(&link.pack));
        match moved {
            Some(link) => UndoInPackTabSnafu { pack: self.linked_name(link), tab: link.title.clone() }.fail()?,
            None => Ok(()),
        }
    }

    /// Whether a turn end or history close asked for parked sends; clears it.
    pub(crate) fn take_link_pull(&mut self) -> bool {
        std::mem::take(&mut self.link_pull)
    }

    /// Asks for the links' parked sends at the next public call.
    ///
    /// Does nothing when this deck has no links.
    pub(crate) fn pull_links(&mut self) {
        self.link_pull |= !self.deck_links.is_empty();
    }
}

#[cfg(test)]
#[path = "deck_pack_link_tests.rs"]
mod deck_pack_link_tests;

#[cfg(test)]
#[path = "deck_pack_undo_tests.rs"]
mod deck_pack_undo_tests;

#[cfg(test)]
#[path = "deck_pack_refuse_tests.rs"]
mod deck_pack_refuse_tests;
