//! Slot and stack edits of the convergence proptest: element moves within, across and into slots.

use std::collections::HashMap;

use oku_slides_core::kinds::base::STACK;
use oku_slides_core::kinds::essentiel::CONTENT;
use oku_slides_core::{Deck, Element, Frame, Moved, ParamType, ParamValue, Params, Slide, apply_all, apply_all_moving};
use serde_json::json;

use super::{base, ok, op};

/// Le premier slot à deux éléments au moins (premier niveau) de la `i`-ième
/// slide qui en a un, modulo leur nombre.
pub(crate) fn busy_slot(deck: &mut Deck, i: usize) -> Option<&mut Vec<Element>> {
    let busy = |s: &Slide| s.params.slots().any(|(_, list)| list.len() >= 2);
    let count = deck.slides.iter().filter(|s| busy(s)).count();
    let slide = deck.slides.iter_mut().filter(|s| busy(s)).nth(i % count.max(1))?;
    slide.params.slots_mut().map(|(_, list)| list).find(|list| list.len() >= 2)
}

/// Le slot de flux et le slot libre du layout `content`.
pub(crate) const FLOW_SLOT: &str = "blocks";
pub(crate) const FREE_SLOT: &str = "elements";

/// Le cadre posé sur un élément qui entre dans le slot libre sans en avoir.
pub(crate) const DEFAULT_FRAME: Frame = Frame { x: 0.0, y: 0.0, w: 400.0, h: 200.0 };

/// La `i`-ième slide (modulo leur nombre) dont le layout déclare les slots
/// [`FLOW_SLOT`] et [`FREE_SLOT`] et dont l'un au moins est peuplé.
pub(crate) fn two_slots(deck: &mut Deck, i: usize) -> Option<&mut Slide> {
    let declares = |kind: &str, name: &str| {
        base().get(kind).and_then(|w| w.param(name)).is_some_and(|p| matches!(p.ty, ParamType::Slot { .. }))
    };
    let eligible = |s: &Slide| {
        declares(&s.kind, FLOW_SLOT)
            && declares(&s.kind, FREE_SLOT)
            && [FLOW_SLOT, FREE_SLOT].iter().any(|n| s.params.slot(n).is_some_and(|l| !l.is_empty()))
    };
    let count = deck.slides.iter().filter(|s| eligible(s)).count();
    deck.slides.iter_mut().filter(|s| eligible(s)).nth(i % count.max(1))
}

/// Retire l'élément `a` (modulo) d'un slot peuplé de `params` et l'ajoute à la
/// fin de l'autre slot : source [`FLOW_SLOT`] si `a` est pair, [`FREE_SLOT`]
/// sinon, l'autre si celui-là est vide. Le slot source vidé disparaît ;
/// l'élément reçoit un cadre dans le slot libre, perd cadre et cellule dans le flux.
pub(crate) fn move_across_slots(params: &mut Params, a: usize) {
    let filled = |n: &str| params.slot(n).is_some_and(|l| !l.is_empty());
    let (from, to) = match (a.is_multiple_of(2), filled(FLOW_SLOT), filled(FREE_SLOT)) {
        (true, true, _) | (false, true, false) => (FLOW_SLOT, FREE_SLOT),
        (_, _, true) => (FREE_SLOT, FLOW_SLOT),
        _ => return,
    };
    let Some(list) = params.slot_mut(from) else { return };
    let mut el = list.remove(a % list.len());
    if list.is_empty() {
        params.remove(from);
    }
    if to == FREE_SLOT {
        el.frame.get_or_insert(DEFAULT_FRAME);
    } else {
        (el.frame, el.cell) = (None, None);
    }
    if let Some(list) = params.slot_mut(to) {
        list.push(el);
    }
}

/// Le widget des piles et leur slot.
pub(crate) const PILE: &str = STACK;
pub(crate) const PILE_SLOT: &str = "children";

/// `deck` et une pile vide d'id `id` à la fin de `blocks` de `s-constat`.
pub(crate) fn with_pile(deck: Deck, id: &str) -> Deck {
    let add = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": FLOW_SLOT },
        "element": { "id": id, "kind": PILE } }));
    let mut d = deck;
    ok(apply_all(&mut d, vec![add], base()));
    d
}

/// Retire l'élément `id` de `params`, à toutes les profondeurs ; un slot vidé
/// disparaît.
pub(crate) fn detach(params: &mut Params, id: &str) -> Option<Element> {
    let mut emptied = None;
    let mut found = None;
    for (name, list) in params.slots_mut() {
        if let Some(at) = list.iter().position(|e| e.id == id) {
            found = Some(list.remove(at));
            if list.is_empty() {
                emptied = Some(name.to_owned());
            }
            break;
        }
        if let Some(el) = list.iter_mut().find_map(|e| detach(&mut e.params, id)) {
            found = Some(el);
            break;
        }
    }
    if let Some(name) = emptied {
        params.remove(&name);
    }
    found
}

/// Ajoute `el` au slot [`PILE_SLOT`] de `owner`, à la position `at` (modulo),
/// à toutes les profondeurs ; `false` si `owner` est absent.
pub(crate) fn attach(params: &mut Params, owner: &str, el: Element, at: usize) -> bool {
    for (_, list) in params.slots_mut() {
        for host in list.iter_mut() {
            if host.id == owner {
                let Some(children) = host.params.slot_mut(PILE_SLOT) else { return false };
                children.insert(at % (children.len() + 1), el);
                return true;
            }
            if contains(host, owner) {
                return attach(&mut host.params, owner, el, at);
            }
        }
    }
    false
}

/// Le sous-arbre de `el` contient-il l'élément `id` ?
pub(crate) fn contains(el: &Element, id: &str) -> bool {
    el.params.slots().any(|(_, list)| list.iter().any(|e| e.id == id || contains(e, id)))
}

/// `Edit::Nest` : l'élément `a` passe dans `children` de la pile `b`, sauf
/// s'il est cette pile ou l'un de ses ancêtres (cycle local).
pub(crate) fn nest(slide: &mut Slide, a: usize, b: usize, at: usize) {
    let (moved, owner) = {
        let all: Vec<&Element> = slide.walk().map(|l| l.element).collect();
        let piles: Vec<&str> = all.iter().filter(|e| e.kind == PILE).map(|e| e.id.as_str()).collect();
        if piles.is_empty() {
            return;
        }
        let (el, owner) = (all[a % all.len()], piles[b % piles.len()]);
        if el.id == owner || contains(el, owner) {
            return;
        }
        (el.id.clone(), owner.to_owned())
    };
    let Some(mut el) = detach(&mut slide.params, &moved) else { return };
    (el.frame, el.cell) = (None, None);
    let attached = attach(&mut slide.params, &owner, el, at);
    assert!(attached, "pile {owner} perdue");
}

/// `Edit::RemoveOwner` : retire la pile `i` (modulo) et ses enfants.
pub(crate) fn remove_owner(slide: &mut Slide, i: usize) {
    let piles: Vec<String> = slide.walk().filter(|l| l.element.kind == PILE).map(|l| l.element.id.clone()).collect();
    if let Some(id) = piles.get(i % piles.len().max(1)) {
        detach(&mut slide.params, id);
    }
}

/// Le premier slot déclaré par le layout `content`.
pub(crate) fn first_content_slot() -> &'static str {
    base()
        .get(CONTENT)
        .and_then(|w| w.params.iter().find(|p| matches!(p.ty, ParamType::Slot { .. })))
        .map_or_else(|| panic!("le layout content ne déclare aucun slot"), |p| p.name.as_str())
}

/// `Edit::MoveToSlide` par l'op `move_element` ; ses déplacements entre
/// slides, vides s'il n'y a qu'une slide `content` ou si l'op est refusée.
pub(crate) fn move_to_slide(deck: &mut Deck, a: usize, b: usize, at: usize) -> Vec<Moved> {
    let contents: Vec<&Slide> = deck.slides.iter().filter(|s| s.kind == CONTENT).collect();
    if contents.len() < 2 {
        return Vec::new();
    }
    let all: Vec<(&str, &str)> =
        contents.iter().flat_map(|s| s.walk().map(|l| (s.id.as_str(), l.element.id.as_str()))).collect();
    let Some(&(from, id)) = all.get(a % all.len().max(1)) else { return Vec::new() };
    let others: Vec<&Slide> = contents.iter().copied().filter(|s| s.id != from).collect();
    let to = others[b % others.len()];
    let slot = first_content_slot();
    let len = to.params.slot(slot).map_or(0, <[Element]>::len);
    let mv = op(json!({ "op": "move_element", "slide": from, "element": id,
        "to": { "owner": { "slide": to.id }, "param": slot }, "index": at % (len + 1) }));
    apply_all_moving(deck, vec![mv], base()).unwrap_or_default()
}

/// Les ids des slides et des éléments de `deck`, chacun avec « slide ou pile ».
pub(crate) fn ids_of(deck: &Deck) -> HashMap<String, bool> {
    let mut ids = HashMap::new();
    for s in &deck.slides {
        ids.insert(s.id.clone(), true);
        for l in s.walk() {
            ids.entry(l.element.id.clone()).or_insert(l.element.kind == PILE);
        }
    }
    ids
}

/// Le gras du premier run de l'élément `id`, à sa première occurrence.
pub(crate) fn bold_of(deck: &Deck, id: &str) -> Option<bool> {
    let el = deck.slides.iter().flat_map(Slide::walk).find(|l| l.element.id == id)?.element;
    match el.params.get("body") {
        Some(ParamValue::Rich(body)) => Some(body.first().and_then(|p| p.runs.first()).is_some_and(|r| r.bold)),
        _ => Some(false),
    }
}
