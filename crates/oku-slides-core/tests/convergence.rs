//! Propriété : des pairs qui éditent en parallèle puis échangent tout convergent.

use oku_slides_core::{Deck, DeckReplica, ElementBody, Slide, SlideKind, validate};
use proptest::prelude::*;

#[derive(Debug, Clone)]
enum Edit {
    Title(usize, String),
    Notes(usize, String),
    Move(usize, usize),
    Delete(usize),
    Add(usize),
    Theme(Option<String>),
    Bold(usize),
}

fn edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        (any::<usize>(), "[a-zé ]{0,10}").prop_map(|(i, s)| Edit::Title(i, s)),
        (any::<usize>(), "[a-z ]{0,10}").prop_map(|(i, s)| Edit::Notes(i, s)),
        (any::<usize>(), any::<usize>()).prop_map(|(a, b)| Edit::Move(a, b)),
        any::<usize>().prop_map(Edit::Delete),
        any::<usize>().prop_map(Edit::Add),
        prop::option::of(prop_oneof![Just("papier".to_owned()), Just("pro".to_owned())]).prop_map(Edit::Theme),
        any::<usize>().prop_map(Edit::Bold),
    ]
}

fn apply(deck: &mut Deck, e: &Edit, peer: usize, n: &mut usize) {
    let len = deck.slides.len();
    match e {
        Edit::Add(i) => {
            *n += 1;
            let s = Slide::new(format!("p{peer}-{n}"), SlideKind::Content);
            deck.slides.insert(if len == 0 { 0 } else { i % (len + 1) }, s);
        }
        _ if len == 0 => {}
        Edit::Title(i, s) => deck.slides[i % len].title = (!s.trim().is_empty()).then(|| s.clone()),
        Edit::Notes(i, s) => deck.slides[i % len].notes.push_str(s),
        Edit::Move(a, b) => {
            let s = deck.slides.remove(a % len);
            deck.slides.insert(b % len, s);
        }
        Edit::Delete(i) => {
            deck.slides.remove(i % len);
        }
        Edit::Theme(t) => deck.theme = t.clone(),
        Edit::Bold(i) => {
            let slide = &mut deck.slides[i % len];
            for el in slide.elements.iter_mut().chain(slide.blocks.iter_mut()) {
                if let ElementBody::Text { text } = &mut el.body
                    && let Some(run) = text.first_mut().and_then(|p| p.runs.first_mut())
                {
                    run.bold = !run.bold;
                    return;
                }
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn peers_editing_in_parallel_converge(
        edits in prop::collection::vec(prop::collection::vec(edit(), 0..10), 2..4),
        seed in any::<u64>(),
    ) {
        let deck = Deck::from_json(include_str!("../../../fixtures/roadmap.json")).unwrap();
        let snap = DeckReplica::new(&deck).unwrap().snapshot().unwrap();
        let mut peers: Vec<DeckReplica> =
            edits.iter().map(|_| DeckReplica::from_snapshot(&snap, 1 << 24).unwrap().0).collect();
        for (p, (replica, list)) in peers.iter_mut().zip(&edits).enumerate() {
            let (mut d, mut n) = (replica.deck().0, 0);
            for e in list {
                apply(&mut d, e, p, &mut n);
                replica.apply(&d, "edit").unwrap();
            }
        }
        // Échanges dans un ordre pseudo-aléatoire, jusqu'à ce que chacun ait tout.
        let k = peers.len();
        let mut order: Vec<(usize, usize)> = (0..k).flat_map(|a| (0..k).map(move |b| (a, b))).filter(|(a, b)| a != b).collect();
        let mut x = seed | 1;
        for i in (1..order.len()).rev() {
            x ^= x << 13; x ^= x >> 7; x ^= x << 17;
            order.swap(i, (x % (i as u64 + 1)) as usize);
        }
        for _ in 0..2 {
            for &(a, b) in &order {
                let src = DeckReplica::from_snapshot(&peers[b].snapshot().unwrap(), 1 << 24).unwrap().0;
                peers[a].merge(&src).unwrap();
            }
        }
        let (first, issues) = peers[0].deck();
        prop_assert!(issues.is_empty(), "lecture partielle : {issues:?}");
        for p in &peers[1..] {
            prop_assert_eq!(&p.deck().0, &first);
        }
        prop_assert!(validate(&first, None).iter().all(|i| i.message.id() != "issue-duplicate-slide-id"));
    }
}
