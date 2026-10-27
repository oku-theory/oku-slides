//! Pack tabs: a library pack edited in its own tab, beside a preview deck.
//!
//! Opening a pack from a tab is routed first (`route_pack`): a deck-only
//! pack's workshop is entered in place, as today; a library pack opens a pack
//! tab, whose session is built fresh and turned into one (`init_pack_tab`)
//! from a copy of the opener's deck (`take_preview`). The preview carries the
//! deck's value, the pack and theme versions its pins reach and its fonts —
//! no file, no identity, no assets — and never adopts anything. The pack
//! tab's `Mode::Pack` has host `Tab` and holds the pack's lease.

use std::collections::BTreeMap;
use std::sync::Arc;

use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{Deck, PackFiles, PackVersion, SourceBundle};

use crate::agent::bounds::label;
use crate::deck_closure::{combined_pins, pinned_versions};
use crate::error::{
    DeckOnlySnafu, InPackTabSnafu, NotEditingPackSnafu, PackTabNotFreshSnafu, PackVersionMissingSnafu, SourcesSnafu,
    UnknownWidgetSnafu,
};
use crate::pack_lease::{LeaseScope, PackLease};
use crate::pack_receive::is_library_pack;
use crate::views::{MAX_WIDGET_TABS, PackHost};
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
    /// Done here: the workshop entered, or a sub-tab of the edited pack opened.
    Here {
        /// True when a deck-only pack's workshop was entered.
        entered: bool,
    },
    /// A library pack: open a pack tab; nothing changed here yet.
    Tab {
        /// The lease scope of this session's library.
        scope: LeaseScope,
        /// The pack's name, as a label.
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

/// True if `files` hold the widget `name` (a bare name; a layout pack's layouts are its widgets).
///
/// `_commun` is never a widget one opens.
fn has_widget(files: &PackFiles, name: &str) -> Result<bool> {
    let file = parse_name(name)?;
    Ok(!file.is_common() && files.widget(&file).is_some())
}

/// Makes room for one more sub-tab in `widgets`, capped at `cap`.
///
/// The oldest sub-tab other than `active` goes; the active one only when it is the last.
fn make_room(widgets: &mut Vec<String>, active: Option<&str>, cap: usize) {
    while !widgets.is_empty() && widgets.len() >= cap {
        let oldest = widgets.iter().position(|w| Some(w.as_str()) != active).unwrap_or(0);
        widgets.remove(oldest);
    }
}

impl Session {
    /// True when this session is a pack tab.
    #[must_use]
    pub(crate) fn is_pack_tab(&self) -> bool {
        matches!(&self.mode, Mode::Pack { edit, .. } if edit.host == PackHost::Tab)
    }

    /// Routes opening `pack` from this tab; enters a deck-only workshop here.
    ///
    /// Changes nothing unless it enters a deck-only pack's workshop, or opens
    /// a sub-tab of the pack this tab edits.
    ///
    /// # Errors
    ///
    /// `ReadOnly`, `InHistory`, `UnknownPack`, `DeckOnly`, `UnknownWidget`, `InPackTab`.
    ///
    /// # Blocking
    ///
    /// Reads the library index; renders when it enters a workshop.
    pub(crate) fn route_pack(&mut self, pack: Uuid, widget: Option<&str>) -> Result<PackRoute> {
        self.ensure_editable()?;
        let stored = self.pack_base(pack)?;
        if let Some(w) = widget {
            ensure!(has_widget(&stored.files, w)?, UnknownWidgetSnafu { name: label(w) });
        }
        let scope = self.library.lease_scope().filter(|_| is_library_pack(&stored.files)).cloned();
        let tab = scope.map(|scope| PackRoute::Tab { scope, name: label(&stored.manifest.name) });
        let edited = match &self.mode {
            Mode::Pack { edit, .. } => Some((edit.pack == pack, edit.host)),
            // A theme tab opens no pack in place.
            Mode::Theme { .. } => return crate::document_kind::refused_in_theme_tab(),
            Mode::Deck | Mode::Help { .. } | Mode::History { .. } => None,
        };
        match (edited, tab) {
            (Some((true, _)), _) => {
                if let Some(w) = widget {
                    self.open_widget_tab(w)?;
                }
                Ok(PackRoute::Here { entered: false })
            }
            (_, Some(route)) => Ok(route),
            // A pack tab never enters a deck-only workshop: that opens from its deck's tab.
            (Some((false, PackHost::Tab)), None) => InPackTabSnafu.fail()?,
            (Some((false, PackHost::Deck)) | None, None) => {
                self.enter_here(pack, widget)?;
                Ok(PackRoute::Here { entered: true })
            }
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

    /// Enters `pack`'s workshop in this tab, then opens `widget` there.
    ///
    /// The notice `edit_pack` set stays for the next snapshot.
    ///
    /// # Errors
    ///
    /// As `edit_pack`, and `open_widget_tab`'s.
    ///
    /// # Blocking
    ///
    /// As `edit_pack`.
    fn enter_here(&mut self, pack: Uuid, widget: Option<&str>) -> Result<()> {
        self.end_widget_session();
        self.edit_pack(pack)?;
        self.persist(true);
        if let Some(w) = widget {
            self.open_widget_tab(w)?;
        }
        Ok(())
    }

    /// Turns this freshly built session into a pack tab on `pack`.
    ///
    /// The preview is installed from `seed` (never adopting), then the
    /// workshop entered with `lease`. Resume is turned on by the caller.
    ///
    /// # Errors
    ///
    /// `PackTabNotFresh`, `DeckOnly`, `Sources`, `LeaseMissing`, and the `enter_pack` errors.
    ///
    /// # Blocking
    ///
    /// Resolves the preview's widgets and renders the workshop.
    pub(crate) fn init_pack_tab(
        &mut self,
        seed: PreviewSeed,
        lease: PackLease,
        pack: Uuid,
        widget: Option<&str>,
    ) -> Result<()> {
        let fresh = matches!(self.mode, Mode::Deck) && self.doc.path.is_none() && self.tab_info().pristine;
        ensure!(fresh, PackTabNotFreshSnafu);
        let PreviewSeed { deck, packs, themes, fonts } = seed;
        let (widgets, notice) = self.resolve_widgets(&combined_pins(&deck), &packs)?;
        let mut sources = SourceBundle::new(deck, Arc::clone(&widgets.registry)).context(SourcesSnafu)?;
        (sources.packs, sources.themes, sources.fonts) = (packs, themes, fonts);
        let doc = Document { blank: false, dirty: false, ..Document::new(sources, None, &self.doc_defaults) };
        self.install_document(doc, widgets, notice)?;
        self.enter_pack(pack, PackHost::Tab, Some(lease))?;
        if let Some(w) = widget {
            self.open_widget_tab(w)?;
        }
        Ok(())
    }

    /// Opens `pack` in this session, for a host without tabs.
    ///
    /// # Errors
    ///
    /// The `route_pack` and `edit_pack` errors; `InPackTab` in a pack tab.
    ///
    /// # Blocking
    ///
    /// As `edit_pack`.
    pub fn open_pack_here(&mut self, pack: Uuid, widget: Option<&str>) -> Result<()> {
        match self.route_pack(pack, widget)? {
            PackRoute::Here { .. } => Ok(()),
            PackRoute::Tab { .. } => self.enter_here(pack, widget),
        }
    }

    /// Opens `widget`'s sub-tab in the pack this tab edits, then persists.
    ///
    /// # Errors
    ///
    /// `NotEditingPack`, `UnknownWidget`, `InvalidWidgetName`.
    ///
    /// # Blocking
    ///
    /// Persists the UI state (writes `state.json`).
    pub(crate) fn open_widget_tab(&mut self, widget: &str) -> Result<()> {
        let Mode::Pack { edit, .. } = &self.mode else { return NotEditingPackSnafu.fail()? };
        let files = self.doc.sources.packs.get(&edit.base);
        let files = files.context(PackVersionMissingSnafu { id: edit.pack, version: edit.base.clone() })?;
        ensure!(has_widget(files, widget)?, UnknownWidgetSnafu { name: label(widget) });
        let mut ui = std::mem::take(&mut self.ui);
        if !ui.widgets.iter().any(|w| w == widget) {
            make_room(&mut ui.widgets, ui.widget.as_deref(), MAX_WIDGET_TABS);
            ui.widgets.push(widget.to_owned());
        }
        ui.widget = Some(widget.to_owned());
        self.ui = ui.sanitized(true);
        self.persist(false);
        Ok(())
    }

    /// This pack tab's info: its pack, label and preview theme; `None` outside a pack tab.
    #[must_use]
    pub(crate) fn pack_tab_info(&self) -> Option<TabInfo> {
        let Mode::Pack { stash, edit, .. } = &self.mode else { return None };
        if edit.host != PackHost::Tab {
            return None;
        }
        let preview = stash.sources.deck();
        Some(TabInfo {
            title: label(&edit.name),
            dirty: self.doc.dirty,
            pristine: false,
            editing_pack: true,
            theme: preview.theme.as_ref().and_then(|ns| preview.themes.get(ns)).map(|pin| pin.id),
            document: TabDocument::Pack { pack: edit.pack },
        })
    }

    /// The version this pack tab wrote last, if it wrote one.
    #[must_use]
    pub(crate) fn wrote_in_pack_tab(&self) -> Option<PackWrite> {
        let Mode::Pack { edit, .. } = &self.mode else { return None };
        let version = edit.session_version.clone().filter(|_| edit.host == PackHost::Tab)?;
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
