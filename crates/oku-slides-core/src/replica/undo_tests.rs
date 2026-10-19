use speculoos::prelude::*;

use crate::{Deck, DeckReplica, Slide};

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap()
}

fn titled(deck: &Deck, t: &str) -> Deck {
    let mut d = deck.clone();
    d.slides[2].params.set("title", crate::ParamValue::Text(t.into()));
    d
}

#[test]
fn undo_restores_previous_deck_and_moves_label() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.apply(&titled(&deck, "Un"), "Renommer", None).unwrap();
    assert_that!(r.undo_label()).is_equal_to(Some("Renommer".to_owned()));
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&deck);
    assert_that!(r.redo_label()).is_equal_to(Some("Renommer".to_owned()));
    assert_that!(r.redo().unwrap()).is_true();
    assert_that!(r.deck().0.slides[2].params.text("title")).is_equal_to(Some("Un"));
}

#[test]
fn two_immediate_commits_are_two_undo_steps() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.apply(&titled(&deck, "Un"), "Un", None).unwrap();
    r.apply(&titled(&deck, "Deux"), "Deux", None).unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0.slides[2].params.text("title")).is_equal_to(Some("Un"));
    assert_that!(r.undo_label()).is_equal_to(Some("Un".to_owned()));
}

#[test]
fn new_replica_has_nothing_to_undo() {
    let r = DeckReplica::new(&roadmap(), crate::test_registry::shared()).unwrap();
    assert_that!(r.can_undo()).is_false();
}

#[test]
fn apply_silent_is_not_undoable() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.apply_silent(&titled(&deck, "Canonique")).unwrap();
    assert_that!(r.can_undo()).is_false();
}

#[test]
fn empty_apply_does_not_leak_its_label() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.apply(&deck, "Vide", None).unwrap();
    r.apply(&titled(&deck, "Deux"), "Deux", None).unwrap();
    assert_that!(r.undo_label()).is_equal_to(Some("Deux".to_owned()));
}

#[test]
fn undo_after_merge_keeps_remote_change() {
    let deck = roadmap();
    let base = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let snap = base.snapshot().unwrap();
    let mut a = DeckReplica::from_snapshot(&snap, 1 << 20, crate::test_registry::shared()).unwrap().0;
    let mut b = DeckReplica::from_snapshot(&snap, 1 << 20, crate::test_registry::shared()).unwrap().0;
    a.apply(&titled(&deck, "de A"), "A", None).unwrap();
    let mut db = deck.clone();
    db.slides[3].params.set("title", crate::ParamValue::Text("de B".into()));
    b.apply(&db, "B", None).unwrap();
    a.merge(&b).unwrap();
    a.undo().unwrap();
    let got = a.deck().0;
    assert_that!(got.slides[2].params.get("title")).is_equal_to(deck.slides[2].params.get("title"));
    assert_that!(got.slides[3].params.text("title")).is_equal_to(Some("de B"));
}

#[test]
fn undo_limit_drops_oldest_steps() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.set_undo_limit(2);
    for t in ["1", "2", "3"] {
        r.apply(&titled(&deck, t), t, None).unwrap();
    }
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.undo().unwrap()).is_false();
}

fn themed(deck: &Deck, t: &str) -> Deck {
    let mut d = deck.clone();
    d.theme = Some(crate::test_registry::theme_ref(t));
    d
}

#[test]
fn group_merges_commits_into_one_undo_step_with_the_group_label() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.begin_group("Agent : titre plus percutant");
    r.apply(&titled(&deck, "Un"), "ignoré", None).unwrap();
    r.apply(&themed(&titled(&deck, "Un"), "b"), "ignoré aussi", None).unwrap();
    r.end_group();
    assert_that!(r.undo_label()).is_equal_to(Some("Agent : titre plus percutant".to_owned()));
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&deck);
    assert_that!(r.can_undo()).is_false();
}

#[test]
fn group_redo_replays_every_commit() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let after = themed(&titled(&deck, "Un"), "b");
    r.begin_group("Tour");
    r.apply(&titled(&deck, "Un"), "a", None).unwrap();
    r.apply(&after, "b", None).unwrap();
    r.end_group();
    r.undo().unwrap();
    assert_that!(r.redo_label()).is_equal_to(Some("Tour".to_owned()));
    assert_that!(r.redo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&after);
}

#[test]
fn empty_group_creates_no_step() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.apply(&titled(&deck, "Avant"), "Avant", None).unwrap();
    r.begin_group("Tour vide");
    r.end_group();
    assert_that!(r.undo_label()).is_equal_to(Some("Avant".to_owned()));
}

#[test]
fn undo_inside_a_group_closes_it() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.begin_group("Tour");
    r.apply(&titled(&deck, "Un"), "a", None).unwrap();
    r.undo().unwrap();
    assert_that!(r.in_group()).is_false();
    r.apply(&titled(&deck, "Seul"), "Seul", None).unwrap();
    assert_that!(r.undo_label()).is_equal_to(Some("Seul".to_owned()));
}

#[test]
fn a_new_group_closes_the_open_one() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.begin_group("Premier");
    r.apply(&titled(&deck, "Un"), "a", None).unwrap();
    r.begin_group("Second");
    r.apply(&titled(&deck, "Deux"), "b", None).unwrap();
    r.end_group();
    assert_that!(r.undo_label()).is_equal_to(Some("Second".to_owned()));
    r.undo().unwrap();
    assert_that!(r.undo_label()).is_equal_to(Some("Premier".to_owned()));
}

/// `roadmap` et, dans `blocks` de `s-constat`, l'élément `f` (`liste`) aux items `ids`.
fn with_items(ids: &[String]) -> Deck {
    let items: Vec<serde_json::Value> = ids.iter().map(|id| serde_json::json!({ "id": id, "date": id })).collect();
    let ops = vec![
        json_op(
            serde_json::json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
            "element": { "id": "f", "kind": "liste" } }),
        ),
        json_op(serde_json::json!({ "op": "set_param", "target": { "slide": "s-constat", "element": "f" },
            "param": "etapes", "value": items })),
    ];
    let mut deck = roadmap();
    ok(crate::apply_all(&mut deck, ops, &crate::test_registry::with_list()));
    deck
}

#[test]
fn step_partly_dropped_by_loro_is_no_longer_undoable() {
    // Limite 1 : Loro garde 64 items ; renverser 80 items d'une liste en fait
    // 79, un commit par `mov` (parade t-263). Les slides, dans l'arbre du
    // deck, n'en font qu'un.
    let ids: Vec<String> = (0..80).map(|k| format!("i{k}")).collect();
    let deck = with_items(&ids);
    let mut r = DeckReplica::new(&deck, crate::test_registry::with_list()).unwrap();
    r.set_undo_limit(1);
    let reversed = with_items(&ids.iter().rev().cloned().collect::<Vec<_>>());
    r.apply(&reversed, "Inverser", None).unwrap();
    assert_that!(r.can_undo()).is_false();
    assert_that!(r.undo().unwrap()).is_false();
    assert_that!(r.deck().0).is_equal_to(&reversed);
}

fn subtitled(deck: &Deck, t: &str) -> Deck {
    let mut d = deck.clone();
    d.slides[2].params.set("subtitle", crate::ParamValue::Text(t.into()));
    d
}

#[test]
fn applies_with_the_same_merge_key_are_one_undo_step() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let one = titled(&deck, "Un");
    r.apply(&one, "Modifier la slide", Some("k1")).unwrap();
    r.apply(&subtitled(&one, "Deux"), "Modifier le texte", Some("k1")).unwrap();
    assert_that!(r.undo_label()).is_equal_to(Some("Modifier la slide".to_owned()));
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&deck);
    assert_that!(r.can_undo()).is_false();
}

#[test]
fn apply_without_key_between_two_keyed_applies_splits_them() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let one = titled(&deck, "Un");
    r.apply(&one, "A", Some("k1")).unwrap();
    let two = subtitled(&one, "Agent");
    r.apply(&two, "Agent", None).unwrap();
    r.apply(&titled(&two, "Trois"), "B", Some("k1")).unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&two);
    assert_that!(r.undo_label()).is_equal_to(Some("Agent".to_owned()));
}

#[test]
fn apply_with_another_key_opens_a_new_step() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let one = titled(&deck, "Un");
    r.apply(&one, "A", Some("k1")).unwrap();
    r.apply(&titled(&deck, "Deux"), "B", Some("k2")).unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&one);
}

#[test]
fn keyed_apply_after_undo_does_not_join_the_redone_history() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let one = titled(&deck, "Un");
    r.apply(&one, "A", Some("k1")).unwrap();
    r.undo().unwrap();
    r.redo().unwrap();
    r.apply(&titled(&deck, "Deux"), "B", Some("k1")).unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&one);
}

#[test]
fn keyed_apply_after_undoing_a_later_step_opens_a_new_step() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let one = titled(&deck, "Un");
    r.apply(&one, "A", Some("k1")).unwrap();
    r.apply(&subtitled(&one, "Agent"), "Agent", None).unwrap();
    r.undo().unwrap();
    r.apply(&subtitled(&one, "Deux"), "B", Some("k1")).unwrap();
    assert_that!(r.undo().unwrap()).is_true();
    assert_that!(r.deck().0).is_equal_to(&one);
}

#[test]
fn undo_steps_counts_new_steps_but_not_merged_applies() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let start = r.undo_steps();
    r.apply(&titled(&deck, "Un"), "a", Some("k")).unwrap();
    assert_that!(r.undo_steps()).is_equal_to(start + 1);
    // Même clé : l'étape du sommet grandit.
    r.apply(&titled(&deck, "Deux"), "a", Some("k")).unwrap();
    assert_that!(r.undo_steps()).is_equal_to(start + 1);
    r.begin_group("Tour");
    r.apply(&titled(&deck, "Trois"), "a", Some("k")).unwrap();
    r.apply(&titled(&deck, "Quatre"), "a", Some("k")).unwrap();
    r.end_group();
    // Le groupe est une nouvelle étape, une seule.
    assert_that!(r.undo_steps()).is_equal_to(start + 2);
    r.apply(&titled(&deck, "Cinq"), "a", Some("k")).unwrap();
    assert_that!(r.undo_steps()).is_equal_to(start + 3);
}

#[test]
fn undo_step_names_the_top_step_and_follows_undo_and_redo() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    assert_that!(r.undo_step()).is_none();
    r.apply(&titled(&deck, "Un"), "Un", None).unwrap();
    let first = r.undo_step();
    r.begin_group("Tour");
    r.apply(&titled(&deck, "Deux"), "a", None).unwrap();
    r.apply(&subtitled(&titled(&deck, "Deux"), "b"), "b", None).unwrap();
    r.end_group();
    let turn = r.undo_step();
    assert_that!(turn).is_equal_to(Some(r.undo_steps()));
    assert_that!(turn).is_not_equal_to(first);
    r.undo().unwrap();
    assert_that!(r.undo_step()).is_equal_to(first);
    r.redo().unwrap();
    assert_that!(r.undo_step()).is_equal_to(turn);
}

#[test]
fn undo_step_is_kept_by_a_merged_apply() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.apply(&titled(&deck, "Un"), "a", Some("k")).unwrap();
    let step = r.undo_step();
    r.apply(&titled(&deck, "Deux"), "a", Some("k")).unwrap();
    assert_that!(r.undo_step()).is_equal_to(step);
}

#[test]
fn clear_undo_leaves_nothing_to_undo_or_redo() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.apply(&titled(&deck, "Un"), "Un", None).unwrap();
    r.apply(&titled(&deck, "Deux"), "Deux", None).unwrap();
    r.undo().unwrap();
    r.clear_undo();
    assert_that!((r.can_undo(), r.can_redo())).is_equal_to((false, false));
}

#[test]
fn clear_undo_inside_a_group_keeps_the_next_apply_undoable_under_the_group() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    r.begin_group("Tour");
    r.apply(&titled(&deck, "Un"), "Un", None).unwrap();
    r.clear_undo();
    r.apply(&titled(&deck, "Deux"), "Deux", None).unwrap();
    r.end_group();
    assert_that!(r.undo_label()).is_equal_to(Some("Tour".to_owned()));
    r.undo().unwrap();
    assert_that!(r.deck().0.slides[2].params.text("title")).is_equal_to(Some("Un"));
}

/// La valeur d'un résultat attendu réussi.
fn ok<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e:?}"))
}

/// Le deck au tableau `t` (3 lignes × 2 colonnes) dans la slide `s`.
fn table_deck() -> Deck {
    let mut deck = ok(Deck::from_json(r#"{"slides":[{"id":"s","kind":"content"}]}"#));
    let add = serde_json::from_value(serde_json::json!({ "op": "add_element",
        "slot": { "owner": { "slide": "s" }, "param": "blocks" },
        "element": { "id": "t", "kind": "base/table", "params": { "data": { "dataset": {
            "source": [["Pays", "Ventes"], ["FR", 10], ["DE", 20]],
            "ids": ["h", "fr", "de"], "columns": ["pays", "ventes"] } } } } }))
    .unwrap_or_else(|e| panic!("op invalide : {e}"));
    ok(crate::apply_all(&mut deck, vec![add], &crate::test_registry::registry()));
    deck
}

fn with_data(deck: &Deck, change: oku_slides_chart::DataOp) -> Deck {
    let mut d = deck.clone();
    let target = crate::Target { slide: "s".into(), element: Some("t".into()) };
    let op = crate::Op::SetData { target, param: "data".into(), change };
    ok(crate::apply_all(&mut d, vec![op], &crate::test_registry::registry()));
    d
}

/// Applique `after` sur `before`, puis vérifie qu'annuler rend `before` et rétablir `after`.
fn assert_undo_redo_exact(before: &Deck, after: &Deck) {
    let mut r = ok(DeckReplica::new(before, crate::test_registry::shared()));
    ok(r.apply(after, "Données", None));
    assert_that!(r.deck().0).is_equal_to(after);
    assert_that!(ok(r.undo())).is_true();
    let (read, issues) = r.deck();
    assert_that!(issues).is_empty();
    assert_that!(read).is_equal_to(before);
    assert_that!(ok(r.redo())).is_true();
    let (read, issues) = r.deck();
    assert_that!(issues).is_empty();
    assert_that!(read).is_equal_to(after);
}

#[test]
fn undo_redo_of_set_cell_on_table_restores_exactly() {
    use oku_slides_chart::{Cell, DataOp};
    let deck = table_deck();
    assert_undo_redo_exact(&deck, &with_data(&deck, DataOp::SetCell { row: 2, col: 1, value: Cell::Number(21.0) }));
}

#[test]
fn undo_redo_of_remove_row_restores_id_and_cells() {
    let deck = table_deck();
    let removed = with_data(&deck, oku_slides_chart::DataOp::RemoveRow { at: 1 });
    assert_undo_redo_exact(&deck, &removed);
}

#[test]
fn undo_redo_of_insert_column_restores_exactly() {
    use oku_slides_chart::{Cell, DataOp};
    let deck = table_deck();
    let cells = vec![Cell::Text("Stock".into()), Cell::Number(1.0), Cell::Null];
    assert_undo_redo_exact(&deck, &with_data(&deck, DataOp::InsertColumn { at: 1, cells, id: None }));
}

#[test]
fn undo_redo_of_replace_restores_exactly() {
    use oku_slides_chart::{Cell, DataOp};
    let deck = table_deck();
    let source = vec![
        vec![Cell::Text("A".into()), Cell::Text("B".into()), Cell::Text("C".into())],
        vec![Cell::Number(1.0), Cell::Bool(true), Cell::Null],
    ];
    assert_undo_redo_exact(&deck, &with_data(&deck, DataOp::Replace { source }));
}

#[test]
fn undo_redo_of_reordered_rows_and_columns_restores_exactly() {
    let deck = table_deck();
    let mut reordered = deck.clone();
    let target = crate::Target { slide: "s".into(), element: Some("t".into()) };
    let value = serde_json::json!({ "dataset": {
        "source": [["Ventes", "Pays"], [20, "DE"], [10, "FR"]],
        "ids": ["h", "de", "fr"], "columns": ["ventes", "pays"] } });
    let op = crate::Op::SetParam { target, param: "data".into(), value };
    crate::apply_all(&mut reordered, vec![op], &crate::test_registry::registry()).unwrap();
    assert_undo_redo_exact(&deck, &reordered);
}

// Éléments d'une slide en arbre (t-694) : la parade t-263 ne vise plus que les
// listes ; ces tests vérifient qu'annuler et rétablir un `mov` d'arbre, seul ou
// mêlé à un `mov` de liste, rend l'état exact.

fn json_op(v: serde_json::Value) -> crate::Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
}

fn with_ops(deck: &Deck, ops: Vec<crate::Op>) -> Deck {
    let mut d = deck.clone();
    ok(crate::apply_all(&mut d, ops, &crate::test_registry::registry()));
    d
}

/// Un texte `id` de corps `t`.
fn text(id: &str, t: &str) -> serde_json::Value {
    serde_json::json!({ "id": id, "kind": "base/text", "params": { "body": [{ "runs": [{ "t": t }] }] } })
}

/// Ajoute `element` au slot `param` de `owner` (`None` : la slide `s-constat`).
fn add(owner: Option<&str>, param: &str, element: serde_json::Value) -> crate::Op {
    let owner = match owner {
        Some(e) => serde_json::json!({ "slide": "s-constat", "element": e }),
        None => serde_json::json!({ "slide": "s-constat" }),
    };
    json_op(serde_json::json!({ "op": "add_element", "slot": { "owner": owner, "param": param }, "element": element }))
}

fn remove(id: &str) -> crate::Op {
    json_op(serde_json::json!({ "op": "remove_element", "slide": "s-constat", "element": id }))
}

/// `roadmap` et un texte cadré `n` dans `elements` de `s-constat`.
fn two_slots() -> Deck {
    let mut n = text("n", "Note libre");
    n["frame"] = serde_json::json!({ "x": 100, "y": 100, "w": 400, "h": 200 });
    with_ops(&roadmap(), vec![add(None, "elements", n)])
}

/// `roadmap` et, dans `blocks` de `s-constat`, une `pile` à deux enfants
/// dont un texte riche (gras, italique, lien).
fn with_pile() -> Deck {
    let rich = serde_json::json!({ "id": "riche", "kind": "base/text", "params": { "body": [
        { "runs": [{ "t": "Gras", "bold": true }, { "t": " et " }, { "t": "lien", "italic": true,
            "link": "https://example.org" }] },
        { "list": "bullet", "runs": [{ "t": "Puce" }] } ] } });
    with_ops(
        &roadmap(),
        vec![
            add(None, "blocks", serde_json::json!({ "id": "pile", "kind": "base/stack" })),
            add(Some("pile"), "children", text("k1", "Un")),
            add(Some("pile"), "children", rich),
        ],
    )
}

/// L'élément `id` de `s-constat`, tel quel.
fn element(deck: &Deck, id: &str) -> serde_json::Value {
    let e = deck.slide("s-constat").and_then(|s| s.locate(id)).map(|l| l.element.clone());
    ok(serde_json::to_value(e.unwrap_or_else(|| panic!("{id} absent"))))
}

/// `deck` où `b-points` quitte `blocks` pour `elements`, cadré.
fn points_to_elements(deck: &Deck) -> Deck {
    let mut moved = element(deck, "b-points");
    moved["frame"] = serde_json::json!({ "x": 0, "y": 0, "w": 400, "h": 200 });
    with_ops(deck, vec![remove("b-points"), add(None, "elements", moved)])
}

#[test]
fn undo_redo_of_a_slide_removal_restores_its_element_tree() {
    let before = with_pile();
    let mut after = before.clone();
    after.slides.retain(|s| s.id != "s-constat");
    assert_undo_redo_exact(&before, &after);
    let mut r = ok(DeckReplica::new(&before, crate::test_registry::shared()));
    ok(r.apply(&after, "Supprimer", None));
    assert_that!(ok(r.undo())).is_true();
    // `node_of` exige l'arbre du deck.
    let (tree, pile) = super::super::tree::node_of(&r.doc, "s-constat", "pile");
    assert_that!(tree.children_num(pile)).is_equal_to(Some(2));
    assert_that!(ok(r.redo())).is_true();
    assert_that!(ok(r.undo())).is_true();
    let (tree, pile) = super::super::tree::node_of(&r.doc, "s-constat", "pile");
    let (_, riche) = super::super::tree::node_of(&r.doc, "s-constat", "riche");
    assert_that!(tree.parent(riche)).is_equal_to(Some(loro::TreeParentId::Node(pile)));
    assert_that!(r.deck()).is_equal_to((before, Vec::new()));
}

#[test]
fn undo_redo_of_removing_an_owner_with_children_restores_exactly() {
    let before = with_pile();
    assert_undo_redo_exact(&before, &with_ops(&before, vec![remove("pile")]));
}

#[test]
fn undo_redo_of_a_slide_move_with_an_element_move_restores_exactly() {
    let before = two_slots();
    let mut after = points_to_elements(&before);
    let at = after.slides.iter().position(|s| s.id == "s-constat").unwrap_or_else(|| panic!("s-constat absente"));
    let s = after.slides.remove(at);
    after.slides.insert(0, s);
    assert_undo_redo_exact(&before, &after);
}

#[test]
fn undo_redo_of_a_move_across_slots_restores_exactly() {
    let before = two_slots();
    assert_undo_redo_exact(&before, &points_to_elements(&before));
}

#[test]
fn undo_redo_of_a_move_into_a_child_slot_restores_exactly() {
    let before = with_pile();
    let moved = element(&before, "b-points");
    let after = with_ops(&before, vec![remove("b-points"), add(Some("pile"), "children", moved)]);
    assert_undo_redo_exact(&before, &after);
}

#[test]
fn undo_redo_of_a_full_element_reversal_restores_exactly() {
    let blocks: Vec<serde_json::Value> = (0..8).map(|k| text(&format!("t{k}"), &format!("Texte {k}"))).collect();
    let before: Deck = ok(serde_json::from_value(serde_json::json!({
        "slides": [{ "id": "s", "kind": "content", "params": { "blocks": blocks } }] })));
    let mut after = before.clone();
    let Some(slot) = after.slides[0].params.slot_mut("blocks") else { panic!("blocks absent") };
    slot.reverse();
    assert_undo_redo_exact(&before, &after);
}

/// Deux pairs ouverts sur le même snapshot de `deck`.
fn two_peers(deck: &Deck) -> (DeckReplica, DeckReplica) {
    let snap = ok(ok(DeckReplica::new(deck, crate::test_registry::shared())).snapshot());
    let open = || ok(DeckReplica::from_snapshot(&snap, 1 << 24, crate::test_registry::shared())).0;
    (open(), open())
}

/// `deck` où le premier run du `body` de `b-points` est en gras.
fn points_bold(deck: &Deck) -> Deck {
    let mut body = element(deck, "b-points")["params"]["body"].clone();
    body[0]["runs"][0]["bold"] = serde_json::json!(true);
    with_ops(
        deck,
        vec![json_op(serde_json::json!({ "op": "set_param",
            "target": { "slide": "s-constat", "element": "b-points" }, "param": "body", "value": body }))],
    )
}

/// Le slot de `b-points` dans `deck`, son propriétaire, et son premier run est-il gras ?
fn points_place(deck: &Deck) -> Vec<(Option<String>, String, bool)> {
    let Some(slide) = deck.slide("s-constat") else { panic!("s-constat absente") };
    slide
        .walk()
        .filter(|l| l.element.id == "b-points")
        .map(|l| {
            let bold = match l.element.params.get("body") {
                Some(crate::ParamValue::Rich(b)) => b.first().and_then(|p| p.runs.first()).is_some_and(|r| r.bold),
                _ => false,
            };
            (l.owner.map(str::to_owned), l.slot.to_owned(), bold)
        })
        .collect()
}

#[test]
fn undo_move_across_slots_keeps_a_remote_edit() {
    let deck = two_slots();
    let (mut a, mut b) = two_peers(&deck);
    ok(a.apply(&points_to_elements(&deck), "Déplacer", None));
    ok(b.apply(&points_bold(&deck), "Gras", None));
    ok(a.merge(&b));
    assert_that!(ok(a.undo())).is_true();
    let (read, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(points_place(&read)).is_equal_to(vec![(None, "blocks".to_owned(), true)]);
    assert_that!(ok(a.redo())).is_true();
    let (read, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(points_place(&read)).is_equal_to(vec![(None, "elements".to_owned(), true)]);
    ok(b.merge(&a));
    assert_that!(b.deck()).is_equal_to(a.deck());
}

#[test]
fn undo_a_stale_move_after_the_peer_moved_it_again_converges() {
    let deck = with_pile();
    let (mut a, mut b) = two_peers(&deck);
    let moved = points_to_elements(&deck);
    ok(a.apply(&moved, "A", None));
    ok(b.merge(&a));
    assert_that!(b.deck().0).is_equal_to(&moved);
    let points = element(&moved, "b-points");
    let nested = with_ops(&moved, vec![remove("b-points"), add(Some("pile"), "children", points)]);
    ok(b.apply(&nested, "B", None));
    ok(a.merge(&b));
    assert_that!(ok(a.undo())).is_true();
    ok(a.merge(&b));
    ok(b.merge(&a));
    let (read, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(b.deck()).is_equal_to((read.clone(), Vec::new()));
    let place = points_place(&read);
    assert_that!(place.len()).is_equal_to(1);
    let declared = [(None, "blocks"), (None, "elements"), (Some("pile".to_owned()), "children")];
    assert_that!(declared.iter().any(|(o, s)| *o == place[0].0 && *s == place[0].1)).is_true();
}

// Le deck en un seul arbre (t-746), sans la parade t-263 pour les slides :
// annuler et rétablir des `mov` de slides et d'éléments d'une slide à
// l'autre rend l'état exact, et ce qu'ils produisent passe le contrôle
// d'entrée (relecture d'un snapshot, fusion avec un pair).

/// Comme [`assert_undo_redo_exact`], puis, après « rétablir » : relecture d'un
/// snapshot et fusion dans un pair du départ, sans `Issue`. Avec `repair`,
/// le premier `apply` du deck relu écrit la réparation (entrée `@slot` d'un
/// parent recréé par « rétablir ») ; sinon il n'écrit rien. Le second, jamais.
fn assert_deck_undo_redo_exact(before: &Deck, after: &Deck, repair: bool) {
    let mut r = ok(DeckReplica::new(before, crate::test_registry::shared()));
    let start = ok(r.snapshot());
    ok(r.apply(after, "Éditer", None));
    assert_that!(r.deck()).is_equal_to((after.clone(), Vec::new()));
    assert_that!(ok(r.undo())).is_true();
    assert_that!(r.deck()).is_equal_to((before.clone(), Vec::new()));
    assert_that!(ok(r.redo())).is_true();
    assert_that!(r.deck()).is_equal_to((after.clone(), Vec::new()));
    let reread = ok(DeckReplica::from_snapshot(&ok(r.snapshot()), 1 << 24, crate::test_registry::shared())).1;
    assert_that!(reread).is_equal_to((after.clone(), Vec::new()));
    let mut peer = ok(DeckReplica::from_snapshot(&start, 1 << 24, crate::test_registry::shared())).0;
    ok(peer.merge(&r));
    assert_that!(peer.deck()).is_equal_to((after.clone(), Vec::new()));
    let read = r.deck().0;
    let redone = r.len_ops();
    ok(r.apply(&read, "Relire", None));
    let settled = r.len_ops();
    assert_that!(settled > redone).named("réparation écrite au premier apply").is_equal_to(repair);
    ok(r.apply(&read, "Relire", None));
    assert_that!(r.len_ops()).is_equal_to(settled);
    assert_that!(r.deck()).is_equal_to((after.clone(), Vec::new()));
}

/// Ajoute `element` en `index` du slot `param` de `owner` (`None` : la slide) de `slide`.
fn add_to(
    slide: &str,
    owner: Option<&str>,
    param: &str,
    index: Option<usize>,
    element: serde_json::Value,
) -> crate::Op {
    let owner = match owner {
        Some(e) => serde_json::json!({ "slide": slide, "element": e }),
        None => serde_json::json!({ "slide": slide }),
    };
    json_op(serde_json::json!({ "op": "add_element", "slot": { "owner": owner, "param": param }, "index": index,
        "element": element }))
}

/// `deck` où `b-points` passe de `s-constat` en tête de `blocks` de `s-priorites`.
fn points_to_priorities(deck: &Deck) -> Deck {
    let points = element(deck, "b-points");
    with_ops(deck, vec![remove("b-points"), add_to("s-priorites", None, "blocks", Some(0), points)])
}

#[test]
fn undo_redo_of_a_slide_reversal_restores_exactly() {
    let before = roadmap();
    let mut after = before.clone();
    after.slides.reverse();
    assert_deck_undo_redo_exact(&before, &after, false);
}

#[test]
fn undo_redo_of_a_move_between_slides_restores_exactly() {
    let before = roadmap();
    assert_deck_undo_redo_exact(&before, &points_to_priorities(&before), false);
}

#[test]
fn undo_redo_of_a_slide_removal_an_element_move_and_a_reorder_in_one_apply_restores_exactly() {
    let before = roadmap();
    let mut after = points_to_priorities(&before);
    after.slides.retain(|s| s.id != "s-ch1");
    let at = after.slides.iter().position(|s| s.id == "s-libre").unwrap_or_else(|| panic!("s-libre absente"));
    let libre = after.slides.remove(at);
    after.slides.insert(0, libre);
    assert_deck_undo_redo_exact(&before, &after, false);
}

/// `pile` (enfants `k1`, `k2`) ajoutée à `blocks` de `s-constat`.
fn pile_with_children() -> Vec<crate::Op> {
    vec![
        add(None, "blocks", serde_json::json!({ "id": "pile", "kind": "base/stack" })),
        add(Some("pile"), "children", text("k1", "Un")),
        add(Some("pile"), "children", text("k2", "Deux")),
    ]
}

/// Rétablir l'ajout rend le même `pile` (t-776) : `k1` et `k2` gardent leur
/// entrée, rien à réparer.
#[test]
fn undo_redo_of_adding_an_owner_with_children_restores_exactly() {
    let before = roadmap();
    assert_deck_undo_redo_exact(&before, &with_ops(&before, pile_with_children()), false);
}

#[test]
fn undo_redo_of_adding_a_slide_with_elements_restores_exactly() {
    let before = roadmap();
    let mut after = before.clone();
    after.slides.insert(2, Slide::new("s-neuve", "content"));
    let after = with_ops(
        &after,
        vec![
            add_to("s-neuve", None, "blocks", None, text("n1", "Un")),
            add_to("s-neuve", None, "blocks", None, serde_json::json!({ "id": "n2", "kind": "base/stack" })),
        ],
    );
    assert_deck_undo_redo_exact(&before, &after, false);
}

/// Après « rétablir », `pile` est le même nœud (t-776) : `b-points` porte
/// l'entrée de `s-constat` (périmée) et celle de `pile`, sans réparation.
#[test]
fn undo_redo_of_moving_an_element_into_a_new_owner_restores_exactly() {
    let before = roadmap();
    let points = element(&before, "b-points");
    let after = with_ops(
        &before,
        vec![
            add(None, "blocks", serde_json::json!({ "id": "pile", "kind": "base/stack" })),
            remove("b-points"),
            add(Some("pile"), "children", points),
        ],
    );
    assert_deck_undo_redo_exact(&before, &after, false);
}

/// Où est `b-points` dans `deck` (slide, slot) et son premier run est-il gras ?
fn points_home(deck: &Deck) -> Vec<(String, String, bool)> {
    deck.slides
        .iter()
        .flat_map(|slide| {
            slide.walk().filter(|l| l.element.id == "b-points").map(|l| {
                let bold = match l.element.params.get("body") {
                    Some(crate::ParamValue::Rich(b)) => b.first().and_then(|p| p.runs.first()).is_some_and(|r| r.bold),
                    _ => false,
                };
                (slide.id.clone(), l.slot.to_owned(), bold)
            })
        })
        .collect()
}

/// A passe `b-points` dans `s-priorites`, B le met en gras : l'annulation de
/// A rend `b-points` à `s-constat` avec l'édition de B (le `mov` ne l'écrase pas).
#[test]
fn undo_move_between_slides_keeps_a_remote_edit() {
    let deck = roadmap();
    let (mut a, mut b) = two_peers(&deck);
    ok(a.apply(&points_to_priorities(&deck), "Déplacer", None));
    ok(b.apply(&points_bold(&deck), "Gras", None));
    ok(a.merge(&b));
    assert_that!(ok(a.undo())).is_true();
    let (read, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(points_home(&read)).is_equal_to(vec![("s-constat".to_owned(), "blocks".to_owned(), true)]);
    assert_that!(ok(a.redo())).is_true();
    let (read, issues) = a.deck();
    assert_that!(issues).is_empty();
    assert_that!(points_home(&read)).is_equal_to(vec![("s-priorites".to_owned(), "blocks".to_owned(), true)]);
    ok(b.merge(&a));
    assert_that!(b.deck()).is_equal_to(a.deck());
}

/// A retire `s-constat`, B en sort `b-points` : après l'échange, `b-points`
/// est dans `s-priorites`. Annuler le retrait rend la slide sans lui.
#[test]
fn undo_of_a_slide_removal_keeps_a_remote_move_of_one_of_its_elements() {
    let deck = roadmap();
    let (mut a, mut b) = two_peers(&deck);
    let mut removed = deck.clone();
    removed.slides.retain(|s| s.id != "s-constat");
    ok(a.apply(&removed, "Retirer", None));
    ok(b.apply(&points_to_priorities(&deck), "Déplacer", None));
    ok(a.merge(&b));
    ok(b.merge(&a));
    for peer in [&a, &b] {
        let (read, issues) = peer.deck();
        assert_that!(issues).is_empty();
        assert_that!(read.slide("s-constat").is_none()).is_true();
        assert_that!(points_home(&read)).is_equal_to(vec![("s-priorites".to_owned(), "blocks".to_owned(), false)]);
    }
    assert_that!(ok(a.undo())).is_true();
    ok(b.merge(&a));
    ok(a.merge(&b));
    for peer in [&a, &b] {
        let (read, issues) = peer.deck();
        assert_that!(issues).is_empty();
        let back = read.slide("s-constat").unwrap_or_else(|| panic!("s-constat absente"));
        assert_that!(back.locate("b-points").is_none()).is_true();
        assert_that!(points_home(&read)).is_equal_to(vec![("s-priorites".to_owned(), "blocks".to_owned(), false)]);
    }
    assert_that!(a.deck()).is_equal_to(b.deck());
}
