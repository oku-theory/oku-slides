//! Propriété : des pairs qui éditent en parallèle puis échangent tout convergent.

mod common;

use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

use oku_slides_chart::{Cell, DataOp, parse_table};
use oku_slides_core::{
    Deck, DeckReplica, Op, PackVersion, ParamValue, Params, Slide, Target, ThemeRef, WidgetRegistry, apply_all,
    validate,
};
use proptest::prelude::*;
use serde_json::{Value, json};

#[derive(Debug, Clone)]
enum Edit {
    Title(usize, String),
    Notes(usize, String),
    Move(usize, usize),
    Delete(usize),
    Add(usize),
    Theme(Option<ThemeRef>),
    Bold(usize),
    /// `set_cell` du tableau `t` : ligne, colonne, valeur.
    SetCell(usize, usize, i32),
    InsertRow(usize),
    RemoveRow(usize),
    InsertColumn(usize),
    RemoveColumn(usize),
}

/// Une référence de thème de test : version `<c>` répété 64 fois.
fn theme(c: &str) -> ThemeRef {
    let version = PackVersion::from_hex(&c.repeat(64)).unwrap_or_else(|| panic!("hex invalide : {c}"));
    ThemeRef { id: uuid::Uuid::from_u128(u128::from(c.as_bytes()[0])), version }
}

fn edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        (any::<usize>(), "[a-zé ]{0,10}").prop_map(|(i, s)| Edit::Title(i, s)),
        (any::<usize>(), "[a-z ]{0,10}").prop_map(|(i, s)| Edit::Notes(i, s)),
        (any::<usize>(), any::<usize>()).prop_map(|(a, b)| Edit::Move(a, b)),
        any::<usize>().prop_map(Edit::Delete),
        any::<usize>().prop_map(Edit::Add),
        prop::option::of(prop_oneof![Just(theme("a")), Just(theme("b"))]).prop_map(Edit::Theme),
        any::<usize>().prop_map(Edit::Bold),
        (any::<usize>(), any::<usize>(), -50i32..50).prop_map(|(r, c, v)| Edit::SetCell(r, c, v)),
        any::<usize>().prop_map(Edit::InsertRow),
        any::<usize>().prop_map(Edit::RemoveRow),
        any::<usize>().prop_map(Edit::InsertColumn),
        any::<usize>().prop_map(Edit::RemoveColumn),
    ]
}

/// Le registre de base, construit une fois.
fn base() -> &'static WidgetRegistry {
    static BASE: OnceLock<WidgetRegistry> = OnceLock::new();
    BASE.get_or_init(common::registry)
}

/// Le paramètre `data` du tableau `t` de `s-constat`, s'il existe.
fn table_data(deck: &Deck) -> Option<&Value> {
    deck.slide("s-constat")?.locate("t")?.element.params.value("data")
}

/// La ligne et la colonne jamais retirées par le proptest :
/// sans l'en-tête ni la première colonne, une fusion pourrait vider le tableau.
const KEPT_ROW: &str = "h";
const KEPT_COLUMN: &str = "pays";

/// Une édition du tableau `t`, aux index tirés dans ses bornes courantes ;
/// une op refusée laisse le deck inchangé.
fn edit_table(deck: &mut Deck, e: &Edit) {
    let Some(data) = table_data(deck) else { return };
    let ds = &data["dataset"];
    let source = ds["source"].as_array().cloned().unwrap_or_default();
    let ids: Vec<&str> = ds["ids"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    let columns: Vec<&str> =
        ds["columns"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    let (rows, width) = (source.len(), columns.len());
    if rows == 0 || width == 0 {
        return;
    }
    let change = match *e {
        Edit::SetCell(r, c, v) => DataOp::SetCell { row: r % rows, col: c % width, value: Cell::Number(f64::from(v)) },
        Edit::InsertRow(at) => DataOp::InsertRow { at: at % (rows + 1), cells: vec![Cell::Null; width], id: None },
        Edit::RemoveRow(at) if ids.get(at % rows) != Some(&KEPT_ROW) => DataOp::RemoveRow { at: at % rows },
        // Au plus 5 colonnes par pair : 4 pairs fusionnés restent sous la borne de 20.
        Edit::InsertColumn(at) if width < 5 => {
            DataOp::InsertColumn { at: at % (width + 1), cells: vec![Cell::Null; rows], id: None }
        }
        Edit::RemoveColumn(at) if columns.get(at % width) != Some(&KEPT_COLUMN) => {
            DataOp::RemoveColumn { at: at % width }
        }
        _ => return,
    };
    let target = Target { slide: "s-constat".into(), element: Some("t".into()) };
    let _refused = apply_all(deck, vec![Op::SetData { target, param: "data".into(), change }], base());
}

/// `parse_table` réussit sur `data`, et `ids`/`columns` sont alignés et uniques.
fn assert_table_ok(data: &Value) -> Vec<Vec<Cell>> {
    let table = parse_table(data).unwrap_or_else(|e| panic!("tableau invalide : {e:?} ({data})"));
    let ds = &table.dataset;
    assert_eq!(ds.ids.len(), ds.source.len(), "ids non alignés : {data}");
    assert_eq!(ds.columns.len(), ds.source.first().map_or(0, Vec::len), "columns non alignées : {data}");
    assert_eq!(ds.ids.iter().collect::<HashSet<_>>().len(), ds.ids.len(), "ids en double : {data}");
    assert_eq!(ds.columns.iter().collect::<HashSet<_>>().len(), ds.columns.len(), "columns en double : {data}");
    table.dataset.source
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
        Edit::SetCell(..) | Edit::InsertRow(_) | Edit::RemoveRow(_) | Edit::InsertColumn(_) | Edit::RemoveColumn(_) => {
            edit_table(deck, e)
        }
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
        let deck = with_table(Deck::from_json(include_str!("../../../fixtures/roadmap.json")).unwrap(), base());
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
        if let Some(data) = table_data(&first) {
            assert_table_ok(data);
        }
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

/// `deck` et, dans les blocs de `s-constat`, le tableau `t` : 3 lignes × 2
/// colonnes, aux identifiants explicites.
fn with_table(deck: Deck, registry: &WidgetRegistry) -> Deck {
    let data = json!({ "dataset": {
        "source": [["Pays", "Ventes"], ["FR", 10], ["DE", 20]],
        "ids": ["h", "fr", "de"],
        "columns": ["pays", "ventes"],
    } });
    let add = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "t", "kind": "table", "params": { "data": data } } }));
    let mut d = deck;
    ok(apply_all(&mut d, vec![add], registry));
    d
}

/// `roadmap` avec le tableau `t`.
fn table_deck() -> Deck {
    with_table(ok(Deck::from_json(include_str!("../../../fixtures/roadmap.json"))), &common::list_registry())
}

/// Le deck au tableau `t`, et deux pairs issus du même snapshot.
fn table_peers() -> (Deck, DeckReplica, DeckReplica) {
    let deck = table_deck();
    let (a, b) = list_peers(&deck);
    (deck, a, b)
}

/// Un `set_data` du tableau `t`.
fn data_op(change: DataOp) -> Op {
    Op::SetData {
        target: Target { slide: "s-constat".into(), element: Some("t".into()) },
        param: "data".into(),
        change,
    }
}

fn set_cell(row: usize, col: usize, value: Value) -> Op {
    data_op(DataOp::SetCell { row, col, value: cell(value) })
}

fn cell(v: Value) -> Cell {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("cellule invalide : {e}"))
}

fn cells(v: Value) -> Vec<Cell> {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("cellules invalides : {e}"))
}

/// Le `source` du tableau `t` fusionné, vérifié (`parse_table`, identifiants).
fn merged_source(deck: &Deck) -> Vec<Vec<Cell>> {
    assert_table_ok(table_data(deck).unwrap_or_else(|| panic!("tableau t absent")))
}

fn source(v: Value) -> Vec<Vec<Cell>> {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("source invalide : {e}"))
}

/// Les identifiants (`ids`, `columns`) du tableau `t`.
fn table_ids(deck: &Deck) -> (Vec<String>, Vec<String>) {
    let data = table_data(deck).unwrap_or_else(|| panic!("tableau t absent"));
    let table = ok(parse_table(data));
    (table.dataset.ids, table.dataset.columns)
}

#[test]
fn concurrent_edits_of_two_cells_in_one_row_are_both_kept() {
    let merged = exchange(&table_deck(), vec![set_cell(1, 0, json!("France"))], vec![set_cell(1, 1, json!(11))]);
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes"], ["France", 11], ["DE", 20]])));
}

#[test]
fn concurrent_edits_of_two_cells_in_one_column_are_both_kept() {
    let merged = exchange(&table_deck(), vec![set_cell(1, 1, json!(11))], vec![set_cell(2, 1, json!(21))]);
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes"], ["FR", 11], ["DE", 21]])));
}

#[test]
fn concurrent_row_inserts_are_both_kept() {
    let merged = exchange(
        &table_deck(),
        vec![data_op(DataOp::InsertRow { at: 3, cells: cells(json!(["IT", 30])), id: Some("it".into()) })],
        vec![data_op(DataOp::InsertRow { at: 1, cells: cells(json!(["ES", 40])), id: None })],
    );
    assert_eq!(
        merged_source(&merged),
        source(json!([["Pays", "Ventes"], ["ES", 40], ["FR", 10], ["DE", 20], ["IT", 30]]))
    );
    let (ids, _) = table_ids(&merged);
    assert_eq!(ids.len(), 5);
    assert_eq!(ids[4], "it");
}

#[test]
fn concurrent_column_inserts_are_both_kept() {
    let merged = exchange(
        &table_deck(),
        vec![data_op(DataOp::InsertColumn { at: 2, cells: cells(json!(["Stock", 1, 2])), id: Some("stock".into()) })],
        vec![data_op(DataOp::InsertColumn { at: 1, cells: cells(json!(["Code", "f", "d"])), id: Some("code".into()) })],
    );
    assert_eq!(
        merged_source(&merged),
        source(json!([["Pays", "Code", "Ventes", "Stock"], ["FR", "f", 10, 1], ["DE", "d", 20, 2]]))
    );
    assert_eq!(table_ids(&merged).1, vec!["pays", "code", "ventes", "stock"]);
}

#[test]
fn concurrent_row_remove_and_other_row_edit_merge() {
    let merged = exchange(&table_deck(), vec![data_op(DataOp::RemoveRow { at: 1 })], vec![set_cell(2, 1, json!(21))]);
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes"], ["DE", 21]])));
    assert_eq!(table_ids(&merged).0, vec!["h", "de"]);
}

#[test]
fn concurrent_column_insert_and_cell_edit_merge() {
    let merged = exchange(
        &table_deck(),
        vec![data_op(DataOp::InsertColumn { at: 2, cells: cells(json!(["Stock", 1, 2])), id: None })],
        vec![set_cell(1, 1, json!(11))],
    );
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes", "Stock"], ["FR", 11, 1], ["DE", 20, 2]])));
}

#[test]
fn concurrent_row_insert_and_column_insert_leave_new_row_aligned() {
    let merged = exchange(
        &table_deck(),
        vec![data_op(DataOp::InsertRow { at: 3, cells: cells(json!(["IT", 30])), id: Some("it".into()) })],
        vec![data_op(DataOp::InsertColumn { at: 2, cells: cells(json!(["Stock", 1, 2])), id: Some("stock".into()) })],
    );
    assert_eq!(
        merged_source(&merged),
        source(json!([["Pays", "Ventes", "Stock"], ["FR", 10, 1], ["DE", 20, 2], ["IT", 30, null]]))
    );
}

/// A et B retirent la même chose (`change`), échangent, annulent chacun,
/// échangent encore ; puis chacun réécrit le deck relu et ils échangent une
/// dernière fois. Le deck commun après la double annulation.
fn removed_by_both_then_undone_by_both(change: DataOp) -> Deck {
    let (deck, mut a, mut b) = table_peers();
    let removed = with_ops(&deck, vec![data_op(change)]);
    ok(a.apply(&removed, "A", None));
    ok(b.apply(&removed, "B", None));
    ok(a.merge(&b));
    ok(b.merge(&a));
    assert!(ok(a.undo()));
    assert!(ok(b.undo()));
    ok(a.merge(&b));
    ok(b.merge(&a));
    let (merged, issues) = a.deck();
    assert!(issues.is_empty(), "lecture partielle : {issues:?}");
    assert_eq!(b.deck().0, merged);
    // La prochaine écriture retire le double caché, des deux côtés : l'état converge.
    ok(a.apply(&merged, "A", None));
    ok(b.apply(&merged, "B", None));
    ok(a.merge(&b));
    ok(b.merge(&a));
    assert_eq!(a.deck().0, merged);
    assert_eq!(b.deck().0, merged);
    merged
}

#[test]
fn same_row_removed_by_both_then_undone_by_both_has_no_duplicate_id() {
    let merged = removed_by_both_then_undone_by_both(DataOp::RemoveRow { at: 1 });
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes"], ["FR", 10], ["DE", 20]])));
    assert_eq!(table_ids(&merged).0, vec!["h", "fr", "de"]);
}

#[test]
fn same_column_removed_by_both_then_undone_by_both_has_no_duplicate_column() {
    let merged = removed_by_both_then_undone_by_both(DataOp::RemoveColumn { at: 1 });
    assert_eq!(merged_source(&merged), source(json!([["Pays", "Ventes"], ["FR", 10], ["DE", 20]])));
    assert_eq!(table_ids(&merged).1, vec!["pays", "ventes"]);
}
