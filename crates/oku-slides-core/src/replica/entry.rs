//! L'entrée des octets non fiables : rejeu, arbre unique, profondeur bornée (t-809).
//!
//! Loro 1.16.2 récurse sur la profondeur d'un arbre : au premier accès de
//! l'état d'un snapshot, puis, même supprimé, dans `is_node_deleted`, l'export
//! et `fork`. Un débordement de pile n'est pas une panique. L'état d'un snapshot
//! n'est pas vérifié par Loro contre son historique : il peut être forgé.
//!
//! L'état reçu n'est donc jamais lu : ses opérations sont rejouées dans un
//! document neuf, qui se mesure sans récursion (sonde `tests/loro_depth_probe.rs`).
//! Un snapshot *shallow* est refusé : ses opérations rejouées perdraient l'état
//! d'avant la coupe (Loro les exporte sans erreur, tronquées).
//! [`check_entry`] refuse ensuite tout arbre autre que `deck.slides` (tout arbre
//! s'il manque), puis tout nœud, supprimé ou non, au-delà de [`MAX_TREE_DEPTH`],
//! et rend les nœuds supprimés : plus de `is_node_deleted` en boucle, en
//! O(nœuds × profondeur) ; la lecture passe de même par [`Ancestry`] (t-865).
//! Un document accepté ne déborde plus : export, `fork` et drop d'un arbre de
//! 1024 niveaux tiennent dans ~1 Mio de pile, sous les 2 Mio des fils de l'app.
//!
//! Résidus : les valeurs plates imbriquées (t-223) ; ne jamais appeler
//! `get_deep_value` sur un document reçu ; au-delà de 65 535 conteneurs
//! imbriqués, Loro panique (rattrapé par [`guarded`]) ; le rejeu d'un très grand
//! historique est lent (un million de nœuds : ~10 s) ; comme une fusion, il
//! réunit en un change les changes consécutifs d'un pair, de même message,
//! dans la même seconde (Loro 1.16.2, intervalle fixe) : deux éditions de même
//! libellé dans la même seconde ne font plus qu'une version.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::mem::ManuallyDrop;
use std::panic::{AssertUnwindSafe, catch_unwind};

use loro::{ContainerTrait, ContainerType, ExportMode, LoroDoc, LoroTree, TreeID, TreeParentId};
use snafu::{OptionExt, ResultExt, ensure};

use crate::error::{
    ExportSnafu, ImportPanickedSnafu, ImportSnafu, MalformedDeckTreeSnafu, NotADeckSnafu, ReplicaInner,
    TreeTooDeepSnafu,
};

/// Profondeur maximale de l'arbre du deck ; une racine vaut 1.
///
/// Une suppression garde son sous-arbre caché, qu'un `mov` emporte : la
/// profondeur réelle dépasse celle des seuls nœuds vivants (`MAX_SLOT_DEPTH`).
pub(in crate::replica) const MAX_TREE_DEPTH: usize = 1024;

pub(super) const TOO_DEEP: &str = "deck tree too deep";
pub(super) const FOREIGN_TREE: &str = "a tree outside deck.slides";
pub(super) const SHALLOW: &str = "shallow snapshot";

/// Exécute `f` sur `doc` sous `catch_unwind` ; une panique devient `ImportPanicked`.
///
/// Après une panique, les verrous de `doc` sont empoisonnés et son drop
/// paniquerait à nouveau : `doc` est abandonné sans drop (fuite assumée).
pub(super) fn guarded<T>(
    doc: LoroDoc,
    f: impl FnOnce(&LoroDoc) -> Result<T, ReplicaInner>,
) -> Result<(LoroDoc, T), ReplicaInner> {
    if let Ok(r) = catch_unwind(AssertUnwindSafe(|| f(&doc))) {
        r.map(|t| (doc, t))
    } else {
        std::mem::forget(doc);
        ImportPanickedSnafu.fail()
    }
}

/// Les opérations d'un snapshot reçu, sans jamais lire son état.
///
/// Refuse un snapshot *shallow*, avant l'export : ses opérations rejouées
/// perdraient l'état d'avant la coupe. À appeler
/// sous [`guarded`] : une panique laisse fuir le document reçu, jamais libéré
/// par le déroulement.
pub(super) fn replayable_updates(bytes: &[u8]) -> Result<Vec<u8>, ReplicaInner> {
    let received = ManuallyDrop::new(LoroDoc::new());
    let out = (|| {
        received.import(bytes).context(ImportSnafu)?;
        ensure!(!received.is_shallow(), NotADeckSnafu { what: SHALLOW });
        received.export(ExportMode::all_updates()).context(ExportSnafu)
    })();
    // L'état reçu n'est jamais lu ; s'il vient d'un blob d'updates, il est bâti par rejeu, dont le drop ne récurse pas (sonde).
    drop(ManuallyDrop::into_inner(received));
    out
}

/// Contrôle d'entrée d'un document rejoué ; rend ses nœuds supprimés par Loro.
///
/// Refuse tout arbre hors de `tree` (tout arbre si `tree` vaut `None`), puis
/// tout nœud au-delà de [`MAX_TREE_DEPTH`]. À appeler en premier, jamais sur
/// un état reçu.
pub(in crate::replica) fn check_entry(doc: &LoroDoc, tree: Option<&LoroTree>) -> Result<HashSet<TreeID>, ReplicaInner> {
    check_single_tree(doc, tree)?;
    tree.map_or_else(|| Ok(HashSet::new()), deleted_nodes)
}

/// Refuse tout arbre touché par l'historique autre que `tree`.
fn check_single_tree(doc: &LoroDoc, tree: Option<&LoroTree>) -> Result<(), ReplicaInner> {
    let deck = tree.map(ContainerTrait::id);
    let heads: Vec<loro::ID> = doc.oplog_frontiers().iter().collect();
    let mut changes: Vec<(loro::ID, usize)> = Vec::new();
    doc.travel_change_ancestors(&heads, &mut |m| {
        changes.push((m.id, m.len));
        std::ops::ControlFlow::Continue(())
    })
    .map_err(|_| NotADeckSnafu { what: "unreadable history" }.build())?;
    for (id, len) in changes {
        for c in doc.get_changed_containers_in(id, len) {
            ensure!(
                c.container_type() != ContainerType::Tree || Some(&c) == deck.as_ref(),
                MalformedDeckTreeSnafu { what: FOREIGN_TREE }
            );
        }
    }
    Ok(())
}

/// Profondeur et suppression des nœuds d'un arbre, mémorisées au fil des requêtes.
///
/// Remonte `parent()` (une lecture de table) jusqu'à un ancêtre déjà mesuré,
/// la racine ou celle des supprimés : chaque nœud n'est remonté qu'une fois,
/// d'où un coût total linéaire en nœuds. Jamais `is_node_deleted`, qui
/// récurse et coûte la profondeur à chaque appel (t-865).
pub(in crate::replica) struct Ancestry<'t> {
    tree: &'t LoroTree,
    /// Par nœud : profondeur (une racine vaut 1), et s'il est sous la racine des supprimés.
    seen: RefCell<HashMap<TreeID, (usize, bool)>>,
}

impl<'t> Ancestry<'t> {
    /// Rien n'est mesuré d'avance : chaque requête remonte au plus une fois chaque nœud.
    pub(in crate::replica) fn new(tree: &'t LoroTree) -> Self {
        Self { tree, seen: RefCell::new(HashMap::new()) }
    }

    /// Profondeur de `node` et s'il est supprimé par Loro ; `None` au-delà de `limit` niveaux.
    ///
    /// Un nœud absent de l'arbre vaut une racine vivante : appeler d'abord
    /// `tree.contains` s'il peut l'être.
    pub(in crate::replica) fn resolve(&self, node: TreeID, limit: usize) -> Option<(usize, bool)> {
        let mut seen = self.seen.borrow_mut();
        let mut chain: Vec<TreeID> = Vec::new();
        let mut at = node;
        let (base, deleted) = loop {
            if let Some(known) = seen.get(&at) {
                break *known;
            }
            chain.push(at);
            if chain.len() > limit {
                return None;
            }
            #[cfg(test)]
            climb();
            match self.tree.parent(at) {
                Some(TreeParentId::Node(p)) => at = p,
                Some(TreeParentId::Deleted) => break (0, true),
                _ => break (0, false),
            }
        };
        if base + chain.len() > limit {
            return None;
        }
        for (i, n) in chain.iter().rev().enumerate() {
            seen.insert(*n, (base + i + 1, deleted));
        }
        seen.get(&node).copied()
    }

    /// Vrai si `node` manque à l'arbre ou est supprimé par Loro.
    pub(in crate::replica) fn gone(&self, node: TreeID) -> bool {
        !self.tree.contains(node) || self.resolve(node, usize::MAX).is_none_or(|(_, deleted)| deleted)
    }
}

/// Les nœuds de `tree` supprimés par Loro ; refuse un nœud au-delà de [`MAX_TREE_DEPTH`].
///
/// Linéaire en nœuds (voir [`Ancestry`]).
pub(in crate::replica) fn deleted_nodes(tree: &LoroTree) -> Result<HashSet<TreeID>, ReplicaInner> {
    let ancestry = Ancestry::new(tree);
    let mut deleted = HashSet::new();
    for node in tree.nodes() {
        let (_, gone) = ancestry.resolve(node, MAX_TREE_DEPTH).context(MalformedDeckTreeSnafu { what: TOO_DEEP })?;
        if gone {
            deleted.insert(node);
        }
    }
    Ok(deleted)
}

/// Refuse d'enregistrer un arbre dont un nœud, retiré compris, dépasse [`MAX_TREE_DEPTH`].
///
/// Un sous-arbre marqué garde sa profondeur et suit son ancêtre vivant : des
/// retraits suivis de déplacements vers le bas l'accumulent sans borne, alors
/// que [`check_entry`] refuserait le fichier à sa réouverture (t-866).
pub(in crate::replica) fn check_depth(tree: &LoroTree) -> Result<(), ReplicaInner> {
    let ancestry = Ancestry::new(tree);
    for node in tree.nodes() {
        ensure!(ancestry.resolve(node, MAX_TREE_DEPTH).is_some(), TreeTooDeepSnafu { max: MAX_TREE_DEPTH });
    }
    Ok(())
}

#[cfg(test)]
thread_local! {
    static CLIMBED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
/// Compte une lecture de `parent()` par [`Ancestry`] (tests de coût).
fn climb() {
    CLIMBED.with(|c| c.set(c.get() + 1));
}

#[cfg(test)]
/// Lectures de `parent()` par [`Ancestry`] depuis le début du fil (tests de coût).
pub(in crate::replica) fn climbed() -> usize {
    CLIMBED.with(std::cell::Cell::get)
}

#[cfg(test)]
#[path = "entry_tests.rs"]
mod entry_tests;
