//! Compaction des nœuds morts de l'arbre du deck, à l'ouverture (t-770, ADR-0007).
//!
//! Avec `limit = min(now, change le plus récent) − délai`, un nœud qui porte
//! une marque datée de `]0, limit]` (une seule suffit : seul son pair pourrait
//! la retirer, et sa fenêtre est passée), ou un nœud vide daté de `]0, limit]`,
//! est supprimé par `tree.delete` avec son sous-arbre, déjà caché. Une marque
//! n'est donc compactée qu'après un délai d'activité du document. Commit
//! silencieux, de message [`COMPACT`], daté comme le change le plus récent
//! (`set_next_commit_timestamp`) : Loro le daterait sinon de `now`, et ouvrir
//! un fichier compterait comme une activité.

use loro::{LoroDoc, LoroTree, TreeID, TreeParentId};
use snafu::ResultExt;

use super::tree::{deck_tree_of, mark_time};
use super::undo;
use crate::error::{LoroSnafu, ReplicaInner};

/// Message du commit de compaction : hors historique et hors rapport de fusion.
pub(super) const COMPACT: &str = "oku-compact";

/// Supprime les nœuds morts depuis `after_secs` ; rend leur nombre.
///
/// Un nœud qu'un ancêtre compacté emporte n'est pas compté. `after_secs <= 0` :
/// rien. Un historique illisible ne compacte rien.
pub(super) fn compact_doc(doc: &LoroDoc, now: i64, after_secs: i64) -> Result<usize, ReplicaInner> {
    if after_secs <= 0 {
        return Ok(0);
    }
    let (Some(tree), Some(newest)) = (deck_tree_of(doc), newest_change(doc)) else { return Ok(0) };
    let limit = now.min(newest).saturating_sub(after_secs);
    let deleted = super::entry::deleted_nodes(&tree)?;
    let mut dead: Vec<TreeID> = Vec::new();
    for n in tree.nodes() {
        if !deleted.contains(&n) && is_dead(doc, &tree, n, limit)? {
            dead.push(n);
        }
    }
    dead.sort_unstable();
    let top: Vec<TreeID> = dead.iter().copied().filter(|n| !has_dead_ancestor(&tree, *n, &dead)).collect();
    for n in &top {
        tree.delete(*n).context(LoroSnafu)?;
    }
    if !top.is_empty() {
        doc.set_next_commit_message(COMPACT);
        doc.set_next_commit_origin(undo::SILENT);
        doc.set_next_commit_timestamp(newest);
        doc.commit();
    }
    Ok(top.len())
}

/// Vrai si `t` est une date ancienne : positive et au plus `limit`.
fn is_old(t: i64, limit: i64) -> bool {
    t > 0 && t <= limit
}

/// Vrai si `n` est mort avant `limit` (une marque, ou nœud vide daté).
fn is_dead(doc: &LoroDoc, tree: &LoroTree, n: TreeID, limit: i64) -> Result<bool, ReplicaInner> {
    let meta = tree.get_meta(n).context(LoroSnafu)?;
    if meta.is_empty() {
        let dated = tree.get_last_move_id(&n).and_then(|id| doc.get_change(id)).map(|c| c.timestamp);
        return Ok(dated.is_some_and(|t| is_old(t, limit)));
    }
    Ok(meta.keys().any(|k| mark_time(&meta, &k).is_some_and(|t| is_old(t, limit))))
}

/// Vrai si un ancêtre de `n` est dans `dead` (trié).
fn has_dead_ancestor(tree: &LoroTree, n: TreeID, dead: &[TreeID]) -> bool {
    let mut at = n;
    while let Some(TreeParentId::Node(p)) = tree.parent(at) {
        if dead.binary_search(&p).is_ok() {
            return true;
        }
        at = p;
    }
    false
}

/// L'horodatage positif le plus récent des changes de `doc`.
///
/// `None` si aucun n'est daté, ou si l'historique ne se parcourt pas.
fn newest_change(doc: &LoroDoc) -> Option<i64> {
    let heads: Vec<loro::ID> = doc.oplog_frontiers().iter().collect();
    let mut newest: Option<i64> = None;
    doc.travel_change_ancestors(&heads, &mut |m| {
        if m.timestamp > 0 {
            newest = Some(newest.map_or(m.timestamp, |n| n.max(m.timestamp)));
        }
        std::ops::ControlFlow::Continue(())
    })
    .ok()?;
    newest
}

#[cfg(test)]
#[path = "compact_tests.rs"]
mod compact_tests;
