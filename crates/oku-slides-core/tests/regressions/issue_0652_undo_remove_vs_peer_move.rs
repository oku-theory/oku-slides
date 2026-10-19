//! t-652 : A supprime une slide (un élément, un item) puis annule pendant que
//! B la déplace ; après fusion, l'identifiant apparaissait deux fois.
//!
//! Loro défait une suppression en insérant un nouveau conteneur (copie) ; le
//! `mov` concurrent du pair garde l'original. La lecture écarte la copie
//! exacte et renomme une version divergente.

use std::sync::Arc;

use oku_slides_core::{
    Deck, DeckReplica, Element, ListItem, Op, ParamValue, SourceBundle, WidgetRegistry, apply_all, validate,
};
use serde_json::json;
use speculoos::prelude::*;

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("fixture : {e}"))
}

fn peers(deck: &Deck, registry: &Arc<WidgetRegistry>) -> (DeckReplica, DeckReplica) {
    let r = DeckReplica::new(deck, registry.clone()).unwrap_or_else(|e| panic!("replica : {e}"));
    let snap = r.snapshot().unwrap_or_else(|e| panic!("snapshot : {e}"));
    let open = || DeckReplica::from_snapshot(&snap, 1 << 24, registry.clone()).unwrap_or_else(|e| panic!("{e}")).0;
    (open(), open())
}

fn ids(deck: &Deck) -> Vec<String> {
    deck.slides.iter().map(|s| s.id.clone()).collect()
}

fn exchange(a: &mut DeckReplica, b: &mut DeckReplica) {
    a.merge(b).unwrap_or_else(|e| panic!("merge : {e}"));
    b.merge(a).unwrap_or_else(|e| panic!("merge : {e}"));
}

fn apply(r: &mut DeckReplica, deck: &Deck, label: &str) {
    r.apply(deck, label, None).unwrap_or_else(|e| panic!("apply : {e}"));
}

/// A applique `removed` et l'annule, B applique `moved` ; ils échangent. Puis
/// chacun réécrit le deck relu et ils échangent encore : la vue de chacun est
/// son replica, et les deux convergent. Le deck commun après la première fusion.
fn undo_vs_move(deck: &Deck, registry: &Arc<WidgetRegistry>, removed: &Deck, moved: &Deck) -> Deck {
    let (mut a, mut b) = peers(deck, registry);
    apply(&mut a, removed, "Supprimer");
    assert_that!(a.undo().unwrap_or_else(|e| panic!("undo : {e}"))).is_true();
    apply(&mut b, moved, "Déplacer");
    exchange(&mut a, &mut b);
    let (merged, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(b.deck().0).is_equal_to(&merged);
    // La prochaine écriture inscrit la réparation, des deux côtés.
    apply(&mut a, &merged, "A");
    assert_that!(a.deck().0).is_equal_to(&merged);
    apply(&mut b, &merged, "B");
    exchange(&mut a, &mut b);
    assert_that!(a.deck().0).is_equal_to(&merged);
    assert_that!(b.deck().0).is_equal_to(&merged);
    merged
}

fn unique(got: &[String]) -> bool {
    let mut u = got.to_vec();
    u.sort();
    u.dedup();
    u.len() == got.len()
}

#[test]
fn undo_remove_while_peer_moves_slide_keeps_ids_unique() {
    let deck = roadmap();
    let registry = crate::common::shared();
    for gone in 0..deck.slides.len() {
        for (from, to) in [(gone, 0), (gone, deck.slides.len() - 1), (0, deck.slides.len() - 1)] {
            let mut removed = deck.clone();
            removed.slides.remove(gone);
            let mut moved = deck.clone();
            let s = moved.slides.remove(from);
            moved.slides.insert(to, s);
            let got = ids(&undo_vs_move(&deck, &registry, &removed, &moved));
            assert_that!(unique(&got)).named(&format!("supprimer {gone}, déplacer {from}→{to} : {got:?}")).is_true();
        }
    }
}

/// Suppression douce (t-747) dans l'arbre du deck (t-746) : plus de `s-toc-2`.
#[test]
fn undo_remove_while_peer_moves_and_edits_slide_keeps_one_slide_with_the_peer_edit() {
    let deck = roadmap();
    let mut removed = deck.clone();
    removed.slides.remove(1);
    let mut moved = deck.clone();
    let mut s = moved.slides.remove(1);
    s.notes.push_str(" (B)");
    moved.slides.insert(0, s);
    let merged = undo_vs_move(&deck, &crate::common::shared(), &removed, &moved);
    assert_that!(ids(&merged)).is_equal_to(
        ["s-toc", "s-title", "s-ch1", "s-constat", "s-priorites", "s-ch2", "s-libre", "s-fin"]
            .map(str::to_owned)
            .to_vec(),
    );
    assert_that!(merged.slides[0].notes.ends_with(" (B)")).is_true();
}

/// Le slot `blocks` de `s-priorites` (`c1`, `c2`, `c3`).
fn blocks(deck: &mut Deck) -> &mut Vec<Element> {
    let s = deck.slides.iter_mut().find(|s| s.id == "s-priorites").unwrap_or_else(|| panic!("s-priorites"));
    match s.params.0.get_mut("blocks") {
        Some(ParamValue::Slot(b)) => b,
        _ => panic!("blocks absent"),
    }
}

fn block_ids(deck: &Deck) -> Vec<String> {
    blocks(&mut deck.clone()).iter().map(|e| e.id.clone()).collect()
}

#[test]
fn undo_remove_while_peer_moves_element_keeps_ids_unique() {
    let deck = roadmap();
    let mut removed = deck.clone();
    blocks(&mut removed).remove(1);
    let mut moved = deck.clone();
    let c2 = blocks(&mut moved).remove(1);
    blocks(&mut moved).insert(0, c2);
    let merged = undo_vs_move(&deck, &crate::common::shared(), &removed, &moved);
    assert_that!(block_ids(&merged)).is_equal_to(["c2", "c1", "c3"].map(str::to_owned).to_vec());
}

/// Suppression douce (t-747, ADR-0006) : annuler retire la marque, et le
/// même nœud revient avec le `mov` et l'édition de B.
#[test]
fn undo_remove_while_peer_moves_and_edits_element_keeps_the_peer_edit() {
    let deck = roadmap();
    let mut removed = deck.clone();
    blocks(&mut removed).remove(1);
    let mut moved = deck.clone();
    let mut c2 = blocks(&mut moved).remove(1);
    c2.params.set("kicker", ParamValue::Text("Pilier B".into()));
    blocks(&mut moved).insert(0, c2);
    // `undo_vs_move` vérifie déjà la convergence des deux pairs, sans `Issue`.
    let mut merged = undo_vs_move(&deck, &crate::common::shared(), &removed, &moved);
    assert_that!(block_ids(&merged)).is_equal_to(["c2", "c1", "c3"].map(str::to_owned).to_vec());
    let c2 = blocks(&mut merged).iter().find(|e| e.id == "c2").cloned().unwrap_or_else(|| panic!("c2 absent"));
    assert_that!(c2.params.get("kicker").cloned()).is_equal_to(Some(ParamValue::Text("Pilier B".into())));
}

/// `roadmap` et, dans les blocs de `s-constat`, l'élément `f` (`liste`) aux items `a`, `b`, `c`.
fn deck_with_list(registry: &WidgetRegistry) -> Deck {
    let mut deck = roadmap();
    let op = |v| serde_json::from_value::<Op>(v).unwrap_or_else(|e| panic!("op : {e}"));
    let ops = vec![
        op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
            "element": { "id": "f", "kind": "liste" } })),
        op(json!({ "op": "set_param", "target": { "slide": "s-constat", "element": "f" }, "param": "etapes",
            "value": [{ "id": "a", "date": "1" }, { "id": "b", "date": "2" }, { "id": "c", "date": "3" }] })),
    ];
    apply_all(&mut deck, ops, registry).unwrap_or_else(|e| panic!("ops : {e:?}"));
    deck
}

/// Les items de `etapes` de `f`.
fn etapes(deck: &mut Deck) -> &mut Vec<ListItem> {
    let s = deck.slides.iter_mut().find(|s| s.id == "s-constat").unwrap_or_else(|| panic!("s-constat"));
    let f = match s.params.0.get_mut("blocks") {
        Some(ParamValue::Slot(b)) => b.iter_mut().find(|e| e.id == "f").unwrap_or_else(|| panic!("f")),
        _ => panic!("blocks absent"),
    };
    match f.params.0.get_mut("etapes") {
        Some(ParamValue::List(l)) => l,
        _ => panic!("etapes absent"),
    }
}

fn item_ids(deck: &Deck) -> Vec<String> {
    etapes(&mut deck.clone()).iter().map(|i| i.id.clone()).collect()
}

/// A retire `b` de `etapes` et annule ; B déplace `b` en tête (`edit` le modifie).
fn list_scenario(edit: fn(&mut ListItem)) -> Vec<String> {
    let registry = Arc::new(crate::common::list_registry());
    let deck = deck_with_list(&registry);
    let mut removed = deck.clone();
    etapes(&mut removed).remove(1);
    let mut moved = deck.clone();
    let mut b = etapes(&mut moved).remove(1);
    edit(&mut b);
    etapes(&mut moved).insert(0, b);
    item_ids(&undo_vs_move(&deck, &registry, &removed, &moved))
}

#[test]
fn undo_remove_while_peer_moves_list_item_keeps_ids_unique() {
    assert_that!(list_scenario(|_| {})).is_equal_to(["b", "a", "c"].map(str::to_owned).to_vec());
}

#[test]
fn undo_remove_while_peer_moves_and_edits_list_item_renames_the_copy() {
    let got = list_scenario(|b| drop(b.values.set("date", ParamValue::Text("2 bis".into()))));
    assert_that!(got).is_equal_to(["b", "a", "b-2", "c"].map(str::to_owned).to_vec());
}

#[test]
fn undo_after_noop_apply_undoes_previous_edit() {
    let deck = roadmap();
    let registry = crate::common::shared();
    let mut edits: Vec<(&str, Deck)> = Vec::new();
    let mut removed = deck.clone();
    removed.slides.remove(1);
    edits.push(("supprimer", removed));
    let mut moved = deck.clone();
    moved.slides.swap(0, 5);
    edits.push(("déplacer", moved));
    let mut titled = deck.clone();
    titled.slides[2].params.set("title", ParamValue::Text("T".into()));
    edits.push(("titre", titled));
    for (what, edited) in edits {
        let (mut a, _) = peers(&deck, &registry);
        apply(&mut a, &edited, "Éditer");
        let steps = a.undo_steps();
        let relu = a.deck().0;
        apply(&mut a, &relu, "Rien");
        apply(&mut a, &edited, "Rien");
        assert_that!(a.undo_steps()).named(what).is_equal_to(steps);
        assert_that!(a.undo_label()).named(what).is_equal_to(Some("Éditer".to_owned()));
        assert_that!(a.undo().unwrap_or_else(|e| panic!("undo : {e}"))).is_true();
        assert_that!(a.deck().0).named(what).is_equal_to(&deck);
    }
}

/// `roadmap` aux identifiants en double, comme un fichier fait à la main :
/// slides (copie exacte, autre version), éléments d'un même slot (idem), d'un
/// slot imbriqué (`e-texte` repris dans un canevas), items d'une liste.
fn hand_made_duplicates(registry: &WidgetRegistry) -> Deck {
    let mut deck = deck_with_list(registry);
    let toc = deck.slides[1].clone();
    let mut other = toc.clone();
    other.notes = "autre".into();
    deck.slides.insert(2, toc);
    deck.slides.push(other);
    let c1 = blocks(&mut deck)[0].clone();
    let mut c1b = c1.clone();
    c1b.params.set("kicker", ParamValue::Text("Autre".into()));
    blocks(&mut deck).extend([c1, c1b]);
    let libre = deck.slides.iter_mut().find(|s| s.id == "s-libre").unwrap_or_else(|| panic!("s-libre"));
    let frame_box: Element = serde_json::from_value(json!({ "id": "cv", "kind": "test/frame-box",
        "frame": { "x": 0.0, "y": 0.0, "w": 10.0, "h": 10.0 },
        "params": { "children": [{ "id": "e-texte", "kind": "base/shape", "frame": { "x": 0.0, "y": 0.0, "w": 1.0, "h": 1.0 } }] } }))
    .unwrap_or_else(|e| panic!("canevas : {e}"));
    match libre.params.0.get_mut("elements") {
        Some(ParamValue::Slot(l)) => l.push(frame_box),
        _ => panic!("elements absent"),
    }
    let items = etapes(&mut deck);
    let mut b2 = items[1].clone();
    b2.values.set("date", ParamValue::Text("2 bis".into()));
    let b = items[1].clone();
    items.extend([b, b2]);
    deck
}

#[test]
fn bundle_of_deck_with_duplicate_ids_keeps_view_equal_to_replica() {
    let registry = Arc::new(crate::common::list_registry());
    let deck = hand_made_duplicates(&registry);
    let mut bundle = SourceBundle::new(deck, registry.clone()).unwrap_or_else(|e| panic!("bundle : {e}"));
    assert_that!(bundle.replica().deck().0).is_equal_to(bundle.deck());
    let view = bundle.deck().clone();
    assert_that!(validate(&view, &registry, None).iter().map(|i| i.message.id()).collect::<Vec<_>>())
        .does_not_contain("issue-duplicate-slide-id");
    let mut next = view.clone();
    next.slides[0].notes.push_str(" édité");
    bundle.apply(next.clone(), "Éditer", None).unwrap_or_else(|e| panic!("apply : {e}"));
    assert_that!(bundle.replica().deck().0).is_equal_to(&next);
    let found: Vec<&str> = validate(&next, &registry, None).iter().map(|i| i.message.id()).collect();
    for dup in ["issue-duplicate-slide-id", "issue-duplicate-element-id", "issue-duplicate-item-id"] {
        assert_that!(found).does_not_contain(dup);
    }
}
