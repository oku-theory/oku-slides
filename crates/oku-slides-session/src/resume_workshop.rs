//! Resuming a pack state by version and host; old workshops reopen on their deck.
//!
//! Older versions edited a pack in a workshop inside its deck tab. Such a
//! state (version 3, or version 5 with host `deck`) resumes in deck mode:
//! `deck/` is the document, `pack/` only tells whether pack edits are
//! dropped (`notice-workshop-reopened-on-deck`), and no lease is taken.
//! Every other pack state is a pack tab (`resume_pack.rs`), or unreadable.
//! This build writes pack states without host.

use oku_slides_core::PackVersion;
use serde::Deserialize;
use snafu::ensure;

use super::{
    EDITOR_STATE_VERSION, PACK_DIR, PACK_TAB_STATE_VERSION, PackTabSnafu, Result, Resume, SavedMode, State, VERSION,
    pack,
};
use crate::views::PackMode;
use crate::{Session, TabDocument};

/// Where an older state edited its pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum LegacyHost {
    /// In its deck tab's workshop.
    Deck,
    /// In its own tab.
    Tab,
}

/// What a saved pack state resumes as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PackResume {
    /// A deck with its workshop open: reopened on the deck.
    Workshop,
    /// A pack tab.
    PackTab,
    /// Neither: set aside as unreadable.
    Unreadable,
}

/// Classifies a pack state by version and saved host.
///
/// v3 is a workshop; v4 a pack tab only with host `tab`; v5 a workshop
/// only with host `deck`, else a pack tab.
pub(super) fn classify(version: u32, legacy: Option<LegacyHost>) -> PackResume {
    match (version, legacy) {
        (VERSION, _) | (EDITOR_STATE_VERSION, Some(LegacyHost::Deck)) => PackResume::Workshop,
        (PACK_TAB_STATE_VERSION, Some(LegacyHost::Tab)) | (EDITOR_STATE_VERSION, _) => PackResume::PackTab,
        _ => PackResume::Unreadable,
    }
}

/// Resumes a pack state on its deck, as a pack tab, or refuses it.
///
/// # Errors
///
/// `PackTab` for an unreadable state; those of `reopen_on_deck` and `restore_pack_tab`.
///
/// # Blocking
///
/// Reads the tab folder and the library, and renders.
pub(super) fn resume_pack_state(
    s: &mut Session,
    resume: &Resume,
    state: State,
    cached: Option<&TabDocument>,
) -> Result<()> {
    let SavedMode::Pack(saved) = &state.mode else { return PackTabSnafu { reason: "not a pack state" }.fail() };
    match classify(state.version, saved.legacy) {
        PackResume::Workshop => reopen_on_deck(s, resume, state, cached),
        PackResume::PackTab => pack::restore_pack_tab(s, resume, state, cached),
        PackResume::Unreadable => PackTabSnafu { reason: "version 4 state of a tab that is not a pack tab" }.fail(),
    }
}

/// Restores a workshop's deck in deck mode, noticing dropped pack edits.
///
/// `cached` must be a deck's document, or none (fail closed). The edits
/// are dropped when `pack/` is dirty or unreadable, or when the version it
/// edited is not kept (`dropped_edits`).
///
/// # Errors
///
/// `PackTab` for a state that is not a pack state or a cached document of
/// another kind; the deck's own errors.
///
/// # Blocking
///
/// Reads the tab folder and renders.
pub(super) fn reopen_on_deck(
    s: &mut Session,
    resume: &Resume,
    state: State,
    cached: Option<&TabDocument>,
) -> Result<()> {
    let State { mode: SavedMode::Pack(saved), ui, .. } = state else {
        return PackTabSnafu { reason: "not a pack state" }.fail();
    };
    ensure!(
        matches!(cached, None | Some(TabDocument::Deck { .. })),
        PackTabSnafu { reason: "workshop state of a tab that is not a deck" }
    );
    s.restore_deck(resume, ui)?;
    let dropped = dropped_edits(s, resume, &saved.edit);
    tracing::info!(pack = %saved.edit.pack, dropped, "workshop of an older version reopened on its deck");
    if dropped {
        s.set_notice(s.localizer.tr("notice-workshop-reopened-on-deck"));
    }
    Ok(())
}

/// Whether reopening on the deck loses pack edits.
///
/// Lost when `pack/` is unreadable or dirty. A library pack's session
/// version lives in the library, so only a pin moved off `edit.base`
/// counts; a deck-only pack's edited version must be the one the deck pins.
fn dropped_edits(s: &Session, resume: &Resume, edit: &PackMode) -> bool {
    let doc = match resume.read_doc(PACK_DIR, &s.doc_defaults, s.source_limits.replica(), &s.registry) {
        Ok(doc) => doc,
        Err(e) => {
            let error = oku_slides_platform::errors::one_line(&e);
            tracing::warn!(error = %error, "unreadable workshop pack: its edits count as dropped");
            return true;
        }
    };
    let edited = pinned_version(doc.sources.deck(), edit);
    let moved = if edit.deck_only {
        let edited = edit.session_version.clone().or(edited);
        edited != pinned_version(s.doc.sources.deck(), edit)
    } else {
        edit.session_version.is_none() && edited.as_ref() != Some(&edit.base)
    };
    doc.dirty || moved
}

/// The version of `edit`'s pack that `deck` pins, in either kind.
fn pinned_version(deck: &oku_slides_core::Deck, edit: &PackMode) -> Option<PackVersion> {
    deck.all_pins().find(|(_, _, p)| p.id == edit.pack).map(|(_, _, p)| p.version.clone())
}

#[cfg(test)]
#[path = "resume_workshop_tests.rs"]
mod resume_workshop_tests;
