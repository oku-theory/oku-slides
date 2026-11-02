//! What a received deck brings, reviewed before it enters the library.
//!
//! A deck that opens or merges holds pack and theme versions; none enters the
//! user's library by itself. Those the library lacks (library scope, not
//! built-in), and those whose received guides await approval, are listed in
//! the snapshot with their capability manifest, so one dialog validates them
//! all (`Request::AcceptReceived`, see `pack_receive`). A version that cannot
//! enter the library says why (`blocker`) and stays in the deck; so does a
//! theme whose parent cannot. The list is cached on what it depends on
//! ([`ReviewKey`]); each capability list shows at most [`SHOWN_NAMES`] names.

use std::sync::{Arc, Mutex, PoisonError};

use serde::Serialize;
use uuid::Uuid;

use oku_slides_core::{Capabilities, CapabilityKind, Guides, Namespace, PackFiles, PackVersion};
use oku_slides_i18n::Localize;

use crate::agent::bounds::{clip, label};
use crate::guide_approvals::VersionRef;
use crate::library_index::library_writes;
use crate::received_held::{HeldVersion, is_builtin};
use crate::{Mode, Session, SessionError, SessionErrorKind};

/// Names a review shows per capability list; [`CapabilityTotals`] counts them all.
pub(crate) const SHOWN_NAMES: usize = 50;

/// How a received version relates to the user's library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceivedRelation {
    /// The library has no version of this pack or theme.
    NewPack,
    /// The library has the pack or theme, not this version.
    NewVersion,
    /// The library already holds this version.
    Held,
}

/// How many names each capability list holds before the review cuts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct CapabilityTotals {
    /// Widget or layout names.
    pub items: usize,
    /// Required namespaces, or the themes a theme extends.
    pub requires: usize,
    /// Token paths or token groups.
    pub tokens: usize,
    /// Font families.
    pub fonts: usize,
}

/// A version the open deck holds, to validate before it enters the library.
#[derive(Debug, Clone, Serialize)]
pub struct ReceivedReview {
    /// Id of the pack or theme.
    pub id: Uuid,
    /// The version.
    pub version: PackVersion,
    /// What it holds, its texts cleaned, each list cut at [`SHOWN_NAMES`] names.
    pub capabilities: Capabilities,
    /// The full length of each list of `capabilities`.
    pub totals: CapabilityTotals,
    /// New pack, new version of a pack the library has, or a version it holds.
    pub relation: ReceivedRelation,
    /// True when `AcceptReceived` adds it to the library.
    pub install: bool,
    /// Why it stays in the deck, localized; then `install` is false.
    pub blocker: Option<String>,
    /// Both guides as the agent would read them; empty when none await approval.
    pub guides: Guides,
}

impl ReceivedReview {
    /// The version this review is about.
    #[must_use]
    pub fn version_ref(&self) -> VersionRef {
        VersionRef { id: self.id, version: self.version.clone() }
    }
}

/// One pin as the review key holds it: kind, namespace, id, version, requirements.
type KeyPin = (CapabilityKind, Namespace, Uuid, PackVersion, Vec<Namespace>);

/// What the reviews depend on: the mode, the pins, library writes and approvals.
///
/// The localizer and the guide limit are fixed per session; `received_later`
/// filters the cached list when read. A write by another process is seen at
/// the next write of this one or the next pin change, as `pack_updates` is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewKey {
    deck: bool,
    pins: Vec<KeyPin>,
    writes: u64,
    approvals: usize,
}

/// The reviews last computed, with the key they were computed for.
#[derive(Debug, Default)]
pub(crate) struct ReviewCache(Mutex<Option<(ReviewKey, Arc<Vec<ReceivedReview>>)>>);

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

/// The name written in `files`, cleaned for a dialog; its namespace when the name is empty.
pub(crate) fn name_of(files: &PackFiles) -> String {
    let raw = |key: &str| files.manifest().get(key).and_then(|n| n.as_str()).unwrap_or_default();
    let name = raw("name");
    label(if name.trim().is_empty() { raw("namespace") } else { name })
}

impl Session {
    /// `guides` as the agent would read them: trimmed, hidden controls dropped, cut at the limit.
    ///
    /// The install dialog shows this, so the user approves the text the agent gets.
    fn as_the_agent_reads(&self, guides: &Guides) -> Guides {
        let shown = |text: &str| clip(text.trim(), self.guide_limit);
        Guides { edit: shown(&guides.edit), use_: shown(&guides.use_) }
    }

    /// The versions of the open deck to validate, in pin order, packs then themes.
    ///
    /// Each one the library lacks (library scope, not built-in), or whose received
    /// guides are not approved, except those put off in this tab (`received_later`).
    /// Only a deck tab asks: a pack or theme tab, history and the help do not.
    pub(crate) fn received_to_review(&self) -> Vec<ReceivedReview> {
        let all = self.received_reviews();
        all.iter().filter(|r| !self.received_later.contains(&r.version_ref())).cloned().collect()
    }

    /// Every review, the versions put off included, from the cache when its key still holds.
    pub(crate) fn received_reviews(&self) -> Arc<Vec<ReceivedReview>> {
        let key = self.review_key();
        let mut cache = self.received_cache.0.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((seen, reviews)) = cache.as_ref()
            && *seen == key
        {
            return Arc::clone(reviews);
        }
        let reviews = Arc::new(self.fresh_reviews());
        *cache = Some((key, Arc::clone(&reviews)));
        reviews
    }

    /// What the reviews depend on now (see [`ReviewKey`]).
    fn review_key(&self) -> ReviewKey {
        let deck = matches!(self.mode, Mode::Deck);
        let pins = if deck {
            let pin = |h: &HeldVersion<'_>| (h.kind, h.namespace.clone(), h.id, h.version.clone(), h.requires.to_vec());
            self.held_versions().iter().map(pin).collect()
        } else {
            Vec::new()
        };
        ReviewKey { deck, pins, writes: library_writes(), approvals: self.library.guide_approvals().count() }
    }

    /// Every review, computed without the cache: install paths read the library afresh.
    pub(crate) fn fresh_reviews(&self) -> Vec<ReceivedReview> {
        if !matches!(self.mode, Mode::Deck) {
            return Vec::new();
        }
        let held = self.held_versions();
        let mut seen = Vec::new();
        let mut reviews = Vec::new();
        for one in &held {
            let this = VersionRef { id: one.id, version: one.version.clone() };
            if seen.contains(&this) {
                continue;
            }
            seen.push(this);
            reviews.extend(self.review_of(one, &held));
        }
        reviews
    }

    /// The review of `held` among `all`; `None` when it needs none (in the library, guides trusted).
    fn review_of(&self, held: &HeldVersion<'_>, all: &[HeldVersion<'_>]) -> Option<ReceivedReview> {
        let guides = guides_of(held.files);
        let waiting = !guides.is_empty() && !self.guide_active(held.id, held.version, held.is_theme());
        // Decided without reading the library first: a snapshot of a deck of built-ins reads none (t-1112).
        let builtin = is_builtin(held);
        if !waiting && (builtin || self.library_has(held)) {
            return None;
        }
        let relation = self.relation_of(held);
        // In the library or shipped: nothing to install. Deck-only: says so (`DeckOnly`).
        let settled = relation == ReceivedRelation::Held || builtin;
        let read = read_capabilities(held);
        let blocker = match &read {
            None => Some(self.localizer.tr("received-blocker-unreadable")),
            Some(_) if settled => None,
            Some(caps) => self.install_blocker(held, all, &caps.namespace),
        };
        let (capabilities, totals) = read.map_or_else(|| unreadable_capabilities(held), |caps| cleaned(&caps));
        Some(ReceivedReview {
            id: held.id,
            version: held.version.clone(),
            capabilities,
            totals,
            relation,
            install: !settled && blocker.is_none(),
            blocker,
            guides: if waiting { self.as_the_agent_reads(&guides) } else { Guides::default() },
        })
    }

    /// Why `held`, under `namespace`, cannot enter its library: its own refusal, then a theme's parent's.
    fn install_blocker(&self, held: &HeldVersion<'_>, all: &[HeldVersion<'_>], namespace: &str) -> Option<String> {
        if let Err(e) = self.check_install(held) {
            return Some(self.blocker_text(&e, namespace));
        }
        let parent = self.blocked_parent(held, all, &mut Vec::new())?;
        Some(self.localizer.tr_with("received-blocker-parent", &[("namespace", label(parent.as_str()).into())]))
    }

    /// How `held` relates to the library of its kind.
    fn relation_of(&self, held: &HeldVersion<'_>) -> ReceivedRelation {
        match (self.knows(held), self.lists(held)) {
            (_, true) => ReceivedRelation::Held,
            (true, false) => ReceivedRelation::NewVersion,
            (false, false) => ReceivedRelation::NewPack,
        }
    }

    /// Why a version under `namespace` stays in the deck, from the library's refusal `e`.
    fn blocker_text(&self, e: &SessionError, namespace: &str) -> String {
        let l = &self.localizer;
        let ns = [("namespace", namespace.into())];
        match e.kind() {
            SessionErrorKind::NamespaceTaken => l.tr_with("received-blocker-namespace-taken", &ns),
            SessionErrorKind::NamespaceReserved => l.tr_with("received-blocker-namespace-reserved", &ns),
            SessionErrorKind::NamespaceChanged => l.tr_with("received-blocker-namespace-changed", &ns),
            SessionErrorKind::DeckOnly => l.tr("received-blocker-deck-only"),
            _ => e.localize(l),
        }
    }
}

/// The capabilities of `held`, when they read as the kind it is held as; `None` otherwise, logged.
///
/// A version whose content claims another kind than where the deck holds it
/// (a theme pinned as a pack, or the reverse) is unreadable: never trusted.
pub(crate) fn read_capabilities(held: &HeldVersion<'_>) -> Option<Capabilities> {
    match Capabilities::of(held.files) {
        Ok(caps) if caps.kind == held.kind => Some(caps),
        Ok(caps) => {
            tracing::warn!(version = %held.version, found = ?caps.kind, "received version of another kind: unreadable");
            None
        }
        Err(e) => {
            tracing::warn!(version = %held.version, error = %e, "received version unreadable");
            None
        }
    }
}

/// The first [`SHOWN_NAMES`] of `texts`, each cleaned for a dialog (`bounds::label`).
fn shown(texts: &[String]) -> Vec<String> {
    texts.iter().take(SHOWN_NAMES).map(|t| label(t)).collect()
}

/// `caps` with every text cleaned for a dialog and each list cut, with the lists' full lengths.
pub(crate) fn cleaned(caps: &Capabilities) -> (Capabilities, CapabilityTotals) {
    let totals = CapabilityTotals {
        items: caps.items.len(),
        requires: caps.requires.len(),
        tokens: caps.tokens.len(),
        fonts: caps.fonts.len(),
    };
    let shown_caps = Capabilities {
        kind: caps.kind,
        name: label(&caps.name),
        namespace: label(&caps.namespace),
        description: if caps.description.trim().is_empty() { String::new() } else { label(&caps.description) },
        items: shown(&caps.items),
        requires: shown(&caps.requires),
        tokens: shown(&caps.tokens),
        fonts: shown(&caps.fonts),
        profiles: caps.profiles,
        code_bytes: caps.code_bytes,
    };
    (shown_caps, totals)
}

/// What a dialog shows of an unreadable version: where the deck holds it, nothing it claims.
fn unreadable_capabilities(held: &HeldVersion<'_>) -> (Capabilities, CapabilityTotals) {
    let requires: Vec<String> = held.requires.iter().map(|n| n.as_str().to_owned()).collect();
    let caps = Capabilities {
        kind: held.kind,
        name: label(held.namespace.as_str()),
        namespace: label(held.namespace.as_str()),
        description: String::new(),
        items: Vec::new(),
        requires: shown(&requires),
        tokens: Vec::new(),
        fonts: Vec::new(),
        profiles: 0,
        code_bytes: 0,
    };
    (caps, CapabilityTotals { requires: requires.len(), ..CapabilityTotals::default() })
}

#[cfg(test)]
#[path = "received_review_tests.rs"]
pub(crate) mod received_review_tests;

#[cfg(test)]
#[path = "received_cache_tests.rs"]
mod received_cache_tests;
