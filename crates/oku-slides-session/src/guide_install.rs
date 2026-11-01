//! Guides to approve at install: the received versions a deck brings.
//!
//! A deck that opens or merges brings pack and theme versions, installed
//! silently. Those whose guides the user has not approved are listed in the
//! snapshot, with what each holds, so one dialog can ask for all of them.

use serde::Serialize;
use uuid::Uuid;

use oku_slides_core::{Guides, PackFiles, PackKind, PackVersion};

use crate::agent::bounds::{clip, label};
use crate::guide_approvals::VersionRef;
use crate::{Mode, Session};

/// What a held version is: a widgets pack, a layouts pack or a theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeldKind {
    /// A pack of widgets.
    Widgets,
    /// A pack of layouts.
    Layouts,
    /// A theme.
    Theme,
}

impl HeldKind {
    /// The held kind of a pack of kind `kind`.
    pub(crate) fn of(kind: PackKind) -> Self {
        if kind == PackKind::Widgets { Self::Widgets } else { Self::Layouts }
    }

    /// The kind's name in a review: `widgets`, `layouts` or `theme`.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Widgets => "widgets",
            Self::Layouts => "layouts",
            Self::Theme => "theme",
        }
    }
}

/// A version the open document pins, with its embedded files.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HeldVersion<'a> {
    /// Id of the pack or theme.
    pub(crate) id: Uuid,
    /// The pinned version.
    pub(crate) version: &'a PackVersion,
    /// The namespace the deck pins it under.
    pub(crate) namespace: &'a oku_slides_core::Namespace,
    /// Widgets, layouts or theme.
    pub(crate) kind: HeldKind,
    /// The namespaces the pin requires (a theme's parents).
    pub(crate) requires: &'a [oku_slides_core::Namespace],
    /// The version's files, embedded in the document's sources.
    pub(crate) files: &'a PackFiles,
}

/// A received version awaiting the user's approval, with what it holds.
#[derive(Debug, Clone, Serialize)]
pub struct GuidesReview {
    /// Id of the pack or theme.
    pub id: Uuid,
    /// The version.
    pub version: PackVersion,
    /// `widgets`, `layouts` or `theme`.
    pub kind: &'static str,
    /// Its name, cleaned of control characters.
    pub name: String,
    /// How many widgets or layouts it holds (`_commun` excluded).
    pub items: usize,
    /// The namespaces it requires (a theme's parents).
    pub requires: Vec<String>,
    /// Both guides as the agent would read them (trimmed, clipped), to be read before approving.
    pub guides: Guides,
}

/// The guides written in `files`, read from the manifest's `guides` key alone.
///
/// Unreadable guides are logged and count as none.
pub(crate) fn guides_of(files: &PackFiles) -> Guides {
    let Some(value) = files.manifest().get("guides") else { return Guides::default() };
    serde_json::from_value(value.clone()).unwrap_or_else(|e| {
        tracing::warn!(version = %files.version(), error = %e, "guides unreadable: none");
        Guides::default()
    })
}

/// The name written in `files`, cleaned for a dialog.
pub(crate) fn name_of(files: &PackFiles) -> String {
    label(files.manifest().get("name").and_then(|n| n.as_str()).unwrap_or_default())
}

impl Session {
    /// Every version the deck pins, packs then themes, in pin order, the deck's own included.
    ///
    /// A pin whose files the sources do not hold is left out: the sources report it.
    pub(crate) fn held_versions(&self) -> Vec<HeldVersion<'_>> {
        Self::held_in(&self.doc.sources)
    }

    /// The same walk over any sources, such as a pack tab's preview deck.
    pub(crate) fn held_in(sources: &oku_slides_core::SourceBundle) -> Vec<HeldVersion<'_>> {
        let deck = sources.deck();
        let packs = deck.all_pins().filter_map(|(kind, namespace, pin)| {
            let kind = HeldKind::of(kind);
            let files = sources.packs.get(&pin.version)?;
            Some(HeldVersion { id: pin.id, version: &pin.version, namespace, kind, requires: &pin.requires, files })
        });
        let themes = deck.themes.iter().filter_map(|(namespace, pin)| {
            let files = sources.themes.get(&pin.version)?;
            Some(HeldVersion {
                id: pin.id,
                version: &pin.version,
                namespace,
                kind: HeldKind::Theme,
                requires: &pin.extends,
                files,
            })
        });
        packs.chain(themes).collect()
    }

    /// `guides` as the agent would read them: trimmed, hidden controls dropped, cut at the limit.
    ///
    /// The install dialog shows this, so the user approves the text the agent gets.
    fn as_the_agent_reads(&self, guides: &Guides) -> Guides {
        let shown = |text: &str| clip(text.trim(), self.guide_limit);
        Guides { edit: shown(&guides.edit), use_: shown(&guides.use_) }
    }

    /// The received versions of the open deck whose guides await approval, in pin order.
    ///
    /// Non-empty guides, origin received, not approved, not put off in this tab.
    /// Only a deck tab asks: a pack or theme tab, history and the help do not.
    pub(crate) fn guides_to_approve(&self) -> Vec<GuidesReview> {
        if !matches!(self.mode, Mode::Deck) {
            return Vec::new();
        }
        let mut seen = Vec::new();
        let mut reviews = Vec::new();
        for held in self.held_versions() {
            let this = VersionRef { id: held.id, version: held.version.clone() };
            let guides = guides_of(held.files);
            let waiting = !guides.is_empty()
                && !seen.contains(&this)
                && !self.guides_later.contains(&this)
                && !self.guide_active(held.id, held.version, held.kind == HeldKind::Theme);
            seen.push(this);
            if waiting {
                reviews.push(GuidesReview {
                    id: held.id,
                    version: held.version.clone(),
                    kind: held.kind.as_str(),
                    name: name_of(held.files),
                    items: held.files.widgets().keys().filter(|f| !f.is_common()).count(),
                    requires: held.requires.iter().map(|n| n.as_str().to_owned()).collect(),
                    guides: self.as_the_agent_reads(&guides),
                });
            }
        }
        reviews
    }
}

#[cfg(test)]
#[path = "guide_install_tests.rs"]
mod guide_install_tests;
