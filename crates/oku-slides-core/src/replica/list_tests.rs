//! Paramètres `list` dans le replica : carte `{list}`, items à `id` en valeur,
//! colonnes comme des paramètres.

use loro::{Container, LoroMap, LoroMovableList, LoroText, ValueOrContainer};
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::*;
use crate::{Deck, Element, Op, apply_all};

fn registry() -> Arc<WidgetRegistry> {
    crate::test_registry::with_list()
}

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap()
}

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
}

/// `ops` appliquées à une copie de `deck` (forme canonique).
fn with_ops(deck: &Deck, ops: Vec<Op>) -> Deck {
    let mut d = deck.clone();
    apply_all(&mut d, ops, &registry()).unwrap();
    d
}

fn edit(change: Value) -> Op {
    op(json!({ "op": "edit_list", "target": { "slide": "s-constat", "element": "f" },
        "param": "etapes", "change": change }))
}

/// `roadmap` et, dans les blocs de `s-constat`, l'élément `f` (`liste`) aux
/// items `ids` : `date` 2019, 2020…, `texte`, et pour le premier une `note`
/// riche et `fait`.
fn deck_with_list(ids: &[&str]) -> Deck {
    let items: Vec<Value> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let mut item = json!({ "id": id, "date": (2019 + i).to_string(), "texte": format!("Étape {i}") });
            if i == 0 {
                item["note"] = json!([{ "runs": [{ "t": "Bilan " }, { "t": "fort", "bold": true }] }]);
                item["fait"] = json!(true);
            }
            item
        })
        .collect();
    let add = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "f", "kind": "liste" } }));
    with_ops(&roadmap(), vec![add, set_list(Value::Array(items))])
}

/// `set_param` de la liste `etapes` de `f` entière.
fn set_list(value: Value) -> Op {
    op(json!({ "op": "set_param", "target": { "slide": "s-constat", "element": "f" },
        "param": "etapes", "value": value }))
}

/// Réconcilie `deck` dans un replica neuf, puis relit son snapshot.
fn round_trip(deck: &Deck) -> Deck {
    let r = DeckReplica::new(deck, registry()).unwrap();
    let (back, issues) = DeckReplica::from_snapshot(&r.snapshot().unwrap(), 1 << 20, registry()).unwrap().1;
    assert_that!(issues).is_empty();
    back
}

fn element<'a>(deck: &'a Deck, id: &str) -> &'a Element {
    deck.slide("s-constat").unwrap().locate(id).unwrap().element
}

fn ids(deck: &Deck) -> Vec<String> {
    element(deck, "f").params.list("etapes").map(|l| l.iter().map(|i| i.id.clone()).collect()).unwrap_or_default()
}

fn as_map(v: Option<ValueOrContainer>) -> LoroMap {
    match v {
        Some(ValueOrContainer::Container(Container::Map(m))) => m,
        other => panic!("carte attendue : {other:?}"),
    }
}

fn as_list(v: Option<ValueOrContainer>) -> LoroMovableList {
    match v {
        Some(ValueOrContainer::Container(Container::MovableList(l))) => l,
        other => panic!("liste attendue : {other:?}"),
    }
}

fn has_id(m: &LoroMap, id: &str) -> bool {
    matches!(m.get("id"), Some(ValueOrContainer::Value(v)) if v.as_string().is_some_and(|s| **s == id))
}

/// La `LoroMovableList` des items de `etapes`, dans la carte `{list}` de `f`.
fn items_list(r: &DeckReplica) -> LoroMovableList {
    let (tree, n) = tree::node_of(&r.doc, "s-constat", "f");
    let f = tree.get_meta(n).unwrap();
    let etapes = as_map(as_map(f.get("params")).get("etapes"));
    assert_that!(etapes.keys().map(|k| k.to_string()).collect::<Vec<_>>()).is_equal_to(vec![schema::LIST.to_owned()]);
    as_list(etapes.get(schema::LIST))
}

#[test]
fn reconcile_list_reads_back_items_and_text_columns() {
    let read = round_trip(&deck_with_list(&["d0", "i-1"]));
    let f = element(&read, "f");
    assert_that!(f.params.list("etapes").map(|l| l.len())).is_equal_to(Some(2));
    let items = f.params.list("etapes").unwrap();
    assert_that!(items[0].id.as_str()).is_equal_to("d0");
    assert_that!(items[0].values.text("date")).is_equal_to(Some("2019"));
    assert_that!(items[1].id.as_str()).is_equal_to("i-1");
    assert_that!(items[1].values.text("texte")).is_equal_to(Some("Étape 1"));
}

#[test]
fn round_trip_list_with_plain_rich_and_flag_columns_is_identical() {
    let deck = deck_with_list(&["d0", "i-1", "i-2"]);
    let back = round_trip(&deck);
    assert_that!(back).is_equal_to(&deck);
    let first = &element(&back, "f").params.list("etapes").unwrap()[0];
    assert_that!(first.values.get("fait")).is_equal_to(Some(&crate::ParamValue::Value(json!(true))));
    assert_that!(first.values.rich("note").map(|p| p[0].runs.len())).is_equal_to(Some(2));
}

#[test]
fn apply_list_edits_then_deck_equals_the_canonical_view() {
    let deck = deck_with_list(&["d0", "i-1"]);
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let edited = with_ops(
        &deck,
        vec![
            edit(json!({ "op": "insert", "index": 0, "id": "i-2", "values": { "date": "2030", "fait": true } })),
            edit(json!({ "op": "set_field", "item": "d0", "field": "note", "value": [{ "runs": [{ "t": "Revu" }] }] })),
            edit(json!({ "op": "set_field", "item": "d0", "field": "fait", "value": null })),
            edit(json!({ "op": "move", "item": "i-1", "to": 0 })),
        ],
    );
    r.apply(&edited, "Éditer", None).unwrap();
    let (read, issues) = r.deck();
    assert_that!(issues).is_empty();
    assert_that!(read).is_equal_to(&edited);
    assert_that!(ids(&read)).is_equal_to(vec!["i-1".to_owned(), "i-2".to_owned(), "d0".to_owned()]);
}

#[test]
fn reconcile_list_plain_column_is_a_loro_text() {
    let r = DeckReplica::new(&deck_with_list(&["d0", "i-1"]), registry()).unwrap();
    let list = items_list(&r);
    assert_that!(list.len()).is_equal_to(2);
    let d0 = as_map(list.get(0));
    // L'id est une valeur (clé de réconciliation), jamais un texte.
    assert_that!(has_id(&d0, "d0")).is_true();
    let date: Option<LoroText> = match d0.get("date") {
        Some(ValueOrContainer::Container(Container::Text(t))) => Some(t),
        _ => None,
    };
    assert_that!(date.map(|t| t.to_string())).is_equal_to(Some("2019".to_owned()));
    // La colonne `rich` est une carte `{rich}`, comme un paramètre.
    assert_that!(as_map(d0.get("note")).get(schema::RICH).is_some()).is_true();
}

#[test]
fn reconcile_absent_declared_list_creates_its_empty_container() {
    let r = DeckReplica::new(&deck_with_list(&[]), registry()).unwrap();
    assert_that!(items_list(&r).len()).is_equal_to(0);
    assert_that!(element(&r.deck().0, "f").params.get("etapes")).is_none();
}

#[test]
fn reconcile_emptied_list_reads_back_absent() {
    let deck = deck_with_list(&["d0", "i-1"]);
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let emptied = with_ops(&deck, vec![set_list(Value::Null)]);
    r.apply(&emptied, "Vider", None).unwrap();
    let (read, issues) = r.deck();
    assert_that!(issues).is_empty();
    assert_that!(element(&read, "f").params.get("etapes")).is_none();
    assert_that!(read).is_equal_to(&emptied);
    // Le conteneur reste, vidé : une insertion concurrente le vise encore.
    assert_that!(items_list(&r).len()).is_equal_to(0);
}

#[test]
fn undo_list_insert_move_remove_each_restores_the_previous_list() {
    let d0 = deck_with_list(&["d0", "i-1"]);
    let d1 = with_ops(&d0, vec![edit(json!({ "op": "insert", "id": "i-2", "values": { "date": "2031" } }))]);
    let d2 = with_ops(&d1, vec![edit(json!({ "op": "move", "item": "i-2", "to": 0 }))]);
    let d3 = with_ops(&d2, vec![edit(json!({ "op": "remove", "item": "d0" }))]);
    let mut r = DeckReplica::new(&d0, registry()).unwrap();
    for (d, label) in [(&d1, "Ajouter"), (&d2, "Déplacer"), (&d3, "Retirer")] {
        r.apply(d, label, None).unwrap();
    }
    assert_that!(ids(&r.deck().0)).is_equal_to(vec!["i-2".to_owned(), "i-1".to_owned()]);
    for (before, expected) in
        [(&d2, ["i-2", "d0", "i-1"].as_slice()), (&d1, &["d0", "i-1", "i-2"]), (&d0, &["d0", "i-1"])]
    {
        assert_that!(r.undo().unwrap()).is_true();
        let (read, issues) = r.deck();
        assert_that!(issues).is_empty();
        assert_that!(ids(&read)).is_equal_to(expected.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>());
        assert_that!(&read).is_equal_to(before);
    }
    // Le retrait défait rend l'item avec ses valeurs.
    let read = r.deck().0;
    let restored = &element(&read, "f").params.list("etapes").unwrap()[0];
    assert_that!(restored.values.text("date")).is_equal_to(Some("2019"));
}

#[test]
fn concurrent_first_inserts_into_an_absent_list_keep_both() {
    let deck = deck_with_list(&[]);
    let snap = DeckReplica::new(&deck, registry()).unwrap().snapshot().unwrap();
    let open = || DeckReplica::from_snapshot(&snap, 1 << 20, registry()).unwrap().0;
    let (mut a, mut b) = (open(), open());
    a.apply(&with_ops(&deck, vec![edit(json!({ "op": "insert", "id": "i-a" }))]), "A", None).unwrap();
    b.apply(&with_ops(&deck, vec![edit(json!({ "op": "insert", "id": "i-b" }))]), "B", None).unwrap();
    a.merge(&b).unwrap();
    let mut merged = ids(&a.deck().0);
    merged.sort();
    assert_that!(merged).is_equal_to(vec!["i-a".to_owned(), "i-b".to_owned()]);
}

#[test]
fn read_list_item_without_id_is_set_aside_and_reported() {
    let r = DeckReplica::new(&deck_with_list(&["d0"]), registry()).unwrap();
    let forged = items_list(&r).insert_container(1, LoroMap::new()).unwrap();
    forged.insert_container("date", LoroText::new()).unwrap().insert(0, "2040").unwrap();
    r.doc.commit();
    let (read, issues) = r.deck();
    assert_that!(ids(&read)).is_equal_to(vec!["d0".to_owned()]);
    assert_that!(issues.iter().map(|i| i.message.id()).collect::<Vec<_>>()).is_equal_to(vec!["issue-unreadable-item"]);
}

/// Messages des changes Loro, du plus ancien au plus récent.
fn messages(r: &DeckReplica) -> Vec<String> {
    let heads: Vec<loro::ID> = r.doc.oplog_frontiers().iter().collect();
    let mut all = Vec::new();
    let _ = r.doc.travel_change_ancestors(&heads, &mut |m| {
        all.push((m.lamport, m.message.map(|s| s.to_string()).unwrap_or_default()));
        std::ops::ControlFlow::Continue(())
    });
    all.sort();
    all.into_iter().map(|(_, m)| m).collect()
}

#[test]
fn apply_reorder_of_items_marks_intermediate_commits_as_parts() {
    let deck = deck_with_list(&["a", "b", "c", "d"]);
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let before = messages(&r).len();
    let moves = [("d", 0), ("c", 1), ("b", 2)];
    let reversed =
        with_ops(&deck, moves.iter().map(|(item, to)| edit(json!({ "op": "move", "item": item, "to": to }))).collect());
    assert_that!(ids(&reversed)).is_equal_to(vec!["d".to_owned(), "c".into(), "b".into(), "a".into()]);
    r.apply(&reversed, "Inverser", None).unwrap();
    let got = messages(&r).split_off(before);
    // Trois `mov` d'items, chacun précédé d'un commit (parade t-263) : le
    // premier ne committe rien, puis deux commits intermédiaires et le dernier.
    assert_that!(got).is_equal_to(vec![format!("{PART}Inverser"), format!("{PART}Inverser"), "Inverser".to_owned()]);
    assert_that!(ids(&r.deck().0)).is_equal_to(ids(&reversed));
}

// Listes imbriquées (t-495) : le widget `plan`, étapes à tâches.

/// `roadmap` et l'élément `p` (`plan`) : étape `a` aux tâches `t1`, `t2`, étape `b` sans tâche.
fn deck_with_plan() -> Deck {
    let add = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "p", "kind": "plan" } }));
    let set = op(json!({ "op": "set_param", "target": { "slide": "s-constat", "element": "p" }, "param": "etapes",
        "value": [
            { "id": "a", "titre": "A", "taches": [{ "id": "t1", "texte": "un", "fait": true }, { "id": "t2", "texte": "deux" }] },
            { "id": "b", "titre": "B" }
        ] }));
    with_ops(&roadmap(), vec![add, set])
}

fn edit_tasks(step: &str, change: Value) -> Op {
    op(json!({ "op": "edit_list", "target": { "slide": "s-constat", "element": "p" },
        "param": format!("etapes/{step}/taches"), "change": change }))
}

fn task_ids(deck: &Deck, step: &str) -> Vec<String> {
    let steps = element(deck, "p").params.list("etapes").unwrap_or_default();
    let it = steps.iter().find(|i| i.id == step).unwrap_or_else(|| panic!("étape {step} absente"));
    it.values.list("taches").unwrap_or_default().iter().map(|t| t.id.clone()).collect()
}

#[test]
fn round_trip_nested_list_is_identical() {
    let deck = deck_with_plan();
    let back = round_trip(&deck);
    assert_that!(back).is_equal_to(&deck);
    assert_that!(task_ids(&back, "a")).is_equal_to(vec!["t1".to_owned(), "t2".into()]);
}

#[test]
fn nested_list_is_a_movable_list_in_its_item_map() {
    let r = DeckReplica::new(&deck_with_plan(), registry()).unwrap();
    let (tree, n) = tree::node_of(&r.doc, "s-constat", "p");
    let p = tree.get_meta(n).unwrap();
    let steps = as_list(as_map(as_map(p.get("params")).get("etapes")).get(schema::LIST));
    let a = as_map(steps.get(0));
    let tasks = as_list(as_map(a.get("taches")).get(schema::LIST));
    assert_that!(tasks.len()).is_equal_to(2);
    // Étape `b` sans tâche : la liste imbriquée est créée d'avance, vide (parade t-188).
    let b = as_map(steps.get(1));
    assert_that!(as_list(as_map(b.get("taches")).get(schema::LIST)).len()).is_equal_to(0);
}

#[test]
fn undo_nested_insert_move_remove_and_set_field_each_restores_the_previous_deck() {
    let d0 = deck_with_plan();
    let d1 = with_ops(&d0, vec![edit_tasks("a", json!({ "op": "insert", "id": "n", "values": { "texte": "neuf" } }))]);
    let d2 = with_ops(&d1, vec![edit_tasks("a", json!({ "op": "move", "item": "n", "to": 0 }))]);
    let d3 = with_ops(&d2, vec![edit_tasks("a", json!({ "op": "remove", "item": "t1" }))]);
    let d4 = with_ops(
        &d3,
        vec![edit_tasks("a", json!({ "op": "set_field", "item": "t2", "field": "texte", "value": "DEUX" }))],
    );
    let mut r = DeckReplica::new(&d0, registry()).unwrap();
    for (d, label) in [(&d1, "Ajouter"), (&d2, "Déplacer"), (&d3, "Retirer"), (&d4, "Écrire")] {
        r.apply(d, label, None).unwrap();
        assert_that!(&r.deck().0).is_equal_to(d);
    }
    for before in [&d3, &d2, &d1, &d0] {
        assert_that!(r.undo().unwrap()).is_true();
        let (read, issues) = r.deck();
        assert_that!(issues).is_empty();
        assert_that!(&read).is_equal_to(before);
    }
}

#[test]
fn concurrent_inserts_into_the_same_nested_list_keep_both() {
    let deck = deck_with_plan();
    let snap = DeckReplica::new(&deck, registry()).unwrap().snapshot().unwrap();
    let open = || DeckReplica::from_snapshot(&snap, 1 << 20, registry()).unwrap().0;
    let (mut a, mut b) = (open(), open());
    a.apply(&with_ops(&deck, vec![edit_tasks("b", json!({ "op": "insert", "id": "x" }))]), "A", None).unwrap();
    b.apply(&with_ops(&deck, vec![edit_tasks("b", json!({ "op": "insert", "id": "y" }))]), "B", None).unwrap();
    a.merge(&b).unwrap();
    let mut merged = task_ids(&a.deck().0, "b");
    merged.sort();
    assert_that!(merged).is_equal_to(vec!["x".to_owned(), "y".into()]);
}

// Suites de revue (t-495).

#[test]
fn undeclared_item_column_holding_items_stays_an_atom() {
    // `liste` ne déclare pas `extra` : une valeur en forme d'items y reste une valeur LWW.
    let deck = deck_with_list(&["d0"]);
    let mut d = deck.clone();
    let slide = d.slides.iter_mut().find(|s| s.id == "s-constat").unwrap();
    let el = slide.params.slot_mut("blocks").unwrap().iter_mut().find(|e| e.id == "f").unwrap();
    let mut items = el.params.list("etapes").unwrap().to_vec();
    items[0].values.set("extra", crate::ParamValue::List(serde_json::from_value(json!([{ "id": "x" }])).unwrap()));
    el.params.set("etapes", crate::ParamValue::List(items));
    let r = DeckReplica::new(&d, registry()).unwrap();
    let d0 = as_map(items_list(&r).get(0));
    assert_that!(matches!(d0.get("extra"), Some(ValueOrContainer::Value(_)))).is_true();
    assert_that!(&r.deck().0).is_equal_to(&d);
}

#[test]
fn undo_redo_of_removing_a_step_restores_its_nested_tasks_exactly() {
    let d0 = deck_with_plan();
    let edit_steps = |change: Value| {
        op(
            json!({ "op": "edit_list", "target": { "slide": "s-constat", "element": "p" }, "param": "etapes", "change": change }),
        )
    };
    let d1 = with_ops(&d0, vec![edit_steps(json!({ "op": "remove", "item": "a" }))]);
    let mut r = DeckReplica::new(&d0, registry()).unwrap();
    r.apply(&d1, "Retirer", None).unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(&r.deck().0).is_equal_to(&d0);
    assert_that!(r.redo().unwrap()).is_true();
    assert_that!(&r.deck().0).is_equal_to(&d1);
    assert_that!(r.undo().unwrap()).is_true();
    let (read, issues) = r.deck();
    assert_that!(issues).is_empty();
    assert_that!(&read).is_equal_to(&d0);
}

#[rstest::rstest]
#[case::removed(json!({ "op": "remove", "item": "a" }))]
#[case::moved(json!({ "op": "move", "item": "a", "to": 1 }))]
fn concurrent_nested_insert_and_outer_change_converge(#[case] outer: Value) {
    let deck = deck_with_plan();
    let snap = DeckReplica::new(&deck, registry()).unwrap().snapshot().unwrap();
    let open = || DeckReplica::from_snapshot(&snap, 1 << 20, registry()).unwrap().0;
    let (mut a, mut b) = (open(), open());
    a.apply(&with_ops(&deck, vec![edit_tasks("a", json!({ "op": "insert", "id": "n" }))]), "A", None).unwrap();
    let outer = op(json!({ "op": "edit_list", "target": { "slide": "s-constat", "element": "p" }, "param": "etapes",
        "change": outer }));
    b.apply(&with_ops(&deck, vec![outer]), "B", None).unwrap();
    a.merge(&b).unwrap();
    b.merge(&a).unwrap();
    let (da, ia) = a.deck();
    let (db, _) = b.deck();
    assert_that!(&da).is_equal_to(&db);
    assert_that!(ia.iter().any(|i| i.message.id() == "issue-unreadable-item")).is_false();
}
