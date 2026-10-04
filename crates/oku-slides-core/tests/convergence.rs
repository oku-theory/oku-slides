//! Propriété : des pairs qui éditent en parallèle puis échangent tout convergent.

mod common;

use std::sync::Arc;

use oku_slides_core::{Deck, DeckReplica, Op, ParamValue, Params, Slide, apply_all, validate};
use proptest::prelude::*;
use serde_json::{Value, json};

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

/// Bascule le gras du premier run du premier `text` de `params` (slots
/// parcourus en profondeur) ; `true` si un run a été trouvé.
fn toggle_bold(params: &mut Params) -> bool {
    for (_, list) in params.slots_mut() {
        for el in list.iter_mut() {
            if el.kind == "text"
                && let Some(ParamValue::Rich(body)) = el.params.0.get_mut("body")
                && let Some(run) = body.first_mut().and_then(|p| p.runs.first_mut())
            {
                run.bold = !run.bold;
                return true;
            }
            if toggle_bold(&mut el.params) {
                return true;
            }
        }
    }
    false
}

fn apply(deck: &mut Deck, e: &Edit, peer: usize, n: &mut usize) {
    let len = deck.slides.len();
    match e {
        Edit::Add(i) => {
            *n += 1;
            let s = Slide::new(format!("p{peer}-{n}"), "content");
            deck.slides.insert(if len == 0 { 0 } else { i % (len + 1) }, s);
        }
        _ if len == 0 => {}
        Edit::Title(i, s) => {
            let contents: Vec<usize> = (0..len).filter(|&k| deck.slides[k].kind == "content").collect();
            if let Some(&k) = contents.get(i % contents.len().max(1)) {
                let params = &mut deck.slides[k].params;
                match s.trim() {
                    "" => params.remove("title"),
                    t => params.set("title", ParamValue::Text(t.to_owned())),
                };
            }
        }
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
            toggle_bold(&mut deck.slides[i % len].params);
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
        let snap = DeckReplica::new(&deck, common::shared()).unwrap().snapshot().unwrap();
        let mut peers: Vec<DeckReplica> =
            edits.iter().map(|_| DeckReplica::from_snapshot(&snap, 1 << 24, common::shared()).unwrap().0).collect();
        for (p, (replica, list)) in peers.iter_mut().zip(&edits).enumerate() {
            let (mut d, mut n) = (replica.deck().0, 0);
            for e in list {
                apply(&mut d, e, p, &mut n);
                replica.apply(&d, "edit", None).unwrap();
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
                let src = DeckReplica::from_snapshot(&peers[b].snapshot().unwrap(), 1 << 24, common::shared()).unwrap().0;
                peers[a].merge(&src).unwrap();
            }
        }
        let (first, issues) = peers[0].deck();
        prop_assert!(issues.is_empty(), "lecture partielle : {issues:?}");
        for p in &peers[1..] {
            prop_assert_eq!(&p.deck().0, &first);
        }
        prop_assert!(validate(&first, &common::registry(), None).iter().all(|i| i.message.id() != "issue-duplicate-slide-id"));
    }
}

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
}

/// Un changement de la liste `etapes` de l'élément `f` de `s-constat`.
fn list_edit(change: Value) -> Op {
    op(json!({ "op": "edit_list", "target": { "slide": "s-constat", "element": "f" },
        "param": "etapes", "change": change }))
}

fn with_ops(deck: &Deck, ops: Vec<Op>) -> Deck {
    let mut d = deck.clone();
    apply_all(&mut d, ops, &common::list_registry()).unwrap_or_else(|e| panic!("ops refusées : {e:?}"));
    d
}

/// La valeur d'un résultat attendu réussi.
fn ok<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e:?}"))
}

/// `roadmap` et, dans les blocs de `s-constat`, l'élément `f` (`liste`) aux
/// items `ids` : `date` 2019, 2020…
fn deck_with_list(ids: &[&str]) -> Deck {
    let deck = ok(Deck::from_json(include_str!("../../../fixtures/roadmap.json")));
    let items: Vec<Value> =
        ids.iter().enumerate().map(|(i, id)| json!({ "id": id, "date": (2019 + i).to_string() })).collect();
    let add = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "f", "kind": "liste" } }));
    let set = op(json!({ "op": "set_param", "target": { "slide": "s-constat", "element": "f" },
        "param": "etapes", "value": items }));
    with_ops(&deck, vec![add, set])
}

/// Deux pairs partis du même replica de `deck`.
fn list_peers(deck: &Deck) -> (DeckReplica, DeckReplica) {
    let registry = Arc::new(common::list_registry());
    let snap = ok(ok(DeckReplica::new(deck, registry.clone())).snapshot());
    let open = || ok(DeckReplica::from_snapshot(&snap, 1 << 24, registry.clone())).0;
    (open(), open())
}

/// A et B appliquent chacun leurs ops sur `deck`, puis échangent tout ; le
/// deck commun, identique sur les deux pairs.
fn exchange(deck: &Deck, ops_a: Vec<Op>, ops_b: Vec<Op>) -> Deck {
    let (mut a, mut b) = list_peers(deck);
    ok(a.apply(&with_ops(deck, ops_a), "A", None));
    ok(b.apply(&with_ops(deck, ops_b), "B", None));
    ok(a.merge(&b));
    ok(b.merge(&a));
    let (merged, issues) = a.deck();
    assert!(issues.is_empty(), "lecture partielle : {issues:?}");
    assert_eq!(b.deck().0, merged);
    merged
}

/// Les items de `etapes` : (id, date).
fn items(deck: &Deck) -> Vec<(String, Option<String>)> {
    let f = deck.slide("s-constat").and_then(|s| s.locate("f")).unwrap_or_else(|| panic!("élément f absent")).element;
    f.params
        .list("etapes")
        .unwrap_or_default()
        .iter()
        .map(|i| (i.id.clone(), i.values.text("date").map(str::to_owned)))
        .collect()
}

fn item(id: &str, date: &str) -> (String, Option<String>) {
    (id.to_owned(), Some(date.to_owned()))
}

#[test]
fn concurrent_list_inserts_are_both_kept() {
    let deck = deck_with_list(&["d0"]);
    let merged = exchange(
        &deck,
        vec![list_edit(json!({ "op": "insert", "id": "i-a", "values": { "date": "2030" } }))],
        vec![list_edit(json!({ "op": "insert", "id": "i-b", "values": { "date": "2031" } }))],
    );
    let got = items(&merged);
    assert_eq!(got.len(), 3, "{got:?}");
    assert_eq!(got[0], item("d0", "2019"));
    assert!(got.contains(&item("i-a", "2030")), "{got:?}");
    assert!(got.contains(&item("i-b", "2031")), "{got:?}");
}

#[test]
fn concurrent_move_and_set_field_merge() {
    let deck = deck_with_list(&["d0", "i-1", "i-2"]);
    let merged = exchange(
        &deck,
        vec![list_edit(json!({ "op": "move", "item": "d0", "to": 2 }))],
        vec![list_edit(json!({ "op": "set_field", "item": "d0", "field": "date", "value": "2018" }))],
    );
    assert_eq!(items(&merged), vec![item("i-1", "2020"), item("i-2", "2021"), item("d0", "2018")]);
}

#[test]
fn concurrent_typing_in_one_plain_cell_merges() {
    let deck = deck_with_list(&["d0", "i-1"]);
    let merged = exchange(
        &deck,
        vec![list_edit(json!({ "op": "set_field", "item": "d0", "field": "date", "value": "début 2019" }))],
        vec![list_edit(json!({ "op": "set_field", "item": "d0", "field": "date", "value": "2019 fin" }))],
    );
    assert_eq!(items(&merged), vec![item("d0", "début 2019 fin"), item("i-1", "2020")]);
}
