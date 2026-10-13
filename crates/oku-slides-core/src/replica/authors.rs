//! Auteurs du deck : le nom de chaque pair, dans la map racine `peers`.
//!
//! Un pair (une session d'édition) s'inscrit à sa première édition, dans le
//! même commit : un pair qui n'a rien gardé n'a pas de nom.

use std::collections::BTreeSet;
use std::ops::ControlFlow;

use loro::{LoroDoc, LoroValue, ValueOrContainer, VersionVector};
use snafu::ResultExt;

use crate::error::{LoroSnafu, ReplicaInner};

pub(super) const PEERS: &str = "peers";

/// Ce qu'une fusion a importé : ses modifications, et qui les a faites.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MergeReport {
    /// Modifications importées (une par commit, voir `configure`).
    pub changes: usize,
    /// Auteurs nommés, triés, sans doublon.
    pub authors: Vec<String>,
    /// Pairs sans nom parmi ces modifications.
    pub anonymous: usize,
}

/// Les modifications de `doc` absentes de `before`.
///
/// Les parties d'une édition (t-263, t-776) ne comptent pas : une édition compte une fois.
pub(super) fn report_since(doc: &LoroDoc, before: &VersionVector) -> MergeReport {
    let heads: Vec<loro::ID> = doc.oplog_frontiers().iter().collect();
    let (mut changes, mut peers) = (0, BTreeSet::new());
    // L'erreur de parcours est ignorée faute de mieux : un historique cohérent
    // (le seul que produit un import réussi) ne la produit pas.
    let _ = doc.travel_change_ancestors(&heads, &mut |meta| {
        let skipped =
            meta.message.as_deref().is_some_and(|m| m.starts_with(super::PART) || m == super::compact::COMPACT);
        if !before.includes_id(meta.id) && !skipped {
            changes += 1;
            peers.insert(meta.id.peer);
        }
        ControlFlow::Continue(())
    });
    let (mut authors, mut anonymous) = (BTreeSet::new(), 0);
    for peer in peers {
        match author_of(doc, peer) {
            Some(name) => {
                authors.insert(name);
            }
            None => anonymous += 1,
        }
    }
    MergeReport { changes, authors: authors.into_iter().collect(), anonymous }
}

pub(super) fn author_of(doc: &LoroDoc, peer: u64) -> Option<String> {
    match doc.get_map(PEERS).get(&peer.to_string()) {
        Some(ValueOrContainer::Value(LoroValue::String(s))) => Some(s.to_string()),
        _ => None,
    }
}

/// Inscrit `name` pour ce pair s'il ne l'est pas encore (sans committer).
pub(super) fn claim(doc: &LoroDoc, name: &str) -> Result<(), ReplicaInner> {
    let key = doc.peer_id().to_string();
    let peers = doc.get_map(PEERS);
    if peers.get(&key).is_none() {
        peers.insert(&key, name).context(LoroSnafu)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "authors_tests.rs"]
mod authors_tests;
