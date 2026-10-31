//! The opener's offer when a pack or theme tab closes (t-1207).
//!
//! A library pack tab offers its opener deck the version it wrote (`Version`)
//! or, after a save as a copy, the switch to the copy (`Copy`); a theme tab
//! offers the version it wrote of the deck's active theme (`Theme`). The
//! deck decides whether each is due (`deck_offer.rs`). The tab's session and
//! the opener's are taken in sequence, never nested.

use std::sync::Mutex;

use serde::Serialize;
use snafu::OptionExt;
use uuid::Uuid;

use oku_slides_core::PackVersion;

use crate::SessionErrorKind;
use crate::deck_offer::DeckOffer;
use crate::pack_tab::PackWrite;
use crate::theme_tab_save::ThemeWrite;

use super::{Result, UnknownTabSnafu, Workspace};

/// What the opener deck is offered to take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum OfferKind {
    /// The version the pack tab wrote of a pack the deck pins.
    Version,
    /// The switch from pack `from`, which the deck pins, to the copy the tab saved.
    Copy {
        /// The original pack.
        from: Uuid,
    },
    /// The version the theme tab wrote of the deck's active theme.
    Theme,
}

/// The deck offered to take what a pack or theme tab wrote.
#[derive(Debug, Serialize)]
pub(crate) struct OpenerOffer {
    /// The opener deck's tab.
    pub tab: String,
    /// The opener deck's title and identity.
    #[serde(flatten)]
    pub deck: DeckOffer,
    /// What is offered.
    #[serde(flatten)]
    pub kind: OfferKind,
    /// The pack (or theme) the tab edited: the copy for `Copy`.
    pub pack: Uuid,
    /// That pack's or theme's name, already a label.
    pub pack_name: String,
    /// The version it wrote.
    pub version: PackVersion,
}

impl Workspace {
    /// The opener of pack or theme tab `tab`, with what it would take.
    ///
    /// `None` for a deck-pack tab: its saves go to its owner deck.
    ///
    /// # Errors
    ///
    /// `UnknownTab` for `tab`.
    ///
    /// # Blocking
    ///
    /// Reads two sessions and the library.
    pub(crate) fn pack_opener(ws: &Mutex<Self>, tab: &str) -> Result<Option<OpenerOffer>> {
        let opener = {
            let w = Self::lock(ws);
            let t = w.tabs.get(tab).context(UnknownTabSnafu { id: tab })?;
            t.opener.clone().filter(|_| t.info.document.deck_tab().is_none())
        };
        let Some(opener) = opener else { return Ok(None) };
        let (wrote, _changed) = Self::with_quiet(ws, tab, |s| (s.wrote_in_pack_tab(), s.wrote_in_theme_tab()));
        let (kind, pack, name, version) = match wrote? {
            (Some(PackWrite { pack, name, version, copied_from }), _) => {
                (copied_from.map_or(OfferKind::Version, |from| OfferKind::Copy { from }), pack, name, version)
            }
            (None, Some(ThemeWrite { theme, name, version })) => (OfferKind::Theme, theme, name, version),
            (None, None) => return Ok(None),
        };
        let (offer, _changed) = Self::with_quiet(ws, &opener, |s| match kind {
            OfferKind::Version => s.opener_offer(pack, &version),
            OfferKind::Copy { from } => s.opener_copy_offer(from, pack, &version),
            OfferKind::Theme => s.opener_theme_offer(pack, &version),
        });
        let deck = match offer {
            Ok(Some(deck)) => deck,
            Ok(None) => return Ok(None),
            Err(e) if e.kind() == SessionErrorKind::UnknownTab => return Ok(None),
            Err(e) => return Err(e),
        };
        Ok(Some(OpenerOffer { tab: opener, deck, kind, pack, pack_name: name, version }))
    }
}

#[cfg(test)]
#[path = "workspace_opener_tests.rs"]
mod workspace_opener_tests;
