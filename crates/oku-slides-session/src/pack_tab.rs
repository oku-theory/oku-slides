//! Pack tabs: a library pack edited in its own tab, beside a preview deck.
//!
//! Opening a pack from a tab is routed first (`route_pack`): a library pack
//! opens a pack tab, whose session is built fresh and turned into one
//! (`init_pack_tab`) from a copy of the opener's deck (`take_preview`); a
//! deck-only pack of a deck opens a deck-pack tab linked to it (`DeckTab`). The preview carries the
//! deck's value, the pack and theme versions its pins reach and its fonts —
//! no file, no identity, no assets — and never adopts anything. The pack
//! tab's `Mode::Pack` holds the pack's lease.

use std::collections::BTreeMap;
use std::sync::Arc;

use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{Deck, PackFiles, PackKind, PackVersion, SourceBundle};

use crate::agent::bounds::label;
use crate::deck_closure::{combined_pins, pinned_versions};
use crate::error::{
    DeckOnlySnafu, InPackTabSnafu, NotEditingPackSnafu, PackTabNotFreshSnafu, PackVersionMissingSnafu,
    SingleDocumentSnafu, SourcesSnafu, UnknownWidgetSnafu,
};
use crate::pack_lease::{LeaseScope, PackLease};
use crate::pack_mode::layout_slide_id;
use crate::pack_mode_preview::preview_slide_id;
use crate::pack_receive::is_library_pack;
use crate::widgets_edit::parse_name;
use crate::{Document, Mode, Result, Session, TabDocument, TabInfo};

/// A deck copy for a pack tab's preview, without file, identity or assets.
pub(crate) struct PreviewSeed {
    /// The user deck's value.
    deck: Deck,
    /// The pack versions its pins reach, from the user blobs.
    packs: BTreeMap<PackVersion, PackFiles>,
    /// The theme versions its pins reach, from the user blobs.
    themes: BTreeMap<PackVersion, PackFiles>,
    /// Its fonts, whole (a deliberate cost).
    fonts: BTreeMap<String, Vec<u8>>,
}

impl std::fmt::Debug for PreviewSeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreviewSeed")
            .field("slides", &self.deck.slides.len())
            .field("packs", &self.packs.len())
            .field("themes", &self.themes.len())
            .field("fonts", &self.fonts.len())
            .finish()
    }
}

/// Where opening a pack from this tab goes.
#[derive(Debug)]
pub(crate) enum PackRoute {
    /// Done here: this tab edits the pack; its item selected, if one was named.
    Here,
    /// A library pack: open a pack tab; nothing changed here yet.
    Tab {
        /// The lease scope of this session's library.
        scope: LeaseScope,
        /// The pack's name, as a label.
        name: String,
    },
    /// A deck-only pack of this deck: its own tab, linked to it.
    DeckTab {
        /// The pack's name, cleaned with `label`.
        name: String,
    },
}

/// The pack version a pack tab wrote last.
#[derive(Debug, Clone)]
pub(crate) struct PackWrite {
    /// The edited pack.
    pub pack: Uuid,
    /// Its name, already a label.
    pub name: String,
    /// The version written.
    pub version: PackVersion,
}

/// True if `files` hold the item `name` (a bare name; a layout pack's layouts are its widgets).
///
/// `_commun` is an item whether or not the file exists.
fn has_widget(files: &PackFiles, name: &str) -> Result<bool> {
    let file = parse_name(name)?;
    Ok(file.is_common() || files.widget(&file).is_some())
}

impl Session {
    /// True when this session is a pack tab.
    #[must_use]
    pub(crate) fn is_pack_tab(&self) -> bool {
        matches!(self.mode, Mode::Pack { .. })
    }

    /// True for a pristine, file-less deck tab.
    ///
    /// Only such a tab may become a pack or theme tab.
    #[must_use]
    pub(crate) fn is_fresh(&self) -> bool {
        matches!(self.mode, Mode::Deck) && self.doc.path.is_none() && self.tab_info().pristine
    }

    /// Refuses opening any pack from this tab: help, history, a theme tab.
    ///
    /// The check every pack open shares, before its own (`route_pack`, a new pack).
    ///
    /// # Errors
    ///
    /// `ReadOnly`, `InHistory`, `WrongDocument { kind: Theme }`.
    pub(crate) fn can_open_pack(&self) -> Result<()> {
        self.ensure_editable()?;
        match self.mode {
            Mode::Theme { .. } => crate::document_kind::refused_in_theme_tab(),
            Mode::Deck | Mode::Help { .. } | Mode::Pack { .. } | Mode::History { .. } => Ok(()),
        }
    }

    /// Routes opening `pack` from this tab (decision 10's table).
    ///
    /// A deck tab opens a library pack in a pack tab, and a deck-only pack it
    /// pins in a deck-pack tab; a pack tab opens its own pack's widget here,
    /// another library pack in a pack tab, and refuses another deck-only one.
    /// Without a lease scope (single-document host) a pack opens nowhere but in
    /// the tab already editing it. Changes nothing unless it selects an item of
    /// the pack this tab edits.
    ///
    /// # Errors
    ///
    /// `ReadOnly`, `InHistory`, `UnknownPack`, `DeckOnly`, `UnknownWidget`,
    /// `InPackTab`, `SingleDocument` for a library pack without a lease scope.
    ///
    /// # Blocking
    ///
    /// Reads the library index; persists when it selects an item.
    pub(crate) fn route_pack(&mut self, pack: Uuid, widget: Option<&str>) -> Result<PackRoute> {
        self.can_open_pack()?;
        let stored = self.pack_base(pack)?;
        if let Some(w) = widget {
            ensure!(has_widget(&stored.files, w)?, UnknownWidgetSnafu { name: label(w) });
        }
        let has_scope = self.library.lease_scope().is_some();
        let deck_only = !is_library_pack(&stored.files);
        let scope = self.library.lease_scope().filter(|_| !deck_only).cloned();
        let tab = scope.map(|scope| PackRoute::Tab { scope, name: label(&stored.manifest.name) });
        let edited = match &self.mode {
            Mode::Pack { edit, .. } => Some(edit.pack == pack),
            // A theme tab opens no pack in place.
            Mode::Theme { .. } => return crate::document_kind::refused_in_theme_tab(),
            Mode::Deck | Mode::Help { .. } | Mode::History { .. } => None,
        };
        match (edited, tab) {
            (Some(true), _) => {
                if let Some(w) = widget {
                    self.select_item(w)?;
                }
                Ok(PackRoute::Here)
            }
            (_, Some(route)) => Ok(route),
            // A pack tab never opens another deck-only pack: that opens from its deck's tab.
            (Some(false), None) => InPackTabSnafu.fail()?,
            (None, None) if deck_only && has_scope => Ok(PackRoute::DeckTab { name: label(&stored.manifest.name) }),
            (None, None) if deck_only => DeckOnlySnafu { id: pack }.fail()?,
            // No lease scope: no tab to open a library pack in.
            (None, None) => SingleDocumentSnafu.fail()?,
        }
    }

    /// Ends the deck's widget session and copies it for `pack`'s new tab.
    ///
    /// # Errors
    ///
    /// The `route_pack` refusals when `pack` no longer routes to a tab (`DeckOnly`).
    ///
    /// # Blocking
    ///
    /// Freezes the widget draft in the library.
    pub(crate) fn take_preview(&mut self, pack: Uuid) -> Result<Box<PreviewSeed>> {
        self.ensure_editable()?;
        let stored = self.pack_base(pack)?;
        let routes_to_tab = self.library.lease_scope().is_some() && is_library_pack(&stored.files);
        ensure!(routes_to_tab, DeckOnlySnafu { id: pack });
        self.preview_seed()
    }

    /// The seed of a preview deck copied from this deck.
    ///
    /// The deck's widget session ends first, so its draft is frozen.
    ///
    /// # Errors
    ///
    /// `ReadOnly`, `InHistory` and the other refusals of `ensure_editable`.
    ///
    /// # Blocking
    ///
    /// Freezes the widget draft in the library.
    pub(crate) fn preview_seed(&mut self) -> Result<Box<PreviewSeed>> {
        self.ensure_editable()?;
        if matches!(self.mode, Mode::Deck) {
            // The deck's draft is frozen before it is copied.
            self.end_widget_session();
        }
        let deck = self.user_deck().clone();
        let reach = pinned_versions(&deck);
        let blobs = self.user_blobs();
        let reached = |all: &BTreeMap<PackVersion, PackFiles>| {
            all.iter().filter(|(v, _)| reach.contains(*v)).map(|(v, f)| (v.clone(), f.clone())).collect()
        };
        let (packs, themes) = (reached(&blobs.packs), reached(&blobs.themes));
        Ok(Box::new(PreviewSeed { deck, packs, themes, fonts: blobs.fonts.clone() }))
    }

    /// Turns this freshly built session into a pack tab on `pack`.
    ///
    /// The preview is installed from `seed` (never adopting), then pack mode
    /// entered with `lease`. Resume is turned on by the caller.
    ///
    /// # Errors
    ///
    /// `PackTabNotFresh`, `DeckOnly`, `Sources`, `LeaseMissing`, and the `enter_pack` errors.
    ///
    /// # Blocking
    ///
    /// Resolves the preview's widgets and renders the working deck.
    pub(crate) fn init_pack_tab(
        &mut self,
        seed: PreviewSeed,
        lease: PackLease,
        pack: Uuid,
        widget: Option<&str>,
    ) -> Result<()> {
        self.install_preview(seed)?;
        self.enter_pack(pack, Some(lease), None)?;
        if let Some(w) = widget {
            self.select_item(w)?;
        }
        Ok(())
    }

    /// Records that this pack or theme tab created its document empty, as `id`.
    ///
    /// Set once, before resume is armed; never cleared (a copy saved since keeps it).
    pub(crate) fn mark_created_empty(&mut self, id: Uuid) {
        match &mut self.mode {
            Mode::Pack { edit, .. } => edit.created_empty = Some(id),
            Mode::Theme { edit, .. } => edit.created_empty = Some(id),
            Mode::Deck | Mode::Help { .. } | Mode::History { .. } => {}
        }
    }

    /// Installs `seed` as the document of this fresh session, never adopting.
    ///
    /// # Errors
    ///
    /// `PackTabNotFresh` unless this session is a pristine deck tab; `Sources`
    /// and the widget resolution errors.
    ///
    /// # Blocking
    ///
    /// Resolves the preview's widgets.
    pub(crate) fn install_preview(&mut self, seed: PreviewSeed) -> Result<()> {
        ensure!(self.is_fresh(), PackTabNotFreshSnafu);
        let PreviewSeed { deck, packs, themes, fonts } = seed;
        let (widgets, notice) = self.resolve_widgets(&combined_pins(&deck), &packs)?;
        let mut sources = SourceBundle::new(deck, Arc::clone(&widgets.registry)).context(SourcesSnafu)?;
        (sources.packs, sources.themes, sources.fonts) = (packs, themes, fonts);
        let doc = Document { blank: false, dirty: false, ..Document::new(sources, None, &self.doc_defaults) };
        self.install_document(doc, widgets, notice)
    }

    /// Turns this fresh session into the tab of library pack `pack`.
    ///
    /// The preview is this session's own deck; the tab holds the pack's lease.
    ///
    /// # Errors
    ///
    /// `UnknownPack`; `DeckOnly` for a deck-only pack, or when the library has no lease scope;
    /// `PackTabNotFresh`, `PackOpenInTab`, the `init_pack_tab` errors.
    ///
    /// # Blocking
    ///
    /// Reads the library, takes `DRAFTS` for the lease, renders the preview.
    pub fn open_pack_tab_here(&mut self, pack: Uuid, widget: Option<&str>) -> Result<()> {
        let stored = self.pack_base(pack)?;
        let scope = self.library.lease_scope().filter(|_| is_library_pack(&stored.files)).cloned();
        let scope = scope.context(DeckOnlySnafu { id: pack })?;
        ensure!(self.is_fresh(), PackTabNotFreshSnafu);
        let name = label(&stored.manifest.name);
        let lease = PackLease::acquire(&scope, pack, &name, &name)?;
        let seed = self.take_preview(pack)?;
        self.init_pack_tab(*seed, lease, pack, widget)
    }

    /// Selects `widget` in this pack tab and persists it; the host's view follows.
    ///
    /// Sets `ui.widget`, and `ui.slide` to the item's slide when the working
    /// deck has it (not for `_commun`).
    ///
    /// # Errors
    ///
    /// `NotEditingPack`, `UnknownWidget`, `InvalidWidgetName`.
    ///
    /// # Blocking
    ///
    /// Persists the UI state (writes `state.json`).
    pub fn select_item(&mut self, widget: &str) -> Result<()> {
        let Mode::Pack { edit, .. } = &self.mode else { return NotEditingPackSnafu.fail()? };
        let files = self.doc.sources.packs.get(&edit.base);
        let files = files.context(PackVersionMissingSnafu { id: edit.pack, version: edit.base.clone() })?;
        ensure!(has_widget(files, widget)?, UnknownWidgetSnafu { name: label(widget) });
        let file = parse_name(widget)?;
        let slide = match files.kind() {
            PackKind::Layouts => layout_slide_id(&file),
            PackKind::Widgets => preview_slide_id(&file),
        };
        let slide = (!file.is_common() && self.deck().slides.iter().any(|s| s.id == slide)).then_some(slide);
        let mut ui = std::mem::take(&mut self.ui);
        ui.widget = Some(widget.to_owned());
        if slide.is_some() {
            ui.slide = slide;
        }
        self.ui = ui.sanitized(true);
        self.persist(false);
        Ok(())
    }

    /// The name of the pack this tab edits.
    ///
    /// # Errors
    ///
    /// `NotEditingPack` outside a pack tab.
    pub fn edited_pack_name(&self) -> Result<String> {
        let Mode::Pack { edit, .. } = &self.mode else { return NotEditingPackSnafu.fail()? };
        Ok(edit.name.clone())
    }

    /// This pack tab's info: its pack, label and preview theme; `None` outside a pack tab.
    ///
    /// A deck-pack tab names its owner and, until its owner acknowledges a
    /// save, the pack that save was built on (never an unaccepted copy).
    #[must_use]
    pub(crate) fn pack_tab_info(&self) -> Option<TabInfo> {
        let Mode::Pack { preview, edit, link, .. } = &self.mode else { return None };
        let preview = preview.sources.deck();
        let document = match link {
            Some(link) => TabDocument::DeckPack { deck_tab: link.owner.clone(), pack: link.reported_pack(edit.pack) },
            None => TabDocument::Pack { pack: edit.pack },
        };
        Some(TabInfo {
            title: label(&edit.name),
            dirty: self.doc.dirty || self.unsent(),
            pristine: false,
            theme: preview.theme.as_ref().and_then(|ns| preview.themes.get(ns)).map(|pin| pin.id),
            document,
            created: edit.created_empty,
        })
    }

    /// The version this pack tab wrote last, if it wrote one.
    #[must_use]
    pub(crate) fn wrote_in_pack_tab(&self) -> Option<PackWrite> {
        let Mode::Pack { edit, .. } = &self.mode else { return None };
        let version = edit.session_version.clone()?;
        Some(PackWrite { pack: edit.pack, name: label(&edit.name), version })
    }
}

#[cfg(test)]
#[path = "pack_tab_route_tests.rs"]
mod pack_tab_route_tests;

#[cfg(test)]
#[path = "pack_tab_open_tests.rs"]
mod pack_tab_open_tests;

#[cfg(test)]
#[path = "pack_tab_tests.rs"]
mod pack_tab_tests;

#[cfg(test)]
#[path = "pack_tab_lease_tests.rs"]
mod pack_tab_lease_tests;

#[cfg(test)]
#[path = "pack_tab_read_only_tests.rs"]
mod pack_tab_read_only_tests;
