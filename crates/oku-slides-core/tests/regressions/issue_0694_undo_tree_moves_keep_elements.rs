//! t-694 : les éléments d'une slide sont les nœuds d'un `LoroTree`, sans la
//! parade t-263 (un `mov` par commit), gardée pour les listes. Plusieurs `mov`
//! d'arbre dans un même commit (permutation, passage d'un slot à l'autre,
//! imbrication), mêlés à des suppressions et des ajouts, doivent se défaire
//! et se refaire exactement.

use oku_slides_core::{Deck, Element, Frame, SourceBundle, canonicalize};
use serde_json::json;
use speculoos::prelude::*;

/// Un slot de `s-constat` : propriétaire (`None` : la slide) et nom.
type Place = (Option<&'static str>, &'static str);

const PLACES: [Place; 3] = [(None, "blocks"), (None, "elements"), (Some("pile"), "children")];

/// `roadmap`, une `pile` vide dans `blocks` et un texte cadré `n` dans `elements` de `s-constat`.
fn start() -> Deck {
    let mut deck =
        Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("fixture : {e}"));
    let slide = constat(&mut deck);
    let pile: Element =
        serde_json::from_value(json!({ "id": "pile", "kind": "stack" })).unwrap_or_else(|e| panic!("pile : {e}"));
    let n: Element = serde_json::from_value(json!({ "id": "n", "kind": "text",
        "params": { "body": [{ "runs": [{ "t": "Note libre" }] }] } }))
    .unwrap_or_else(|e| panic!("n : {e}"));
    slot(slide, PLACES[0]).push(pile);
    put(slide, PLACES[1], 0, n);
    canonicalize(&mut deck);
    deck
}

fn constat(deck: &mut Deck) -> &mut oku_slides_core::Slide {
    deck.slide_mut("s-constat").unwrap_or_else(|| panic!("s-constat absente"))
}

/// Les éléments du slot `place` de `slide`.
fn slot(slide: &mut oku_slides_core::Slide, (owner, name): Place) -> &mut Vec<Element> {
    let params = match owner {
        None => &mut slide.params,
        Some(o) => {
            let blocks = slide.params.slot_mut("blocks").unwrap_or_else(|| panic!("blocks absent"));
            &mut blocks.iter_mut().find(|e| e.id == o).unwrap_or_else(|| panic!("{o} absent")).params
        }
    };
    params.slot_mut(name).unwrap_or_else(|| panic!("slot {name} absent"))
}

/// Range `e` en `at` du slot `place`, cadré seulement dans `elements`.
fn put(slide: &mut oku_slides_core::Slide, place: Place, at: usize, mut e: Element) {
    e.frame = (place == PLACES[1]).then_some(Frame { x: 40.0, y: 40.0, w: 400.0, h: 200.0 });
    slot(slide, place).insert(at, e);
}

/// Les positions `(slot, rang)` des éléments autres que `pile`.
fn movable(slide: &mut oku_slides_core::Slide) -> Vec<(Place, usize)> {
    PLACES
        .iter()
        .flat_map(|&p| {
            let ids: Vec<String> = slot(slide, p).iter().map(|e| e.id.clone()).collect();
            ids.into_iter().enumerate().filter(|(_, id)| id != "pile").map(move |(i, _)| (p, i))
        })
        .collect()
}

/// Générateur congruentiel : des permutations reproductibles sans dépendance.
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % n
    }
}

/// Une édition au hasard de `s-constat` : chaque slot permuté, un ou deux
/// éléments qui changent de slot (de la slide à l'autre, ou dans
/// `pile.children` et retour), parfois une suppression, puis une copie au
/// nouvel id `e-copie-{round}` : la slide grandit, les permutations aussi.
fn edited(deck: &Deck, rng: &mut Lcg, round: usize) -> Deck {
    let mut d = deck.clone();
    let slide = constat(&mut d);
    for place in PLACES {
        let items = slot(slide, place);
        for k in (1..items.len()).rev() {
            let j = rng.below(k + 1);
            items.swap(k, j);
        }
    }
    for _ in 0..1 + rng.below(2) {
        let spots = movable(slide);
        let (from, i) = spots[rng.below(spots.len())];
        let e = slot(slide, from).remove(i);
        let to = match (from, rng.below(2)) {
            (p, _) if p == PLACES[2] => PLACES[rng.below(2)],
            (p, 0) if p == PLACES[0] => PLACES[1],
            (_, 0) => PLACES[0],
            _ => PLACES[2],
        };
        let at = rng.below(slot(slide, to).len() + 1);
        put(slide, to, at, e);
    }
    let spots = movable(slide);
    if spots.len() > 1 && rng.below(2) == 0 {
        let (p, i) = spots[rng.below(spots.len())];
        slot(slide, p).remove(i);
    }
    let spots = movable(slide);
    let (p, i) = spots[rng.below(spots.len())];
    let mut copy = slot(slide, p)[i].clone();
    copy.id = format!("e-copie-{round}");
    let to = PLACES[rng.below(PLACES.len())];
    let at = rng.below(slot(slide, to).len() + 1);
    put(slide, to, at, copy);
    canonicalize(&mut d);
    d
}

#[test]
fn undo_and_redo_random_tree_moves_walk_back_and_forth_exactly() {
    let (mut nested, mut copied) = (0, 0);
    for seed in [1_u64, 7, 42, 2026] {
        let mut rng = Lcg(seed);
        let mut b = SourceBundle::new(start(), crate::common::shared()).unwrap_or_else(|e| panic!("bundle : {e}"));
        let mut states = vec![start()];
        for round in 0..6 {
            let next = edited(states.last().unwrap_or_else(|| unreachable!()), &mut rng, round);
            b.apply(next.clone(), &format!("Étape {round}"), None).unwrap_or_else(|e| panic!("apply : {e}"));
            assert_that!(b.replica().deck())
                .named(&format!("graine {seed}, étape {round}"))
                .is_equal_to((next.clone(), Vec::new()));
            let mut probe = next.clone();
            nested += usize::from(!slot(constat(&mut probe), PLACES[2]).is_empty());
            copied +=
                usize::from(probe.slide("s-constat").is_some_and(|s| s.locate(&format!("e-copie-{round}")).is_some()));
            states.push(next);
        }
        for (k, want) in states.iter().enumerate().rev().skip(1) {
            assert_that!(b.replica().undo_label()).is_equal_to(Some(format!("Étape {k}")));
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
    // Le hasard exerce bien l'imbrication et les copies.
    assert_that!(nested).is_greater_than(0);
    assert_that!(copied).is_greater_than(0);
}
