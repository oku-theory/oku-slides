//! Les éléments dans l'arbre du deck : placement par id, ordre des enfants.
//!
//! Le deck est un seul `LoroTree` sous [`SLIDES`](super::schema::SLIDES) : ses
//! racines sont les slides (`tree/deck.rs`), leurs descendants les éléments
//! des slots. La méta d'un nœud est la carte de l'élément, plus [`SLOT`] : le
//! nom du slot, par `TreeID` du parent. Un déplacement, même d'un slot ou
//! d'une slide à l'autre, est un `mov` : les éditions concurrentes du nœud
//! restent. Un retrait est une marque [`mark_key`](super::schema::mark_key)
//! du pair, jamais un `tree.delete` : annuler la retire, et le nœud revient
//! avec ce que les autres pairs y ont fait.

use std::collections::{BTreeMap, HashSet};

use loro::{Container, ContainerTrait, LoroMap, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer};
use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};

use super::schema::{Kind, ORPHANS, SLIDE, SLOT, mark_peer, param_kind};
use super::write::{Res, Writer, keyed, set_value};
use crate::error::{LoroSnafu, MalformedDeckTreeSnafu, ReplicaInner, SlotsTooDeepSnafu, TreeOrderSnafu};
use crate::locate::MAX_SLOT_DEPTH;
use crate::widget::{ParamSchema, WidgetRegistry};

mod deck;
mod place;

use place::arrange;
#[cfg(test)]
use place::lis;

pub(super) use deck::{deck_tree_at, slide_order, stray_roots, wanted_slides};

/// L'arbre de `map[key]`, créé au besoin ; erreur si la clé porte autre chose.
pub(super) fn tree_at(map: &LoroMap, key: &str) -> Result<LoroTree, ReplicaInner> {
    let tree = match map.get(key) {
        Some(ValueOrContainer::Container(Container::Tree(t))) => t,
        None => map.insert_container(key, LoroTree::new()).context(LoroSnafu)?,
        Some(_) => return MalformedDeckTreeSnafu { what: "slides are not a tree" }.fail(),
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

/// Vrai si `key` est une marque de suppression bien formée de valeur `true` (R1).
pub(super) fn is_mark(meta: &LoroMap, key: &str) -> bool {
    mark_peer(key).is_some() && matches!(meta.get(key), Some(ValueOrContainer::Value(LoroValue::Bool(true))))
}

/// Vrai si la méta porte une marque de suppression (R1).
///
/// Une marque est une clé `@deleted/<pair>` de valeur `true` ; le contrôle
/// d'entrée refuse toute autre forme.
pub(super) fn is_marked(meta: &LoroMap) -> bool {
    meta.keys().any(|k| is_mark(meta, &k))
}

/// La carte `@slot` fusionnable d'une méta, créée au besoin (R3).
///
/// Fusionnable : deux pairs qui la créent en concurrence sur un même nœud
/// gardent leurs deux entrées (parade t-188). Un `@slot` d'une autre forme est
/// refusé à l'entrée ; il ne se voit ici que par un bogue.
pub(super) fn slot_map_at(meta: &LoroMap) -> Result<LoroMap, ReplicaInner> {
    match meta.get(SLOT) {
        None => meta.ensure_mergeable_map(SLOT).context(LoroSnafu),
        Some(ValueOrContainer::Container(Container::Map(m))) if m.id().is_mergeable() => Ok(m),
        Some(_) => MalformedDeckTreeSnafu { what: "@slot is not a mergeable map" }.fail(),
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

/// Le slot inscrit dans `@slot` pour un autre parent (R2, règle 3), et s'il est réparé.
///
/// Seules comptent les clés `TreeID` canoniques à valeur non vide. 3a : si
/// les clés de nœuds absents ou supprimés donnent toutes le même slot, il
/// est pris sans réparation (parent recréé par « rétablir »). 3b : sinon, la
/// première clé, nœuds vivants compris, avec réparation.
fn inscribed_slot(tree: &LoroTree, meta: &LoroMap) -> Option<(String, bool)> {
    let slots = slot_map(meta)?;
    let mut entries: Vec<(String, TreeID)> = slots
        .keys()
        .filter_map(|k| {
            let id = TreeID::try_from(&*k).ok()?;
            (id.to_string() == *k).then(|| (k.to_string(), id))
        })
        .collect();
    entries.sort();
    let named: Vec<(TreeID, String)> =
        entries.into_iter().filter_map(|(k, id)| Some((id, name(slots.get(&k))?))).collect();
    let gone = |id: &TreeID| !tree.contains(*id) || tree.is_node_deleted(id).unwrap_or(true);
    let mut vanished = named.iter().filter(|(id, _)| gone(id)).map(|(_, slot)| slot);
    if let Some(first) = vanished.next()
        && vanished.all(|slot| slot == first)
    {
        return Some((first.clone(), false));
    }
    named.into_iter().next().map(|(_, slot)| (slot, true))
}

/// Un enfant, son slot résolu, et si ce slot a été réparé (règles 2 à 4).
#[derive(Debug, Clone)]
pub(super) struct Child {
    pub(super) node: TreeID,
    pub(super) slot: String,
    pub(super) repaired: bool,
    /// Sa méta, lue une fois ; `None` si elle est illisible.
    pub(super) meta: Option<LoroMap>,
}

/// Les enfants visibles d'un parent, groupés par slot résolu.
#[derive(Debug, Default)]
pub(super) struct Children {
    /// Par nom de slot, dans l'ordre des enfants de Loro.
    pub(super) slots: BTreeMap<String, Vec<Child>>,
}

impl Children {
    /// Nombre d'enfants visibles, tous slots confondus.
    pub(super) fn len(&self) -> usize {
        self.slots.values().map(Vec::len).sum()
    }
}

/// Enfants visibles de `parent` (de `TreeID` `pid`), groupés (stable) par slot résolu.
///
/// La méta de chaque enfant est lue une fois. Un enfant marqué, vide (méta
/// sans clé, R1) ou portant `@slide` (refusé à l'entrée) est écarté avec son
/// sous-arbre avant le calcul du slot hérité (R2, règle 2), sans `Issue`.
pub(super) fn ordered_children(tree: &LoroTree, parent: TreeParentId, pid: &str) -> Children {
    let kids: Vec<(TreeID, Option<LoroMap>)> = tree
        .children(parent)
        .unwrap_or_default()
        .into_iter()
        .map(|k| {
            #[cfg(test)]
            deck::visit();
            (k, tree.get_meta(k).ok())
        })
        .filter(|(_, meta)| meta.as_ref().is_none_or(|m| !m.is_empty() && !is_marked(m) && m.get(SLIDE).is_none()))
        .collect();
    let own: Vec<Option<String>> = kids.iter().map(|(_, m)| m.as_ref().and_then(|m| slot_for(m, pid))).collect();
    let sibling = own.iter().flatten().next().cloned();
    let mut out = Children::default();
    for ((node, meta), own) in kids.into_iter().zip(own) {
        let (slot, repaired) = match (own, &sibling) {
            (Some(slot), _) => (slot, false),
            (None, Some(slot)) => (slot.clone(), true),
            (None, None) => {
                meta.as_ref().and_then(|m| inscribed_slot(tree, m)).unwrap_or_else(|| (ORPHANS.to_owned(), true))
            }
        };
        out.slots.entry(slot.clone()).or_default().push(Child { node, slot, repaired, meta });
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

/// Clé de réconciliation d'un objet : son id et son rang d'occurrence.
type Key = (String, usize);

/// Un nœud élément vivant, dans l'ordre `uorder` de sa slide.
struct Live {
    key: Key,
    node: TreeID,
    /// `kind` de la méta (appariement d'une slide à l'autre).
    kind: String,
    /// Slot résolu sous son parent.
    slot: String,
    /// Index du parent dans l'index ; `None` sous la slide.
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

/// La chaîne `key` d'une méta ; vide s'il n'y en a pas.
fn meta_str(meta: &LoroMap, key: &str) -> String {
    name(meta.get(key)).unwrap_or_default()
}

/// Un nœud trouvé par [`live_index`] : nœud, id, `kind`, slot, parent.
type Found = (TreeID, String, String, String, Option<usize>);

/// Nœuds éléments visibles de `slide`, en ordre `uorder`. Bornés par `MAX_SLOT_DEPTH` ; rend aussi leur préordre.
///
/// Jamais sous un nœud marqué. La méta de chaque nœud n'est lue qu'une fois,
/// par [`ordered_children`].
fn live_index(tree: &LoroTree, slide: TreeID) -> (Vec<Live>, Vec<usize>) {
    let mut found: Vec<Found> = Vec::new();
    let mut depths: Vec<usize> = Vec::new();
    let mut stack: Vec<Step<Option<usize>, LiveSlot>> = vec![Step::Expand(None)];
    while let Some(step) = stack.pop() {
        match step {
            Step::Expand(parent) => {
                let (node, depth) = match parent {
                    None => (slide, 0),
                    Some(i) => (found[i].0, depths[i] + 1),
                };
                if depth >= MAX_SLOT_DEPTH {
                    continue;
                }
                let groups = ordered_children(tree, TreeParentId::Node(node), &node.to_string()).slots;
                stack.extend(groups.into_values().rev().map(|kids| Step::Slot(LiveSlot { parent, depth, kids })));
            }
            Step::Slot(LiveSlot { parent, depth, kids }) => {
                let first = found.len();
                for c in kids {
                    let (id, kind) = match &c.meta {
                        Some(m) => (meta_str(m, "id"), meta_str(m, "kind")),
                        None => (String::new(), String::new()),
                    };
                    found.push((c.node, id, kind, c.slot, parent));
                    depths.push(depth);
                }
                stack.extend((first..found.len()).rev().map(|i| Step::Expand(Some(i))));
            }
        }
    }
    let ids: Vec<String> = found.iter().map(|f| f.1.clone()).collect();
    let mut live: Vec<Live> = found
        .into_iter()
        .zip(keyed(&ids))
        .map(|((node, _, kind, slot, parent), key)| Live { key, node, kind, slot, parent, end: 0 })
        .collect();
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
    /// Temps 1 d'une slide : marque les non voulus. Rend les différés.
    ///
    /// Un nœud non voulu sans descendant voulu est marqué, sous-arbre sauté ;
    /// un nœud non voulu dont un descendant est voulu est différé (temps 3).
    /// Avec `whole` faux (slide retirée, cachée par sa propre marque), seuls
    /// les descendants d'un nœud voulu sont touchés : ceux qui en sortent.
    fn mark_elements(
        &self,
        tree: &LoroTree,
        live: &[Live],
        pre: &[usize],
        wanted: &HashSet<TreeID>,
        whole: bool,
    ) -> Result<Vec<TreeID>, ReplicaInner> {
        let mut has_wanted: Vec<bool> = live.iter().map(|l| wanted.contains(&l.node)).collect();
        for &i in pre.iter().rev() {
            if has_wanted.get(i).copied().unwrap_or_default()
                && let Some(p) = live.get(i).and_then(|l| l.parent)
                && let Some(h) = has_wanted.get_mut(p)
            {
                *h = true;
            }
        }
        // Sous un nœud voulu ; le préordre lit un parent avant ses enfants.
        let mut inside = vec![whole; live.len()];
        if !whole {
            for &i in pre {
                let Some(p) = live.get(i).and_then(|l| l.parent) else { continue };
                let under =
                    live.get(p).is_some_and(|l| wanted.contains(&l.node)) || inside.get(p).copied().unwrap_or_default();
                if let Some(x) = inside.get_mut(i) {
                    *x = under;
                }
            }
        }
        let mut deferred = Vec::new();
        let mut at = 0;
        while let Some(&i) = pre.get(at) {
            let l = live.get(i).context(TreeOrderSnafu)?;
            if wanted.contains(&l.node) {
                at += 1;
            } else if !inside.get(i).copied().unwrap_or_default() {
                // Hors de tout nœud voulu : caché avec la slide.
                at = if has_wanted.get(i).copied().unwrap_or_default() { at + 1 } else { l.end.max(at + 1) };
            } else if has_wanted.get(i).copied().unwrap_or_default() {
                deferred.push(l.node);
                at += 1;
            } else {
                self.mark(tree, l.node)?;
                at = l.end.max(at + 1);
            }
        }
        Ok(deferred)
    }

    /// Marque `node` retiré par ce pair (R1), sans `tree.delete`.
    ///
    /// Ne vise qu'un nœud visible : Loro saute un `insert` de même valeur, et
    /// l'annulation n'aurait alors rien à défaire.
    fn mark(&self, tree: &LoroTree, node: TreeID) -> Res {
        set_value(&tree.get_meta(node).context(LoroSnafu)?, &self.mark_key, LoroValue::Bool(true))
    }
}

#[cfg(test)]
/// Le nœud visible d'id `id` de la slide `slide`, et l'arbre du deck (tests).
///
/// Parcours brut et itératif, en préordre, sans borne de profondeur : un nœud
/// marqué (clé `@deleted/…`) est sauté avec son sous-arbre.
pub(super) fn find_node(doc: &loro::LoroDoc, slide: &str, id: &str) -> Option<(LoroTree, TreeID)> {
    let is = |m: &LoroMap, want: &str| name(m.get("id")).as_deref() == Some(want);
    let marked = |m: &LoroMap| m.keys().any(|k| k.starts_with("@deleted/"));
    let Some(ValueOrContainer::Container(Container::Tree(tree))) = doc.get_map(super::ROOT).get(super::schema::SLIDES)
    else {
        panic!("arbre du deck attendu")
    };
    let root = tree.children(TreeParentId::Root).unwrap_or_default().into_iter().find(|n| {
        let meta = tree.get_meta(*n).unwrap_or_else(|e| panic!("méta de {n:?} : {e}"));
        !marked(&meta) && is(&meta, slide)
    })?;
    let mut stack: Vec<TreeID> = tree.children(TreeParentId::Node(root)).unwrap_or_default();
    stack.reverse();
    while let Some(n) = stack.pop() {
        let meta = tree.get_meta(n).unwrap_or_else(|e| panic!("méta de {n:?} : {e}"));
        if marked(&meta) {
            continue;
        }
        if is(&meta, id) {
            return Some((tree, n));
        }
        stack.extend(tree.children(TreeParentId::Node(n)).unwrap_or_default().into_iter().rev());
    }
    None
}

#[cfg(test)]
/// Le nœud visible d'id `id` de la slide `slide`, et l'arbre du deck (tests) ; panique s'il manque.
pub(super) fn node_of(doc: &loro::LoroDoc, slide: &str, id: &str) -> (LoroTree, TreeID) {
    find_node(doc, slide, id).unwrap_or_else(|| panic!("élément {id} absent de {slide}"))
}

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tree_tests;

#[cfg(test)]
#[path = "tree_read_tests.rs"]
mod tree_read_tests;

#[cfg(test)]
#[path = "soft_delete_tests.rs"]
mod soft_delete_tests;
