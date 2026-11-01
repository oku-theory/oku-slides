//! Where a pack or theme version came from, and whether its guides may reach the agent.
//!
//! A built-in or local version is trusted; a received one (from a deck, or
//! held only by a deck) needs the user's approval, version by version.

use serde::Serialize;
use uuid::Uuid;

use oku_slides_core::{GuideKind, Guides, PackFiles, PackVersion};

use oku_slides_render::BuiltinPack;

use crate::Session;
use crate::guide_approvals::VersionRef;
use crate::guide_install::guides_of;
use crate::library::builtin_owner;
use crate::library_kind::LibraryKind;
use crate::theme::{Themes, shipped_theme_versions};
use crate::views::GuidesView;

/// Where a version of a pack or theme came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuideOrigin {
    /// Shipped with the app.
    Builtin,
    /// Written by the user, or kept by the user's library.
    Local,
    /// Received with a deck, or held only by a deck.
    Received,
}

/// The guides of the version a write replaces, and whether they were active.
#[derive(Debug, Clone)]
pub(crate) struct Predecessor {
    /// Its guides.
    guides: Guides,
    /// Built-in, local or approved.
    active: bool,
}

/// True if a non-empty guide of `guides` is kept unchanged from an inactive predecessor.
fn inherits_unapproved(guides: &Guides, before: Option<&Predecessor>) -> bool {
    let Some(p) = before.filter(|p| !p.active) else { return false };
    [GuideKind::Edit, GuideKind::Use].iter().any(|k| !guides.get(*k).is_empty() && guides.get(*k) == p.guides.get(*k))
}

/// True if `version` of `id` is a version the app ships now or shipped in a release.
///
/// Reads the shipped history (`versions.txt` of each built-in pack, `themes/versions.txt`),
/// so a deck pinning a built-in from an earlier release stays built-in after an upgrade.
/// The id must be the built-in's own: another pack claiming a shipped digest is not built-in.
fn is_shipped(id: Uuid, version: &PackVersion, theme: bool) -> bool {
    let hex = version.hex();
    if theme {
        Themes::builtins().iter().filter(|b| b.manifest.id == id).any(|b| {
            let ns = b.manifest.namespace.as_str();
            shipped_theme_versions().any(|(n, h)| n == ns && h == hex)
        })
    } else {
        BuiltinPack::ALL
            .into_iter()
            .any(|p| builtin_owner(&p.namespace()) == Some(id) && p.shipped_versions().any(|h| h == hex))
    }
}

impl GuideOrigin {
    /// The origin's name, as serde writes it.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::Local => "local",
            Self::Received => "received",
        }
    }
}

impl Session {
    /// The origin of version `version` of pack or theme `id` (`theme` selects the theme library).
    ///
    /// Built-in if the app ships or shipped it; local if the user's library holds it and did
    /// not receive it; received otherwise, a version only a deck holds included.
    ///
    /// A shipped version is decided without reading the library.
    pub(crate) fn guide_origin(&self, id: Uuid, version: &PackVersion, theme: bool) -> GuideOrigin {
        if is_shipped(id, version, theme) {
            return GuideOrigin::Builtin;
        }
        let (held, received) = if theme {
            (self.themes.version(id, version).is_some(), self.themes.is_received_or_unreadable(id, version))
        } else {
            (self.library.version(id, version).is_some(), self.library.is_received_or_unreadable(id, version))
        };
        if held && !received { GuideOrigin::Local } else { GuideOrigin::Received }
    }

    /// What a write replaces: the version's guides and whether they may already reach the agent.
    ///
    /// Taken before the write, while the version's files are still held; `None` when no files
    /// are found (a brand-new pack has none).
    pub(crate) fn predecessor(&self, id: Uuid, version: &PackVersion, theme: bool) -> Option<Predecessor> {
        let sources = &self.doc.sources;
        let held = if theme { sources.themes.get(version) } else { sources.packs.get(version) };
        let files = match held {
            Some(f) => f.clone(),
            None if theme => self.themes.version(id, version)?,
            None => self.library.version(id, version)?,
        };
        Some(Predecessor { guides: guides_of(&files), active: self.guide_active(id, version, theme) })
    }

    /// Marks the library write of `files` received if it only carries an unapproved guide forward.
    ///
    /// Called just before a library write of the user's own: a version that keeps a guide
    /// unchanged from an inactive predecessor (a hostile received guide carried through an edit)
    /// is then written with the received mark, so its origin is received and it stays pending.
    /// Receive and merge paths never call this.
    pub(crate) fn taint_inherited(&self, files: &PackFiles, before: Option<&Predecessor>, theme: bool) {
        if inherits_unapproved(&guides_of(files), before) {
            if theme {
                self.themes.taint(&files.version());
            } else {
                self.library.taint_pack(&files.version(), files.kind());
            }
        }
    }

    /// Approves the version this session just wrote, if the user wrote its guides.
    ///
    /// A guide text the write inherits unchanged from a predecessor that is not itself active
    /// (a hostile received guide carried through a widget edit) is not approved: the version stays
    /// pending like the original. No predecessor and guides: not approved, since nothing shows
    /// the user wrote them (a new pack starts with none). Receive and merge paths never call
    /// this. A failed write is logged: the version then asks for approval like any other.
    pub(crate) fn approve_written(&self, id: Uuid, files: &PackFiles, before: Option<&Predecessor>) {
        let guides = guides_of(files);
        if guides.is_empty() || before.is_none() || inherits_unapproved(&guides, before) {
            return;
        }
        let version = files.version();
        let approvals = self.library.guide_approvals();
        if approvals.is_approved(id, &version) {
            return;
        }
        if let Err(e) = approvals.approve(&[VersionRef { id, version }]) {
            tracing::warn!(pack = %id, error = %e.one_line(), "guides of the written version not approved");
        }
    }

    /// True if the guides of the version may reach the agent: trusted, or approved by the user.
    pub(crate) fn guide_active(&self, id: Uuid, version: &PackVersion, theme: bool) -> bool {
        self.guide_origin(id, version, theme) != GuideOrigin::Received
            || self.library.guide_approvals().is_approved(id, version)
    }

    /// The guides view of the edited version `version` of `id`, with its origin and approval.
    ///
    /// Approvable when received and not approved: `guides` must then be that version's.
    pub(crate) fn guides_view_of(&self, id: Uuid, version: &PackVersion, theme: bool, guides: Guides) -> GuidesView {
        let origin = self.guide_origin(id, version, theme);
        let approved = self.library.guide_approvals().is_approved(id, version);
        GuidesView {
            guides,
            max_chars: self.guide_limit,
            id,
            version: version.clone(),
            origin,
            approved,
            approvable: origin == GuideOrigin::Received && !approved,
        }
    }
}

#[cfg(test)]
#[path = "guide_origin_tests.rs"]
mod guide_origin_tests;

#[cfg(test)]
#[path = "guides_builtin_tests.rs"]
mod guides_builtin_tests;
