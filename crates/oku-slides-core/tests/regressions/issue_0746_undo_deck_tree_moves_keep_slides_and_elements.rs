//! t-746 : le deck est un seul `LoroTree`, slides en racine, sans la parade
//! t-263 (un `mov` par commit). Des `mov` de slides et d'éléments d'une slide
//! à l'autre dans un même commit, mêlés à des retraits et des ajouts, doivent
//! se défaire et se refaire exactement.

use oku_slides_core::{Deck, Element, Frame, Slide, SourceBundle, canonicalize};
use serde_json::json;
use speculoos::prelude::*;

/// Les slides de contenu et le slot de leurs éléments (cadrés dans `elements`).
const CONTENT: [(&str, &str); 3] = [("s-constat", "blocks"), ("s-priorites", "blocks"), ("s-libre", "elements")];

fn element(v: serde_json::Value) -> Element {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("élément : {e}"))
}

/// `roadmap` et une `pile` vide dans `blocks` de `s-constat`.
fn start() -> Deck {
    let mut deck =
        Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("fixture : {e}"));
    slot(&mut deck, CONTENT[0]).push(element(json!({ "id": "pile", "kind": "stack" })));
    canonicalize(&mut deck);
    deck
}

/// Les éléments du slot `(slide, name)` de `deck`.
fn slot<'d>(deck: &'d mut Deck, (slide, name): (&str, &str)) -> &'d mut Vec<Element> {
    let s: &mut Slide = deck.slide_mut(slide).unwrap_or_else(|| panic!("{slide} absente"));
    s.params.slot_mut(name).unwrap_or_else(|| panic!("slot {name} absent de {slide}"))
}

/// Range `e` en `at` du slot `place`, cadré seulement dans `elements`.
fn put(deck: &mut Deck, place: (&str, &str), at: usize, mut e: Element) {
    e.frame = (place.1 == "elements").then_some(Frame { x: 40.0, y: 40.0, w: 400.0, h: 200.0 });
    let items = slot(deck, place);
    let at = at.min(items.len());
    items.insert(at, e);
}

/// Générateur congruentiel : des permutations reproductibles sans dépendance.
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % n
    }
}

/// Les slides de contenu encore présentes dans `deck`.
fn content(deck: &Deck) -> Vec<(&'static str, &'static str)> {
    CONTENT.iter().copied().filter(|(id, _)| deck.slide(id).is_some()).collect()
}

/// Les positions `(slide, slot, rang)` des éléments de premier niveau, `pile` comprise ou non.
fn spots(deck: &mut Deck, with_pile: bool) -> Vec<((&'static str, &'static str), usize)> {
    let places = content(deck);
    let mut out = Vec::new();
    for place in places {
        for (i, e) in slot(deck, place).iter().enumerate() {
            if with_pile || e.id != "pile" {
                out.push((place, i));
            }
        }
    }
    out
}

/// Ce que fait une édition : le genre tiré, ou la permutation à défaut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edit {
    Permute,
    MoveBetweenSlides,
    RemoveSlide,
    RemoveThenCopy,
    AddOwner,
}

/// Une édition au hasard du deck ; rend le deck et le genre d'édition faite.
fn edited(deck: &Deck, rng: &mut Lcg, round: usize) -> (Deck, Edit) {
    let mut d = deck.clone();
    let kinds = [Edit::Permute, Edit::MoveBetweenSlides, Edit::RemoveSlide, Edit::RemoveThenCopy, Edit::AddOwner];
    let mut kind = kinds[rng.below(kinds.len())];
    let places = content(&d);
    let movable = spots(&mut d, true);
    let removable: Vec<String> =
        d.slides.iter().map(|s| s.id.clone()).filter(|id| id != "s-constat" && d.slides.len() > 2).collect();
    let copyable = spots(&mut d, false);
    kind = match kind {
        Edit::MoveBetweenSlides if places.len() < 2 || movable.is_empty() => Edit::Permute,
        Edit::RemoveSlide if removable.is_empty() => Edit::Permute,
        Edit::RemoveThenCopy if copyable.is_empty() => Edit::Permute,
        k => k,
    };
    match kind {
        Edit::Permute => {
            for k in (1..d.slides.len()).rev() {
                let j = rng.below(k + 1);
                d.slides.swap(k, j);
            }
        }
        Edit::MoveBetweenSlides => {
            let (from, i) = movable[rng.below(movable.len())];
            let others: Vec<_> = places.iter().copied().filter(|p| *p != from).collect();
            let to = others[rng.below(others.len())];
            let e = slot(&mut d, from).remove(i);
            let at = rng.below(slot(&mut d, to).len() + 1);
            put(&mut d, to, at, e);
        }
        Edit::RemoveSlide => {
            let gone = &removable[rng.below(removable.len())];
            d.slides.retain(|s| s.id != *gone);
        }
        Edit::RemoveThenCopy => {
            let (from, i) = copyable[rng.below(copyable.len())];
            let mut copy = slot(&mut d, from).remove(i);
            copy.id = format!("e-copie-{round}");
            let places = content(&d);
            let to = places[rng.below(places.len())];
            let at = rng.below(slot(&mut d, to).len() + 1);
            put(&mut d, to, at, copy);
        }
        Edit::AddOwner => {
            let text = |id: String| {
                element(json!({ "id": id, "kind": "text", "params": { "body": [{ "runs": [{ "t": "Enfant" }] }] } }))
            };
            let mut owner = element(json!({ "id": format!("pile-{round}"), "kind": "stack" }));
            owner.params.set(
                "children",
                oku_slides_core::ParamValue::Slot(vec![
                    text(format!("pile-{round}-a")),
                    text(format!("pile-{round}-b")),
                ]),
            );
            let to = places[rng.below(places.len())];
            let at = rng.below(slot(&mut d, to).len() + 1);
            put(&mut d, to, at, owner);
        }
    }
    canonicalize(&mut d);
    (d, kind)
}

/// Six commits par graine, de `per_commit` éditions chacun (un tour qui ne
/// change rien est sauté : il n'aurait pas d'étape), puis tout annuler et tout
/// rétablir ; chaque état relu égale l'état appliqué, sans `Issue`. Rend les
/// genres d'édition tirés.
fn walk(per_commit: usize) -> Vec<Edit> {
    let mut seen: Vec<Edit> = Vec::new();
    for seed in [1_u64, 7, 42, 2026] {
        let mut rng = Lcg(seed);
        let mut b = SourceBundle::new(start(), crate::common::shared()).unwrap_or_else(|e| panic!("bundle : {e}"));
        let mut states = vec![start()];
        let mut n = 0;
        while states.len() <= 6 {
            let last = states.last().unwrap_or_else(|| unreachable!()).clone();
            let mut next = last.clone();
            for _ in 0..per_commit {
                let (d, kind) = edited(&next, &mut rng, n);
                n += 1;
                seen.push(kind);
                next = d;
            }
            if next == last {
                continue;
            }
            let k = states.len();
            b.apply(next.clone(), &format!("Étape {k}"), None).unwrap_or_else(|e| panic!("apply : {e}"));
            assert_that!(b.replica().deck())
                .named(&format!("graine {seed}, étape {k}"))
                .is_equal_to((next.clone(), Vec::new()));
            states.push(next);
        }
        for (k, want) in states.iter().enumerate().rev().skip(1) {
            assert_that!(b.replica().undo_label()).is_equal_to(Some(format!("Étape {}", k + 1)));
            assert_that!(b.undo().unwrap_or_else(|e| panic!("undo : {e}"))).is_true();
            assert_that!(b.replica().deck())
                .named(&format!("graine {seed}, annuler vers {k}"))
                .is_equal_to((want.clone(), Vec::new()));
        }
        assert_that!(b.replica().can_undo()).is_false();
        for (k, want) in states.iter().enumerate().skip(1) {
            assert_that!(b.redo().unwrap_or_else(|e| panic!("redo : {e}"))).is_true();
            assert_that!(b.replica().deck())
                .named(&format!("graine {seed}, rétablir vers {k}"))
                .is_equal_to((want.clone(), Vec::new()));
        }
    }
    seen
}

/// Le hasard exerce bien chaque genre d'édition.
fn assert_every_kind(seen: &[Edit]) {
    for kind in [Edit::Permute, Edit::MoveBetweenSlides, Edit::RemoveSlide, Edit::RemoveThenCopy, Edit::AddOwner] {
        assert_that!(seen.contains(&kind)).named(&format!("{kind:?}")).is_true();
    }
}

#[test]
fn undo_and_redo_random_deck_tree_moves_walk_back_and_forth_exactly() {
    assert_every_kind(&walk(1));
}

/// Trois éditions par commit : plusieurs `mov` de slides et d'éléments d'une
/// slide à l'autre, mêlés à des marques et des créations, dans un même commit.
#[test]
fn undo_and_redo_three_random_edits_per_commit_walk_back_and_forth_exactly() {
    assert_every_kind(&walk(3));
}
