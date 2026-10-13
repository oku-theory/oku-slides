//! Placement des éléments voulus sous leur parent : ordre minimal (LIS).

use std::collections::{BTreeMap, HashMap, HashSet};

use loro::{LoroTree, LoroValue, TreeID, TreeParentId};
use snafu::{OptionExt, ResultExt};

use super::super::schema::Ctx;
use super::super::write::{Res, Writer, set_value};
use super::{Wanted, slot_map_at};
use crate::error::{LoroSnafu, ReplicaInner, TreeOrderSnafu};

/// Les positions d'une plus longue sous-suite croissante de `ranks` (stables).
///
/// Patience, O(k log k) ; à longueur égale, la sous-suite qui finit au plus
/// petit rang.
pub(super) fn lis(ranks: &[usize]) -> Vec<bool> {
    // `tails[l]` : position de la plus petite fin d'une sous-suite de longueur `l + 1`.
    let mut tails: Vec<usize> = Vec::new();
    let mut prev: Vec<Option<usize>> = vec![None; ranks.len()];
    for (i, r) in ranks.iter().enumerate() {
        let l = tails.partition_point(|&t| ranks[t] < *r);
        prev[i] = l.checked_sub(1).map(|p| tails[p]);
        match tails.get_mut(l) {
            Some(t) => *t = i,
            None => tails.push(i),
        }
    }
    let mut out = vec![false; ranks.len()];
    let mut at = tails.last().copied();
    while let Some(i) = at {
        out[i] = true;
        at = prev[i];
    }
    out
}

/// Place les nœuds `list` sous `parent`, dans cet ordre.
/// Un `None` est une erreur d'ordre. `present(n)` : `n` est déjà dans ce groupe. Une plus longue
/// sous-suite ordonnée ne bouge pas. Rend chaque nœud.
///
/// L'appariement est complet : `sync_deck` a déjà créé les nœuds manquants.
///
/// `fresh` : les nœuds que `sync_deck` vient de créer, en fin de `parent`. Ils sont
/// hors de la sous-suite stable (un ajout ne déplace jamais un nœud existant) et ne
/// sont déplacés que s'ils ne sont pas déjà à leur place.
///
/// `kids` : les enfants de `parent`, tenus à jour à chaque `mov`.
pub(super) fn arrange(
    tree: &LoroTree,
    parent: TreeParentId,
    kids: &mut Vec<TreeID>,
    list: &[Option<TreeID>],
    present: &dyn Fn(TreeID) -> bool,
    fresh: &HashSet<TreeID>,
) -> Result<Vec<TreeID>, ReplicaInner> {
    let rank: HashMap<TreeID, usize> = list.iter().enumerate().filter_map(|(r, n)| Some(((*n)?, r))).collect();
    // Les enfants déjà là, dans ce groupe, et leur rang dans `list`.
    let here: Vec<(TreeID, usize)> =
        kids.iter().filter(|n| present(**n) && !fresh.contains(*n)).filter_map(|n| Some((*n, *rank.get(n)?))).collect();
    let ranks: Vec<usize> = here.iter().map(|(_, r)| *r).collect();
    let mut stable: Vec<Option<TreeID>> = vec![None; list.len()];
    for ((n, r), keep) in here.iter().zip(lis(&ranks)) {
        if keep && let Some(s) = stable.get_mut(*r) {
            *s = Some(*n);
        }
    }
    // `next[r]` : premier nœud stable après le rang `r`.
    let mut next: Vec<Option<TreeID>> = vec![None; list.len()];
    let mut after = None;
    for (n, s) in next.iter_mut().zip(&stable).rev() {
        *n = after;
        after = s.or(after);
    }
    let mut out = Vec::with_capacity(list.len());
    let mut pred: Option<TreeID> = None;
    for ((want, stable), next_stable) in list.iter().zip(stable).zip(next) {
        let n = match (stable, *want) {
            (Some(n), _) => n,
            (None, Some(n)) if fresh.contains(&n) && in_place(kids, n, pred, next_stable) => n,
            (None, Some(n)) => {
                match (pred, next_stable) {
                    (Some(p), _) => tree.mov_after(n, p).context(LoroSnafu)?,
                    (None, Some(s)) => tree.mov_before(n, s).context(LoroSnafu)?,
                    (None, None) => tree.mov(n, parent).context(LoroSnafu)?,
                }
                kids.retain(|k| *k != n);
                let at = insertion(kids, pred, next_stable);
                kids.insert(at, n);
                n
            }
            (None, None) => return TreeOrderSnafu.fail(),
        };
        out.push(n);
        pred = Some(n);
    }
    Ok(out)
}

/// Vrai si `n`, nœud neuf, est déjà où `arrange` le déplacerait.
fn in_place(kids: &[TreeID], n: TreeID, pred: Option<TreeID>, next: Option<TreeID>) -> bool {
    let at = |x: TreeID| kids.iter().position(|k| *k == x);
    let Some(i) = at(n) else { return false };
    match (pred, next) {
        (Some(p), _) => at(p).is_some_and(|j| j + 1 == i),
        (None, Some(s)) => at(s).is_some_and(|j| i < j),
        (None, None) => true,
    }
}

/// Position dans `kids` qui suit `pred`, sinon celle de `next`, sinon la fin.
///
/// Cherchée depuis la fin : `pred` est le plus souvent près de la fin (nœuds placés à la suite).
fn insertion(kids: &[TreeID], pred: Option<TreeID>, next: Option<TreeID>) -> usize {
    let at = |n: TreeID| kids.iter().rposition(|k| *k == n);
    match (pred, next) {
        (Some(p), _) => at(p).map_or(kids.len(), |i| i + 1),
        (None, Some(s)) => at(s).unwrap_or(kids.len()),
        (None, None) => kids.len(),
    }
}

/// Ce que le placement d'une slide consulte : éléments voulus et leur appariement.
pub(in crate::replica::tree) struct Layout<'a> {
    pub wanted: &'a [Wanted<'a>],
    pub paired: &'a [Option<TreeID>],
    /// Slot résolu de chaque nœud vivant.
    pub slots: &'a HashMap<TreeID, String>,
    pub fresh: &'a HashSet<TreeID>,
}

impl Writer<'_> {
    /// Temps 2 d'une slide : place ses éléments voulus, appariés par `paired`.
    ///
    /// La slide, puis chaque élément voulu ; un propriétaire précède toujours
    /// ses éléments. `slots` : slot résolu de chaque nœud vivant, à l'index.
    pub(in crate::replica::tree) fn place_elements(&self, tree: &LoroTree, slide: TreeID, layout: &Layout<'_>) -> Res {
        let wanted = layout.wanted;
        let mut by_owner: Vec<BTreeMap<&str, Vec<usize>>> = vec![BTreeMap::new(); wanted.len() + 1];
        for (i, w) in wanted.iter().enumerate() {
            let group = by_owner.get_mut(w.owner.map_or(0, |o| o + 1)).context(TreeOrderSnafu)?;
            group.entry(w.slot).or_default().push(i);
        }
        let mut placed: Vec<Option<TreeID>> = vec![None; wanted.len()];
        for (o, groups) in by_owner.iter().enumerate() {
            if groups.is_empty() {
                continue;
            }
            let parent = match o.checked_sub(1) {
                None => slide,
                Some(p) => placed.get(p).copied().flatten().context(TreeOrderSnafu)?,
            };
            self.place(tree, parent, groups, layout, &mut placed)?;
        }
        Ok(())
    }

    /// Place les enfants voulus `groups` de `parent`, slot par slot.
    fn place(
        &self,
        tree: &LoroTree,
        parent: TreeID,
        groups: &BTreeMap<&str, Vec<usize>>,
        layout: &Layout<'_>,
        placed: &mut [Option<TreeID>],
    ) -> Res {
        let Layout { wanted, paired, slots, fresh } = *layout;
        let pid = parent.to_string();
        let tp = TreeParentId::Node(parent);
        let mut kids = tree.children(tp).unwrap_or_default();
        for (&slot, list) in groups {
            let nodes: Vec<Option<TreeID>> = list.iter().map(|w| paired.get(*w).copied().flatten()).collect();
            let present = |n: TreeID| slots.get(&n).is_some_and(|s| s == slot);
            let arranged = arrange(tree, tp, &mut kids, &nodes, &present, fresh)?;
            for (&w, n) in list.iter().zip(arranged) {
                let meta = tree.get_meta(n).context(LoroSnafu)?;
                set_value(&slot_map_at(&meta)?, &pid, LoroValue::from(slot))?;
                self.sync_map(&meta, wanted.get(w).context(TreeOrderSnafu)?.obj, Ctx::Element)?;
                *placed.get_mut(w).context(TreeOrderSnafu)? = Some(n);
            }
        }
        Ok(())
    }
}
