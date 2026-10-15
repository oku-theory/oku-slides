//! L'arbre du deck : slides en racine, appariement d'une slide à l'autre.
//!
//! Les racines du `LoroTree` du deck sont les slides (méta : la slide, plus
//! `@slide = true`) ; leurs descendants, les éléments (`tree.rs`). Une slide
//! et un élément sont appariés par leur `id`, jamais par leur `TreeID` (R5) :
//! un élément ne passe d'une slide à l'autre que si son id est seul de son
//! espèce des deux côtés.

use std::collections::{HashMap, HashSet, VecDeque};

use loro::{Container, LoroDoc, LoroMap, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer};
use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt};

use super::super::entry::Ancestry;
use super::super::schema::{Ctx, SLIDE, SLIDES};
use super::super::write::{Res, Writer, keyed, set_value};
use super::place::Layout;
use super::{Key, Live, Wanted, arrange, is_marked, live_index, meta_str, slots_of, tree_at, wanted_index};
use crate::error::{LoroSnafu, ReplicaInner, TreeOrderSnafu};
use crate::ops::Moved;
use crate::widget::WidgetRegistry;

/// Vrai si la méta porte `@slide = true`.
fn is_slide(meta: &LoroMap) -> bool {
    matches!(meta.get(SLIDE), Some(ValueOrContainer::Value(LoroValue::Bool(true))))
}

/// Les racines de l'arbre et leur méta (lue une fois, sans échec attendu).
fn roots(tree: &LoroTree) -> impl Iterator<Item = (TreeID, LoroMap)> + '_ {
    tree.children(TreeParentId::Root).unwrap_or_default().into_iter().filter_map(|n| {
        #[cfg(test)]
        visit();
        Some((n, tree.get_meta(n).ok()?))
    })
}

/// Les slides visibles : racines non marquées, dans l'ordre de Loro (R4).
///
/// Une racine sans `@slide = true` est ignorée avec son sous-arbre : Loro en
/// crée une, à méta vide, en défaisant le déplacement d'une slide supprimée.
pub(in crate::replica) fn slide_order(tree: &LoroTree) -> Vec<TreeID> {
    roots(tree).filter(|(_, m)| is_slide(m) && !is_marked(m)).map(|(n, _)| n).collect()
}

/// Les racines non vides sans `@slide = true` (lecture seule : `issue-unreadable-slide`).
///
/// Une racine marquée n'en est pas : elle est cachée, comme une slide retirée.
pub(in crate::replica) fn stray_roots(tree: &LoroTree) -> Vec<TreeID> {
    roots(tree).filter(|(_, m)| !m.is_empty() && !is_slide(m) && !is_marked(m)).map(|(n, _)| n).collect()
}

/// L'arbre du deck sous `map[SLIDES]`, créé au besoin.
pub(in crate::replica) fn deck_tree_at(map: &LoroMap) -> Result<LoroTree, ReplicaInner> {
    tree_at(map, SLIDES)
}

/// L'arbre du deck de `doc`, s'il existe ; jamais créé (lecture).
///
/// [`deck_tree_at`] le crée au besoin, pour écrire.
pub(in crate::replica) fn deck_tree_of(doc: &LoroDoc) -> Option<LoroTree> {
    match doc.get_map(crate::replica::ROOT).get(SLIDES) {
        Some(ValueOrContainer::Container(Container::Tree(t))) => Some(t),
        _ => None,
    }
}

/// Une slide voulue et ses éléments voulus (ordre `uorder`).
pub(in crate::replica) struct WantedSlide<'a> {
    key: Key,
    obj: &'a Map<String, Value>,
    elements: Vec<Wanted<'a>>,
}

/// Les slides voulues du deck JSON ; `SlotsTooDeep` avant toute écriture.
pub(in crate::replica) fn wanted_slides<'a>(
    registry: &WidgetRegistry,
    slides: &'a [Value],
) -> Result<Vec<WantedSlide<'a>>, ReplicaInner> {
    let objs: Vec<&'a Map<String, Value>> = slides.iter().filter_map(Value::as_object).collect();
    let ids: Vec<String> =
        objs.iter().map(|o| o.get("id").and_then(Value::as_str).unwrap_or_default().to_owned()).collect();
    objs.into_iter()
        .zip(keyed(&ids))
        .map(|(obj, key)| {
            let schema = obj.get("kind").and_then(Value::as_str).and_then(|k| registry.get(k));
            let params = obj.get("params").and_then(Value::as_object);
            let elements = wanted_index(registry, &slots_of(params, schema.map(|w| w.params.as_slice())))?;
            Ok(WantedSlide { key, obj, elements })
        })
        .collect()
}

/// Une slide de l'arbre, et l'index de ses éléments.
struct LiveSlide {
    key: Key,
    node: TreeID,
    elements: Vec<Live>,
    pre: Vec<usize>,
}

fn live_slides(tree: &LoroTree) -> Vec<LiveSlide> {
    let nodes = slide_order(tree);
    let ancestry = Ancestry::new(tree);
    let ids: Vec<String> = nodes
        .iter()
        .map(|n| {
            #[cfg(test)]
            visit();
            tree.get_meta(*n).map(|m| meta_str(&m, "id")).unwrap_or_default()
        })
        .collect();
    nodes
        .into_iter()
        .zip(keyed(&ids))
        .map(|(node, key)| {
            let (elements, pre) = live_index(tree, &ancestry, node);
            LiveSlide { key, node, elements, pre }
        })
        .collect()
}

/// Le nœud apparié à chaque slide voulue et à chaque élément voulu (R5).
struct Pairing {
    slides: Vec<Option<TreeID>>,
    elements: Vec<Vec<Option<TreeID>>>,
}

fn pair(live: &[LiveSlide], wanted: &[WantedSlide<'_>], moves: &[Moved]) -> Pairing {
    let live_at: HashMap<&Key, usize> = live.iter().enumerate().map(|(i, l)| (&l.key, i)).collect();
    let home: Vec<Option<usize>> = wanted.iter().map(|w| live_at.get(&w.key).copied()).collect();
    let slides = home.iter().map(|l| l.and_then(|l| live.get(l)).map(|l| l.node)).collect();
    // Passe 1 : dans une slide appariée, même clé `(id, n)`.
    let mut elements: Vec<Vec<Option<TreeID>>> = wanted
        .iter()
        .zip(&home)
        .map(|(w, l)| {
            let Some(ls) = l.and_then(|l| live.get(l)) else { return vec![None; w.elements.len()] };
            let by_key: HashMap<&Key, TreeID> = ls.elements.iter().map(|e| (&e.key, e.node)).collect();
            w.elements.iter().map(|e| by_key.get(&e.key).copied()).collect()
        })
        .collect();
    // Passe 2, seulement s'il reste un élément voulu non apparié dans une slide gardée.
    let pending = elements.iter().zip(&home).any(|(e, l)| l.is_some() && e.iter().any(Option::is_none));
    if pending {
        let mut taken: HashSet<TreeID> = elements.iter().flatten().flatten().copied().collect();
        Moves::new(live, wanted, &home, moves).infer(&mut elements, &mut taken);
    }
    Pairing { slides, elements }
}

/// La slide où figure chaque id, s'il n'en a qu'une (`None` : plusieurs).
fn sole_slide<'k>(ids: impl Iterator<Item = (usize, &'k str)>) -> HashMap<&'k str, Option<usize>> {
    let mut out: HashMap<&str, Option<usize>> = HashMap::new();
    for (slide, id) in ids {
        out.entry(id)
            .and_modify(|s| {
                if *s != Some(slide) {
                    *s = None;
                }
            })
            .or_insert(Some(slide));
    }
    out
}

/// Le `kind` d'un objet voulu ; vide s'il n'y en a pas.
fn kind_of(obj: &Map<String, Value>) -> &str {
    obj.get("kind").and_then(Value::as_str).unwrap_or_default()
}

/// Passe 2 de l'appariement : les déplacements d'une slide à l'autre (R5, point 3).
struct Moves<'l, 'w, 'a> {
    live: &'l [LiveSlide],
    wanted: &'w [WantedSlide<'a>],
    home: &'w [Option<usize>],
    /// Les slides vivantes appariées.
    kept: Vec<bool>,
    wanted_sole: HashMap<&'w str, Option<usize>>,
    /// Pour un id d'une seule slide vivante : ses occurrences, en ordre `uorder`.
    occurrences: HashMap<&'l str, (usize, VecDeque<usize>)>,
    /// Déplacements annoncés : (slide voulue, élément) → slide vivante d'origine.
    announced: HashMap<(&'w str, &'w str), usize>,
    /// Occurrences d'un élément annoncé dans sa slide d'origine, en ordre `uorder`.
    announced_at: HashMap<(usize, &'w str), VecDeque<usize>>,
    /// Par slide vivante, la position de chaque élément dans son préordre.
    positions: HashMap<usize, Vec<usize>>,
}

impl<'l, 'w, 'a> Moves<'l, 'w, 'a> {
    fn new(
        live: &'l [LiveSlide],
        wanted: &'w [WantedSlide<'a>],
        home: &'w [Option<usize>],
        moves: &'w [Moved],
    ) -> Self {
        let mut kept = vec![false; live.len()];
        for l in home.iter().flatten() {
            if let Some(k) = kept.get_mut(*l) {
                *k = true;
            }
        }
        let live_ids = live.iter().enumerate().flat_map(|(s, l)| l.elements.iter().map(move |e| (s, e.key.0.as_str())));
        let wanted_ids =
            wanted.iter().enumerate().flat_map(|(s, w)| w.elements.iter().map(move |e| (s, e.key.0.as_str())));
        let live_sole = sole_slide(live_ids);
        // Un seul parcours : chaque élément d'un id à slide unique rejoint ses occurrences.
        let mut occurrences: HashMap<&str, (usize, VecDeque<usize>)> = HashMap::new();
        for (slide, l) in live.iter().enumerate() {
            for (x, e) in l.elements.iter().enumerate() {
                #[cfg(test)]
                visit();
                if live_sole.get(e.key.0.as_str()) == Some(&Some(slide)) {
                    occurrences.entry(e.key.0.as_str()).or_insert_with(|| (slide, VecDeque::new())).1.push_back(x);
                }
            }
        }
        let wanted_sole = sole_slide(wanted_ids);
        let (announced, announced_at) = announced(live, moves);
        Self { live, wanted, home, kept, wanted_sole, occurrences, announced, announced_at, positions: HashMap::new() }
    }

    /// Apparie les racines déplacées, puis leurs descendants (R5, points 3 et 4).
    ///
    /// Un déplacement annoncé passe d'abord ; sinon, déplacement déduit.
    fn infer(mut self, elements: &mut [Vec<Option<TreeID>>], taken: &mut HashSet<TreeID>) {
        let (wanted, homes) = (self.wanted, self.home);
        for (i, (w, paired)) in wanted.iter().zip(elements.iter_mut()).enumerate() {
            let Some(home) = homes.get(i).copied().flatten() else { continue };
            let mut owned: Option<Vec<Vec<usize>>> = None;
            for (j, e) in w.elements.iter().enumerate() {
                let owner_paired = e.owner.is_none_or(|o| paired.get(o).copied().flatten().is_some());
                if paired.get(j).copied().flatten().is_some() || !owner_paired {
                    continue;
                }
                let found = self.announced_root(w, home, e, taken).or_else(|| self.inferred_root(i, home, e, taken));
                let Some((from, x)) = found else { continue };
                let Some(ls) = self.live.get(from) else { continue };
                let Some(root) = ls.elements.get(x) else { continue };
                if let Some(p) = paired.get_mut(j) {
                    *p = Some(root.node);
                }
                taken.insert(root.node);
                let owned = owned.get_or_insert_with(|| owned_by(&w.elements));
                let positions = self.positions.entry(from).or_insert_with(|| preorder_positions(&ls.pre));
                let live_sub = descendants(ls, positions, x);
                pair_subtree(ls, &live_sub, (&w.elements, owned, j), paired, taken);
            }
        }
    }

    /// Déplacement annoncé de `e` vers la slide voulue `w` (gardée en `home`) : `(slide d'origine, élément)`.
    ///
    /// L'origine n'est pas `home` ; elle peut être retirée dans la même
    /// transaction, car l'annonce fait foi. Au temps 1, une slide non gardée
    /// est marquée sur son propre nœud, avec les non voulus sous l'élément ;
    /// le `mov` du temps 2 en sort l'élément. L'élément est la première
    /// occurrence non appariée de l'id dans l'origine, de même `kind`.
    fn announced_root(
        &mut self,
        w: &'w WantedSlide<'a>,
        home: usize,
        e: &'w Wanted<'a>,
        taken: &HashSet<TreeID>,
    ) -> Option<(usize, usize)> {
        if w.key.1 != 0 {
            return None;
        }
        let id = e.key.0.as_str();
        let from = *self.announced.get(&(w.key.0.as_str(), id))?;
        if from == home {
            return None;
        }
        let at = self.announced_at.get_mut(&(from, id))?;
        first_free(self.live.get(from)?, at, taken, kind_of(e.obj)).map(|x| (from, x))
    }

    /// Déplacement déduit de `e` vers la slide voulue `i` (gardée en `home`) : `(slide d'origine, élément)`.
    ///
    /// L'id figure dans une seule slide vivante et une seule slide voulue,
    /// appariées, différentes, et les `kind` sont égaux.
    fn inferred_root(
        &mut self,
        i: usize,
        home: usize,
        e: &'w Wanted<'a>,
        taken: &HashSet<TreeID>,
    ) -> Option<(usize, usize)> {
        let id = e.key.0.as_str();
        if self.wanted_sole.get(id) != Some(&Some(i)) {
            return None;
        }
        let (from, at) = self.occurrences.get_mut(id)?;
        let from = *from;
        if from == home || !self.kept.get(from).copied().unwrap_or_default() {
            return None;
        }
        first_free(self.live.get(from)?, at, taken, kind_of(e.obj)).map(|x| (from, x))
    }
}

/// Déplacements annoncés par (slide voulue, élément), et occurrences par (slide d'origine, élément).
type Announced<'w> = (HashMap<(&'w str, &'w str), usize>, HashMap<(usize, &'w str), VecDeque<usize>>);

/// Les déplacements annoncés de `moves` dont la slide d'origine est vivante, et leurs occurrences.
///
/// Un seul parcours de chaque slide d'origine.
fn announced<'w>(live: &[LiveSlide], moves: &'w [Moved]) -> Announced<'w> {
    let mut by_to = HashMap::new();
    let mut ids_by_from: HashMap<usize, HashSet<&'w str>> = HashMap::new();
    if moves.is_empty() {
        return (by_to, HashMap::new());
    }
    let live_at: HashMap<&str, usize> =
        live.iter().enumerate().filter(|(_, l)| l.key.1 == 0).map(|(s, l)| (l.key.0.as_str(), s)).collect();
    for m in moves {
        if let Some(&from) = live_at.get(m.from.as_str()) {
            by_to.insert((m.to.as_str(), m.element.as_str()), from);
            ids_by_from.entry(from).or_default().insert(m.element.as_str());
        }
    }
    let mut at: HashMap<(usize, &'w str), VecDeque<usize>> = HashMap::new();
    for (from, ids) in &ids_by_from {
        let Some(ls) = live.get(*from) else { continue };
        for (x, e) in ls.elements.iter().enumerate() {
            if let Some(id) = ids.get(e.key.0.as_str()) {
                at.entry((*from, *id)).or_default().push_back(x);
            }
        }
    }
    (by_to, at)
}

/// Retire de `at` les éléments déjà appariés, puis prend le premier s'il est de `kind`.
fn first_free(ls: &LiveSlide, at: &mut VecDeque<usize>, taken: &HashSet<TreeID>, kind: &str) -> Option<usize> {
    while at.front().and_then(|x| ls.elements.get(*x)).is_some_and(|l| taken.contains(&l.node)) {
        at.pop_front();
    }
    let x = at.front().copied()?;
    ls.elements.get(x).filter(|l| l.kind == kind)?;
    at.pop_front();
    Some(x)
}

/// Les éléments de chaque élément voulu (index par propriétaire).
fn owned_by(elements: &[Wanted<'_>]) -> Vec<Vec<usize>> {
    let mut out = vec![Vec::new(); elements.len()];
    for (j, e) in elements.iter().enumerate() {
        if let Some(list) = e.owner.and_then(|o| out.get_mut(o)) {
            list.push(j);
        }
    }
    out
}

/// La position de chaque élément dans le préordre `pre`.
fn preorder_positions(pre: &[usize]) -> Vec<usize> {
    let mut out = vec![0; pre.len()];
    for (p, i) in pre.iter().enumerate() {
        if let Some(o) = out.get_mut(*i) {
            *o = p;
        }
    }
    out
}

/// Les descendants vivants de l'élément `x`, en ordre `uorder`.
fn descendants(ls: &LiveSlide, positions: &[usize], x: usize) -> Vec<usize> {
    let Some(&p) = positions.get(x) else { return Vec::new() };
    let end = ls.elements.get(x).map_or(p, |l| l.end);
    let mut sub: Vec<usize> = ls.pre.get(p + 1..end).unwrap_or_default().to_vec();
    sub.sort_unstable();
    sub
}

/// Apparie les descendants de la racine déplacée `j` aux descendants vivants `live_sub` (R5, point 4).
///
/// Clés `(id, n)` calculées dans chaque sous-arbre, en ordre `uorder` ; un
/// nœud déjà apparié ne l'est pas deux fois.
fn pair_subtree(
    ls: &LiveSlide,
    live_sub: &[usize],
    (wanted, owned, j): (&[Wanted<'_>], &[Vec<usize>], usize),
    paired: &mut [Option<TreeID>],
    taken: &mut HashSet<TreeID>,
) {
    let mut wanted_sub = Vec::new();
    let mut stack: Vec<usize> = owned.get(j).cloned().unwrap_or_default();
    while let Some(d) = stack.pop() {
        wanted_sub.push(d);
        stack.extend(owned.get(d).into_iter().flatten());
    }
    wanted_sub.sort_unstable();
    let live_ids: Vec<String> =
        live_sub.iter().map(|i| ls.elements.get(*i).map(|l| l.key.0.clone()).unwrap_or_default()).collect();
    let live_keys = keyed(&live_ids);
    let by_key: HashMap<&Key, TreeID> =
        live_keys.iter().zip(live_sub).filter_map(|(k, i)| Some((k, ls.elements.get(*i)?.node))).collect();
    let wanted_ids: Vec<String> =
        wanted_sub.iter().map(|d| wanted.get(*d).map(|w| w.key.0.clone()).unwrap_or_default()).collect();
    let wanted_keys = keyed(&wanted_ids);
    for (key, d) in wanted_keys.iter().zip(&wanted_sub) {
        let Some(slot) = paired.get_mut(*d) else { continue };
        if slot.is_some() {
            continue;
        }
        if let Some(n) = by_key.get(key).filter(|n| !taken.contains(n)) {
            *slot = Some(*n);
            taken.insert(*n);
        }
    }
}

impl Writer<'_> {
    /// Crée les nœuds des slides et éléments voulus sans nœud apparié.
    ///
    /// Une slide naît en fin de racine, un élément en fin des enfants de son
    /// propriétaire (un propriétaire précède ses éléments) ; `pairing` reçoit
    /// les nœuds créés, que le placement traite comme existants. Avec `born`,
    /// ils naissent marqués, ensemble, dans un commit silencieux : défaire
    /// l'ajout remet la marque au lieu d'un `tree.delete` de Loro (t-776).
    /// Rend les nœuds créés, avec le slot voulu d'un élément (`None` : une slide) ;
    /// ils sont déjà bien rangés, le placement ne les déplace que s'ils ne le sont pas.
    fn create_missing<'w>(
        &self,
        tree: &LoroTree,
        wanted: &[WantedSlide<'w>],
        pairing: &mut Pairing,
    ) -> Result<Vec<(TreeID, Option<&'w str>)>, ReplicaInner> {
        let missing = pairing.slides.iter().flatten().count() < pairing.slides.len()
            || pairing.elements.iter().any(|e| e.iter().any(Option::is_none));
        if !missing {
            return Ok(Vec::new());
        }
        if self.born.is_some() {
            (self.split)();
        }
        let mut created = Vec::new();
        let mut make = |parent: TreeParentId, slot: Option<&'w str>| -> Result<TreeID, ReplicaInner> {
            let n = tree.create(parent).context(LoroSnafu)?;
            if self.born.is_some() {
                self.mark(tree, n)?;
            }
            created.push((n, slot));
            Ok(n)
        };
        for (i, ws) in wanted.iter().enumerate() {
            let slide = pairing.slides.get_mut(i).context(TreeOrderSnafu)?;
            let slide = match slide {
                Some(n) => *n,
                None => *slide.insert(make(TreeParentId::Root, None)?),
            };
            for (d, w) in ws.elements.iter().enumerate() {
                if pairing.elements.get(i).and_then(|e| e.get(d)).context(TreeOrderSnafu)?.is_some() {
                    continue;
                }
                let owner = match w.owner {
                    None => slide,
                    Some(o) => {
                        pairing.elements.get(i).and_then(|e| e.get(o)).copied().flatten().context(TreeOrderSnafu)?
                    }
                };
                let n = make(TreeParentId::Node(owner), Some(w.slot))?;
                *pairing.elements.get_mut(i).and_then(|e| e.get_mut(d)).context(TreeOrderSnafu)? = Some(n);
            }
        }
        if let Some(born) = self.born {
            born();
        }
        Ok(created)
    }

    /// Amène l'arbre du deck aux slides voulues (R6).
    ///
    /// Temps 0 : index et appariement, sans écriture. Temps 1 : marque des
    /// slides non appariées (et, dans une telle slide, des non voulus sous un
    /// élément qui en sort), puis des éléments non voulus de chaque slide
    /// gardée. Naissance groupée : quand il y a des nœuds à créer, ce qui
    /// précède forme une partie, puis un commit silencieux les crée tous.
    /// Temps 2 : les slides à la racine, puis, slide par slide, ses champs et
    /// ses éléments. Temps 3 : marque des différés. Le commit de l'appelant
    /// clôt l'étape.
    pub(in crate::replica) fn sync_deck(&self, tree: &LoroTree, wanted: Vec<WantedSlide<'_>>, moves: &[Moved]) -> Res {
        let live = live_slides(tree);
        let mut pairing = pair(&live, &wanted, moves);
        let paired: HashSet<TreeID> = pairing.elements.iter().flatten().flatten().copied().collect();
        let kept: HashSet<TreeID> = pairing.slides.iter().flatten().copied().collect();

        // Temps 1.
        let mut deferred = Vec::new();
        for ls in &live {
            if kept.contains(&ls.node) {
                deferred.extend(self.mark_elements(tree, &ls.elements, &ls.pre, &paired, true)?);
            } else {
                self.mark(tree, ls.node)?;
                if ls.elements.iter().any(|e| paired.contains(&e.node)) {
                    deferred.extend(self.mark_elements(tree, &ls.elements, &ls.pre, &paired, false)?);
                }
            }
        }

        // Naissance : tous les nœuds manquants, en un seul commit (t-776).
        let born = self.create_missing(tree, &wanted, &mut pairing)?;
        let created: HashSet<TreeID> = born.iter().filter(|(_, s)| s.is_none()).map(|(n, _)| *n).collect();
        let fresh: HashSet<TreeID> = born.iter().map(|(n, _)| *n).collect();

        // Temps 2 : les slides à la racine, avant tout élément.
        let shown: HashSet<TreeID> = live.iter().map(|l| l.node).chain(created).collect();
        let mut roots = tree.children(TreeParentId::Root).unwrap_or_default();
        let placed = arrange(tree, TreeParentId::Root, &mut roots, &pairing.slides, &|n| shown.contains(&n), &fresh)?;
        let slots: HashMap<TreeID, String> = live
            .iter()
            .flat_map(|l| l.elements.iter().map(|e| (e.node, e.slot.clone())))
            .chain(born.iter().filter_map(|(n, s)| Some((*n, (*s)?.to_owned()))))
            .collect();
        for (i, ws) in wanted.iter().enumerate() {
            let node = placed.get(i).copied().context(TreeOrderSnafu)?;
            let meta = tree.get_meta(node).context(LoroSnafu)?;
            set_value(&meta, SLIDE, LoroValue::Bool(true))?;
            self.sync_map(&meta, ws.obj, Ctx::Slide)?;
            let paired = pairing.elements.get(i).context(TreeOrderSnafu)?;
            let layout = Layout { wanted: &ws.elements, paired, slots: &slots, fresh: &fresh };
            self.place_elements(tree, node, &layout)?;
        }

        // Les nœuds nés marqués perdent leur marque dans l'étape.
        for (n, _) in born {
            self.unborn(&tree.get_meta(n).context(LoroSnafu)?)?;
        }

        // Temps 3 : un différé a un descendant voulu, donc aucun de ses
        // ancêtres n'a été marqué au temps 1 ; il n'est pas encore marqué.
        for n in deferred {
            self.mark(tree, n)?;
        }
        Ok(())
    }
}

#[cfg(test)]
thread_local! {
    static VISITED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
/// Compte une visite : méta lue, ou élément parcouru (tests de coût).
pub(super) fn visit() {
    VISITED.with(|v| v.set(v.get() + 1));
}

#[cfg(test)]
/// Visites depuis le dernier `reset_visited` (tests de coût) : métas lues, éléments parcourus par l'appariement.
pub(in crate::replica) fn visited() -> usize {
    VISITED.with(std::cell::Cell::get)
}

#[cfg(test)]
pub(in crate::replica) fn reset_visited() {
    VISITED.with(|v| v.set(0));
}

#[cfg(test)]
#[path = "deck_tree_tests.rs"]
mod deck_tree_tests;
