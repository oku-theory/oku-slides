//! L'arbre des éléments d'une slide : placement par id, ordre des enfants.
//!
//! Chaque slide porte un `LoroTree` sous [`ELEMENTS`]. Ses racines sont les
//! éléments des slots de la slide ; les enfants d'un nœud, ceux des slots de
//! l'élément. La méta d'un nœud est la carte de l'élément, plus [`SLOT`] :
//! le nom du slot, par id de parent. Un déplacement, même d'un slot à un
//! autre, est un `mov` : les éditions concurrentes du nœud restent.

use std::collections::{BTreeMap, HashMap};

use loro::{Container, LoroMap, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer};
use serde_json::{Map, Value};
use snafu::{ResultExt, ensure};

use super::schema::{Ctx, Kind, ORPHANS, ROOT_OWNER, SLOT, param_kind};
use super::write::{Res, Writer, keyed, map_at, set_value};
use crate::error::{ElementsNotATreeSnafu, LoroSnafu, ReplicaInner, SlotsTooDeepSnafu, TreeOrderSnafu};
use crate::locate::MAX_SLOT_DEPTH;
use crate::widget::{ParamSchema, WidgetRegistry};

/// L'arbre de `map[key]`, créé au besoin ; erreur si la clé porte autre chose.
pub(super) fn tree_at(map: &LoroMap, key: &str) -> Result<LoroTree, ReplicaInner> {
    let tree = match map.get(key) {
        Some(ValueOrContainer::Container(Container::Tree(t))) => t,
        None => map.insert_container(key, LoroTree::new()).context(LoroSnafu)?,
        Some(_) => return ElementsNotATreeSnafu.fail(),
    };
    // Actif par défaut en 1.16.2 ; explicité (sonde, ADR-0005).
    tree.enable_fractional_index(0);
    Ok(tree)
}

/// La carte `@slot` d'une méta, si c'en est une.
fn slot_map(meta: &LoroMap) -> Option<LoroMap> {
    match meta.get(SLOT) {
        Some(ValueOrContainer::Container(Container::Map(m))) => Some(m),
        _ => None,
    }
}

/// Une chaîne non vide.
fn name(v: Option<ValueOrContainer>) -> Option<String> {
    match v {
        Some(ValueOrContainer::Value(LoroValue::String(s))) if !s.is_empty() => Some(s.to_string()),
        _ => None,
    }
}

/// Le slot de `@slot[owner]` : chaîne non vide, sinon `None`.
pub(super) fn slot_for(meta: &LoroMap, owner: &str) -> Option<String> {
    name(slot_map(meta)?.get(owner))
}

/// Première valeur valide de `@slot`, par ordre des clés (règle 3).
pub(super) fn any_slot(meta: &LoroMap) -> Option<String> {
    let slots = slot_map(meta)?;
    let mut keys: Vec<String> = slots.keys().map(|k| k.to_string()).collect();
    keys.sort();
    keys.into_iter().find_map(|k| name(slots.get(&k)))
}

/// Un enfant, son slot résolu, et si ce slot a été réparé (règles 2 à 4).
#[derive(Debug, Clone)]
pub(super) struct Child {
    pub(super) node: TreeID,
    pub(super) slot: String,
    pub(super) repaired: bool,
}

/// Enfants vivants de `parent` (d'id `pid`), groupés (stable) par slot résolu.
pub(super) fn ordered_children(tree: &LoroTree, parent: TreeParentId, pid: &str) -> BTreeMap<String, Vec<Child>> {
    let kids = tree.children(parent).unwrap_or_default();
    let metas: Vec<Option<LoroMap>> = kids.iter().map(|k| tree.get_meta(*k).ok()).collect();
    let own: Vec<Option<String>> = metas.iter().map(|m| m.as_ref().and_then(|m| slot_for(m, pid))).collect();
    let sibling = own.iter().flatten().next().cloned();
    let mut out: BTreeMap<String, Vec<Child>> = BTreeMap::new();
    for ((node, meta), own) in kids.into_iter().zip(&metas).zip(own) {
        let repaired = own.is_none();
        let slot = own
            .or_else(|| sibling.clone())
            .or_else(|| meta.as_ref().and_then(any_slot))
            .unwrap_or_else(|| ORPHANS.to_owned());
        out.entry(slot.clone()).or_default().push(Child { node, slot, repaired });
    }
    out
}

/// Les éléments d'une valeur tableau ; aucun pour une autre valeur.
pub(super) fn elements(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or_default()
}

/// Les slots de `params` (catégorie `Kind::Slot`), triés par nom.
pub(super) fn slots_of<'v>(
    params: Option<&'v Map<String, Value>>,
    schema: Option<&[ParamSchema]>,
) -> Vec<(&'v str, &'v [Value])> {
    let mut out: Vec<(&str, &[Value])> = params
        .into_iter()
        .flatten()
        .filter(|(name, value)| param_kind(schema, name, value) == Kind::Slot)
        .map(|(name, value)| (name.as_str(), elements(value)))
        .collect();
    out.sort_by_key(|(name, _)| *name);
    out
}

/// Les positions d'une plus longue sous-suite croissante de `ranks` (stables).
///
/// Patience, O(k log k) ; à longueur égale, la sous-suite qui finit au plus
/// petit rang.
fn lis(ranks: &[usize]) -> Vec<bool> {
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

/// Clé de réconciliation d'un élément : son id et son rang d'occurrence.
type Key = (String, usize);

/// Un nœud vivant de l'arbre, dans l'ordre `uorder`.
struct Live {
    key: Key,
    node: TreeID,
    /// Index du parent dans l'index ; `None` pour une racine.
    parent: Option<usize>,
    /// Fin (exclue) de son sous-arbre, en positions du préordre.
    end: usize,
}

/// Un élément voulu, dans l'ordre `uorder`.
pub(super) struct Wanted<'a> {
    key: Key,
    /// Index du propriétaire ; `None` : la slide.
    owner: Option<usize>,
    slot: &'a str,
    obj: &'a Map<String, Value>,
}

/// Les deux index d'une réconciliation, et leurs correspondances.
struct Index<'a> {
    wanted: Vec<Wanted<'a>>,
    /// Index dans `wanted` d'une clé voulue.
    wanted_at: HashMap<Key, usize>,
    /// Nœud vivant d'une clé de l'arbre.
    known: HashMap<Key, TreeID>,
    /// Clé d'un nœud vivant.
    key_of: HashMap<TreeID, Key>,
}

/// Un slot à compter du parcours de l'arbre : ses enfants, sous `parent`.
struct LiveSlot {
    parent: Option<usize>,
    depth: usize,
    kids: Vec<Child>,
}

/// Un slot à compter du parcours du JSON : ses éléments, de `owner`.
struct WantedSlot<'a> {
    owner: Option<usize>,
    depth: usize,
    slot: &'a str,
    elements: &'a [Value],
}

/// Pas d'un parcours `uorder` : développer un parent, ou compter un slot.
enum Step<P, S> {
    Expand(P),
    Slot(S),
}

/// L'id d'une méta ; vide s'il n'y en a pas.
fn meta_id(tree: &LoroTree, node: TreeID) -> String {
    tree.get_meta(node).ok().and_then(|m| name(m.get("id"))).unwrap_or_default()
}

/// Nœuds vivants de profondeur < `MAX_SLOT_DEPTH`, dans l'ordre `uorder`, et leur préordre.
fn live_index(tree: &LoroTree) -> (Vec<Live>, Vec<usize>) {
    // (nœud, id, parent), dans l'ordre `uorder` ; les profondeurs sont à part.
    let mut found: Vec<(TreeID, String, Option<usize>)> = Vec::new();
    let mut depths: Vec<usize> = Vec::new();
    let mut stack: Vec<Step<Option<usize>, LiveSlot>> = vec![Step::Expand(None)];
    while let Some(step) = stack.pop() {
        match step {
            Step::Expand(parent) => {
                let (tp, pid, depth) = match parent {
                    None => (TreeParentId::Root, String::from(ROOT_OWNER), 0),
                    Some(i) => (TreeParentId::Node(found[i].0), found[i].1.clone(), depths[i] + 1),
                };
                if depth >= MAX_SLOT_DEPTH {
                    continue;
                }
                let groups = ordered_children(tree, tp, &pid);
                stack.extend(groups.into_values().rev().map(|kids| Step::Slot(LiveSlot { parent, depth, kids })));
            }
            Step::Slot(LiveSlot { parent, depth, kids }) => {
                let first = found.len();
                for c in kids {
                    found.push((c.node, meta_id(tree, c.node), parent));
                    depths.push(depth);
                }
                stack.extend((first..found.len()).rev().map(|i| Step::Expand(Some(i))));
            }
        }
    }
    let ids: Vec<String> = found.iter().map(|(_, id, _)| id.clone()).collect();
    let mut live: Vec<Live> =
        found.into_iter().zip(keyed(&ids)).map(|((node, _, parent), key)| Live { key, node, parent, end: 0 }).collect();
    // Préordre : enfants dans l'ordre de l'index.
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); live.len()];
    let mut roots = Vec::new();
    for (i, l) in live.iter().enumerate() {
        match l.parent {
            Some(p) => children[p].push(i),
            None => roots.push(i),
        }
    }
    let mut pre = Vec::with_capacity(live.len());
    let mut stack: Vec<Step<usize, usize>> = roots.into_iter().rev().map(Step::Expand).collect();
    while let Some(step) = stack.pop() {
        match step {
            Step::Expand(i) => {
                pre.push(i);
                stack.push(Step::Slot(i));
                stack.extend(children[i].iter().rev().map(|c| Step::Expand(*c)));
            }
            // Sous-arbre de `i` parcouru : sa fin est la position courante.
            Step::Slot(i) => live[i].end = pre.len(),
        }
    }
    (live, pre)
}

/// Éléments voulus dans l'ordre `uorder` ; `SlotsTooDeep` au-delà de la borne.
pub(super) fn wanted_index<'a>(
    registry: &WidgetRegistry,
    roots: &[(&'a str, &'a [Value])],
) -> Result<Vec<Wanted<'a>>, ReplicaInner> {
    let mut found: Vec<(Option<usize>, &'a str, &'a Map<String, Value>)> = Vec::new();
    let mut depths: Vec<usize> = Vec::new();
    let mut stack: Vec<Step<usize, WantedSlot<'a>>> = roots
        .iter()
        .rev()
        .map(|(slot, elements)| Step::Slot(WantedSlot { owner: None, depth: 0, slot, elements }))
        .collect();
    while let Some(step) = stack.pop() {
        match step {
            Step::Expand(i) => {
                let obj = found[i].2;
                let schema = obj.get("kind").and_then(Value::as_str).and_then(|k| registry.get(k));
                let params = obj.get("params").and_then(Value::as_object);
                let slots = slots_of(params, schema.map(|w| w.params.as_slice()));
                stack.extend(slots.into_iter().rev().map(|(slot, elements)| {
                    Step::Slot(WantedSlot { owner: Some(i), depth: depths[i] + 1, slot, elements })
                }));
            }
            Step::Slot(WantedSlot { owner, depth, slot, elements }) => {
                let first = found.len();
                for obj in elements.iter().filter_map(Value::as_object) {
                    ensure!(depth < MAX_SLOT_DEPTH, SlotsTooDeepSnafu { max: MAX_SLOT_DEPTH });
                    found.push((owner, slot, obj));
                    depths.push(depth);
                }
                stack.extend((first..found.len()).rev().map(Step::Expand));
            }
        }
    }
    let id_of = |obj: &Map<String, Value>| obj.get("id").and_then(Value::as_str).unwrap_or_default().to_owned();
    let ids: Vec<String> = found.iter().map(|(_, _, obj)| id_of(obj)).collect();
    Ok(found.into_iter().zip(keyed(&ids)).map(|((owner, slot, obj), key)| Wanted { key, owner, slot, obj }).collect())
}

impl Writer<'_> {
    /// Amène l'arbre `tree` aux éléments voulus `wanted` de la slide.
    ///
    /// `wanted` vient de [`wanted_index`], appelé avant toute écriture.
    ///
    /// Temps 0 : index, sans écriture. Temps 1 : suppressions des nœuds non
    /// voulus sans descendant voulu. Temps 2 : placement par (parent, slot),
    /// en laissant en place une plus longue sous-suite déjà ordonnée. Temps
    /// 3 : suppression des nœuds non voulus dont les descendants voulus sont
    /// partis.
    pub(super) fn sync_tree(&self, tree: &LoroTree, wanted: Vec<Wanted<'_>>) -> Res {
        let (current, pre) = live_index(tree);
        let ix = Index {
            wanted_at: wanted.iter().enumerate().map(|(i, w)| (w.key.clone(), i)).collect(),
            known: current.iter().map(|l| (l.key.clone(), l.node)).collect(),
            key_of: current.iter().map(|l| (l.node, l.key.clone())).collect(),
            wanted,
        };
        let wanted = &ix.wanted;
        let wanted_at = &ix.wanted_at;

        // Temps 1.
        let mut has_wanted: Vec<bool> = current.iter().map(|l| wanted_at.contains_key(&l.key)).collect();
        for &i in pre.iter().rev() {
            if has_wanted[i]
                && let Some(p) = current[i].parent
            {
                has_wanted[p] = true;
            }
        }
        let mut deferred = Vec::new();
        let mut at = 0;
        while let Some(&i) = pre.get(at) {
            let l = &current[i];
            if wanted_at.contains_key(&l.key) {
                at += 1;
            } else if has_wanted[i] {
                deferred.push(l.node);
                at += 1;
            } else {
                tree.delete(l.node).context(LoroSnafu)?;
                at = l.end;
            }
        }

        // Temps 2 : la slide, puis chaque élément voulu ; un propriétaire
        // précède toujours ses éléments.
        let mut by_owner: Vec<BTreeMap<&str, Vec<usize>>> = vec![BTreeMap::new(); wanted.len() + 1];
        for (i, w) in wanted.iter().enumerate() {
            by_owner[w.owner.map_or(0, |o| o + 1)].entry(w.slot).or_default().push(i);
        }
        let mut placed: Vec<Option<TreeID>> = vec![None; wanted.len()];
        for (o, slots) in by_owner.iter().enumerate() {
            if slots.is_empty() {
                continue;
            }
            let (parent, pid) = match o.checked_sub(1) {
                None => (TreeParentId::Root, ROOT_OWNER),
                Some(p) => match placed[p] {
                    Some(n) => (TreeParentId::Node(n), wanted[p].key.0.as_str()),
                    None => return TreeOrderSnafu.fail(),
                },
            };
            self.place(tree, (parent, pid), slots, &ix, &mut placed)?;
        }

        // Temps 3.
        for n in deferred {
            if !tree.is_node_deleted(&n).context(LoroSnafu)? {
                tree.delete(n).context(LoroSnafu)?;
            }
        }
        Ok(())
    }

    /// Place les enfants voulus `slots` de `parent` (d'id `pid`), slot par slot.
    fn place(
        &self,
        tree: &LoroTree,
        (parent, pid): (TreeParentId, &str),
        slots: &BTreeMap<&str, Vec<usize>>,
        ix: &Index<'_>,
        placed: &mut [Option<TreeID>],
    ) -> Res {
        let wanted = &ix.wanted;
        let mut kids = tree.children(parent).unwrap_or_default();
        // Slots résolus une fois, avant tout `mov` sous ce parent.
        let resolved: HashMap<TreeID, String> =
            ordered_children(tree, parent, pid).into_values().flatten().map(|c| (c.node, c.slot)).collect();
        for (&slot, list) in slots {
            let rank_of: HashMap<usize, usize> = list.iter().enumerate().map(|(r, w)| (*w, r)).collect();
            // Les enfants déjà là, voulus sous (parent, slot), et leur rang dans `list`.
            let present: Vec<(TreeID, usize)> = kids
                .iter()
                .filter(|n| resolved.get(n).is_some_and(|s| s == slot))
                .filter_map(|n| {
                    let w = *ix.wanted_at.get(ix.key_of.get(n)?)?;
                    Some((*n, *rank_of.get(&w)?))
                })
                .collect();
            let ranks: Vec<usize> = present.iter().map(|(_, r)| *r).collect();
            let mut stable: Vec<Option<TreeID>> = vec![None; list.len()];
            for ((n, r), keep) in present.iter().zip(lis(&ranks)) {
                if keep {
                    stable[*r] = Some(*n);
                }
            }
            let mut pred: Option<TreeID> = None;
            for (r, &w) in list.iter().enumerate() {
                let next_stable = stable[r + 1..].iter().flatten().next().copied();
                let n = match (stable[r], ix.known.get(&wanted[w].key)) {
                    (Some(n), _) => n,
                    (None, Some(&n)) => {
                        match (pred, next_stable) {
                            (Some(p), _) => tree.mov_after(n, p).context(LoroSnafu)?,
                            (None, Some(s)) => tree.mov_before(n, s).context(LoroSnafu)?,
                            (None, None) => tree.mov(n, parent).context(LoroSnafu)?,
                        }
                        kids.retain(|k| *k != n);
                        let at = insertion(&kids, pred, next_stable);
                        kids.insert(at, n);
                        n
                    }
                    (None, None) => {
                        let at = insertion(&kids, pred, next_stable);
                        let n = tree.create_at(parent, at).context(LoroSnafu)?;
                        kids.insert(at, n);
                        map_at(&tree.get_meta(n).context(LoroSnafu)?, SLOT)?;
                        n
                    }
                };
                let meta = tree.get_meta(n).context(LoroSnafu)?;
                set_value(&map_at(&meta, SLOT)?, pid, LoroValue::from(slot))?;
                self.sync_map(&meta, wanted[w].obj, Ctx::Element)?;
                placed[w] = Some(n);
                pred = Some(n);
            }
        }
        Ok(())
    }
}

/// Position dans `kids` qui suit `pred`, sinon celle de `next`, sinon la fin.
fn insertion(kids: &[TreeID], pred: Option<TreeID>, next: Option<TreeID>) -> usize {
    let at = |n: TreeID| kids.iter().position(|k| *k == n);
    match (pred, next) {
        (Some(p), _) => at(p).map_or(kids.len(), |i| i + 1),
        (None, Some(s)) => at(s).unwrap_or(kids.len()),
        (None, None) => kids.len(),
    }
}

#[cfg(test)]
/// Le nœud vivant d'id `id` de la slide `slide`, et son arbre (tests).
pub(super) fn node_of(doc: &loro::LoroDoc, slide: &str, id: &str) -> (LoroTree, TreeID) {
    let is = |m: &LoroMap, want: &str| name(m.get("id")).as_deref() == Some(want);
    let Some(ValueOrContainer::Container(Container::MovableList(slides))) = doc.get_map(super::ROOT).get("slides")
    else {
        panic!("pas de liste `slides`")
    };
    let map = (0..slides.len())
        .find_map(|i| match slides.get(i) {
            Some(ValueOrContainer::Container(Container::Map(m))) if is(&m, slide) => Some(m),
            _ => None,
        })
        .unwrap_or_else(|| panic!("slide {slide} absente"));
    let Some(ValueOrContainer::Container(Container::Tree(tree))) = map.get(super::schema::ELEMENTS) else {
        panic!("slide {slide} sans arbre")
    };
    let node = tree
        .nodes()
        .into_iter()
        .filter(|n| !tree.is_node_deleted(n).unwrap_or(true))
        .find(|n| tree.get_meta(*n).is_ok_and(|m| is(&m, id)))
        .unwrap_or_else(|| panic!("élément {id} absent de {slide}"));
    (tree, node)
}

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tree_tests;

#[cfg(test)]
#[path = "tree_read_tests.rs"]
mod tree_read_tests;
