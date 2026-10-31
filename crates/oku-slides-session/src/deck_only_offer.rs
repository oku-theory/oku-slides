//! Deck-only packs the deck no longer pins, offered again by the packs panel.
//!
//! Removing a deck-only pack (`PackScope::Deck`) keeps its versions in the
//! deck's sources, but the library never had it: the panel's add choices,
//! read from the library, lost it for good (t-937). These choices come from
//! the sources instead, and pinning one again writes its embedded version without
//! writing the library.

use std::collections::{BTreeMap, HashSet};

use oku_slides_core::{Namespace, Op, PackKind, PackPin, PackVersion};
use uuid::Uuid;

use crate::library::may_own;
use crate::pack::PackManifest;
use crate::pack_receive::is_deck_only;
use crate::{PackLibrary, Result, Session};

/// A deck-only pack the deck can pin again: its newest embedded version.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DeckOnlyChoice {
    /// Namespace the pack gives its widgets or layouts.
    pub namespace: Namespace,
    /// Name shown in the add choices.
    pub name: String,
    /// The pin `pin_deck_only` writes.
    pub pin: PackPin,
}

impl Session {
    /// The deck-only packs of kind `kind` the sources hold and the deck does not pin.
    ///
    /// One per namespace, the newest leaf version of its pack: one that no
    /// other embedded version (deck-only or not) names as its `parent`, the
    /// latest `created`, then the greatest version. Left out: ids and
    /// namespaces the deck pins under either kind, a namespace the library
    /// offers (its pack wins, so a deck cannot pass for it) and a builtin
    /// namespace claimed by another id (R15).
    pub(crate) fn deck_only_choices(&self, kind: PackKind) -> Vec<DeckOnlyChoice> {
        let deck = self.deck();
        let pinned_ids: HashSet<Uuid> = deck.all_pins().map(|(_, _, pin)| pin.id).collect();
        let pinned_ns = |ns: &Namespace| deck.widgets.contains_key(ns) || deck.layouts.contains_key(ns);
        let library_ns: HashSet<Namespace> = self
            .library
            .entries(kind)
            .into_iter()
            .filter(|s| PackLibrary::owns(s, &s.manifest.namespace) && s.manifest.scope.is_library())
            .map(|s| s.manifest.namespace)
            .collect();
        let mut parents: HashSet<PackVersion> = HashSet::new();
        let mut by_id: BTreeMap<Uuid, Vec<(PackVersion, PackManifest)>> = BTreeMap::new();
        for (version, files) in &self.user_blobs().packs {
            if files.kind() != kind {
                continue;
            }
            let m = match PackManifest::from_files(files) {
                Ok(m) => m,
                Err(e) => {
                    tracing::debug!(%version, error = %e, "unreadable embedded manifest: not offered");
                    continue;
                }
            };
            parents.extend(m.parent.clone());
            let offered = is_deck_only(files)
                && may_own(m.id, &m.namespace)
                && !pinned_ids.contains(&m.id)
                && !pinned_ns(&m.namespace)
                && !library_ns.contains(&m.namespace);
            if offered {
                by_id.entry(m.id).or_default().push((version.clone(), m));
            }
        }
        let mut by_ns: BTreeMap<Namespace, (DeckOnlyChoice, String)> = BTreeMap::new();
        for (id, versions) in by_id {
            let Some((choice, created)) = newest_leaf(versions, &parents) else {
                tracing::debug!(%id, "deck-only pack without a leaf version: not offered");
                continue;
            };
            match by_ns.get(&choice.namespace) {
                Some((_, kept)) if *kept >= created => {
                    tracing::debug!(%id, namespace = %choice.namespace, "older deck-only pack on a namespace: not offered");
                }
                _ => {
                    by_ns.insert(choice.namespace.clone(), (choice, created));
                }
            }
        }
        let mut choices: Vec<DeckOnlyChoice> = by_ns.into_values().map(|(c, _)| c).collect();
        choices.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.namespace.cmp(&b.namespace)));
        choices
    }

    /// The deck-only choice of kind `kind` offered under `ns`, if any.
    pub(crate) fn deck_only_choice(&self, kind: PackKind, ns: &Namespace) -> Option<DeckOnlyChoice> {
        self.deck_only_choices(kind).into_iter().find(|c| c.namespace == *ns)
    }

    /// Pins again the deck-only `choice`, found by [`Session::deck_only_choice`].
    ///
    /// Writes no library. One undo step; its missing requirements join it (`Session::apply`).
    ///
    /// # Errors
    ///
    /// Fails outside deck mode, or if the transaction is rejected.
    pub(crate) fn pin_deck_only(&mut self, kind: PackKind, choice: DeckOnlyChoice) -> Result<()> {
        self.ensure_deck_mode()?;
        let label = self.localizer.tr_with("packs-undo-add", &[("name", choice.name.as_str().into())]);
        self.apply(&label, vec![Op::AddPack { kind, namespace: choice.namespace, pin: choice.pin }], None)
    }
}

/// The newest leaf among the embedded versions of one pack, with its `created`.
fn newest_leaf(
    versions: Vec<(PackVersion, PackManifest)>,
    parents: &HashSet<PackVersion>,
) -> Option<(DeckOnlyChoice, String)> {
    let (version, m) = versions
        .into_iter()
        .filter(|(v, _)| !parents.contains(v))
        .max_by(|(va, a), (vb, b)| a.created.cmp(&b.created).then_with(|| va.cmp(vb)))?;
    let created = m.created;
    Some((
        DeckOnlyChoice {
            namespace: m.namespace,
            name: m.name,
            pin: PackPin { id: m.id, version, requires: m.requires },
        },
        created,
    ))
}

#[cfg(test)]
#[path = "deck_only_offer_tests.rs"]
mod deck_only_offer_tests;
