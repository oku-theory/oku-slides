//! Paramètres `host` dans le replica : `dataset` en colonnes et lignes à
//! identifiant, réparé à la relecture.

use loro::{Container, ValueOrContainer};
use loro::{ContainerID, ContainerTrait, LoroMap, LoroMovableList, LoroValue};
use oku_slides_chart::{Cell, DataOp};
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::*;
use crate::{Deck, Op, Target, apply_all};

/// La valeur d'un résultat attendu réussi.
fn ok<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e:?}"))
}

fn registry() -> Arc<WidgetRegistry> {
    crate::test_registry::shared()
}

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
}

fn with_ops(deck: &Deck, ops: Vec<Op>) -> Deck {
    let mut d = deck.clone();
    apply_all(&mut d, ops, &registry()).unwrap_or_else(|e| panic!("ops refusées : {e:?}"));
    d
}

fn empty_deck() -> Deck {
    ok(Deck::from_json(r#"{"slides":[{"id":"s","kind":"content"}]}"#))
}

/// Les données de départ : 3 lignes × 3 colonnes, nombres, booléens, `null`.
fn data() -> Value {
    json!({ "dataset": {
        "source": [["Pays", "Ventes", "Actif"], ["FR", 1.0, true], ["DE", null, false]],
        "ids": ["h", "fr", "de"],
        "columns": ["pays", "ventes", "actif"],
    }, "align": ["start", "end", "center"] })
}

/// Le deck au tableau `t`, de données `data`.
fn table_deck(data: Value) -> Deck {
    let add = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s" }, "param": "blocks" },
        "element": { "id": "t", "kind": "table", "params": { "data": data } } }));
    with_ops(&empty_deck(), vec![add])
}

/// Un deck au tableau `t` écrit tel quel, sans passer par les ops.
fn raw_table_deck(data: Value) -> Deck {
    let deck = json!({ "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
        { "id": "t", "kind": "table", "params": { "data": data } } ] } }] });
    ok(Deck::from_json(&deck.to_string()))
}

fn data_op(change: DataOp) -> Op {
    Op::SetData { target: Target { slide: "s".into(), element: Some("t".into()) }, param: "data".into(), change }
}

fn set_data(deck: &Deck, value: Value) -> Deck {
    with_ops(
        deck,
        vec![op(
            json!({ "op": "set_param", "target": { "slide": "s", "element": "t" }, "param": "data", "value": value }),
        )],
    )
}

/// Le paramètre `data` du tableau `t`.
fn stored(deck: &Deck) -> Value {
    let t = deck.slides[0].locate("t").unwrap_or_else(|| panic!("tableau t absent"));
    t.element.params.value("data").unwrap_or_else(|| panic!("data absent")).clone()
}

/// Réconcilie `deck` dans un replica neuf, puis relit son snapshot.
fn round_trip(deck: &Deck) -> Deck {
    let r = ok(DeckReplica::new(deck, registry()));
    let (back, issues) = ok(DeckReplica::from_snapshot(&ok(r.snapshot()), 1 << 24, registry())).1;
    assert_that!(issues).is_empty();
    back
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

fn keys(m: &LoroMap) -> Vec<String> {
    let mut k: Vec<String> = m.keys().map(|k| k.to_string()).collect();
    k.sort();
    k
}

/// La carte de l'option du paramètre `param` de l'élément `id` de la slide `s`.
fn host_map(r: &DeckReplica, id: &str, param: &str) -> LoroMap {
    let (tree, n) = tree::node_of(&r.doc, "s", id);
    as_map(as_map(ok(tree.get_meta(n)).get("params")).get(param))
}

/// La carte `{columns, rows}` du tableau `t`.
fn dataset(r: &DeckReplica) -> LoroMap {
    as_map(host_map(r, "t", "data").get(schema::DATASET))
}

fn rows(r: &DeckReplica) -> LoroMovableList {
    as_list(dataset(r).get(schema::ROWS))
}

fn columns(r: &DeckReplica) -> Vec<String> {
    let l = as_list(dataset(r).get(schema::COLUMNS));
    (0..l.len())
        .map(|i| match l.get(i) {
            Some(ValueOrContainer::Value(LoroValue::String(s))) => s.to_string(),
            other => panic!("identifiant attendu : {other:?}"),
        })
        .collect()
}

/// La ligne d'identifiant `id`.
fn row(r: &DeckReplica, id: &str) -> LoroMap {
    let l = rows(r);
    (0..l.len())
        .map(|i| as_map(l.get(i)))
        .find(|m| matches!(m.get("id"), Some(ValueOrContainer::Value(v)) if v.as_string().is_some_and(|s| **s == id)))
        .unwrap_or_else(|| panic!("ligne {id} absente"))
}

fn cells(r: &DeckReplica, id: &str) -> LoroMap {
    as_map(row(r, id).get(schema::CELLS))
}

/// Identifiant de conteneur de chaque ligne, par identifiant de ligne.
fn row_containers(r: &DeckReplica) -> Vec<(String, ContainerID)> {
    let l = rows(r);
    (0..l.len())
        .map(|i| {
            let m = as_map(l.get(i));
            let id = match m.get("id") {
                Some(ValueOrContainer::Value(LoroValue::String(s))) => s.to_string(),
                other => panic!("id attendu : {other:?}"),
            };
            (id, m.id())
        })
        .collect()
}

/// Le `data` relu de `r`, et ce qui n'a pas pu être lu.
fn read(r: &DeckReplica) -> (Value, Vec<String>) {
    let (deck, issues) = r.deck();
    (stored(&deck), issues.iter().map(|i| i.message.id().to_owned()).collect())
}

#[test]
fn host_table_round_trips_through_the_replica() {
    let deck = table_deck(data());
    let back = round_trip(&deck);
    assert_that!(back).is_equal_to(&deck);
    assert_that!(stored(&back)["dataset"]).is_equal_to(&stored(&deck)["dataset"]);
    assert_that!(stored(&back)["dataset"]["source"][1][1]).is_equal_to(json!(1.0));
    assert_that!(stored(&back)["dataset"]["ids"]).is_equal_to(json!(["h", "fr", "de"]));
}

#[test]
fn host_chart_round_trips_through_the_replica() {
    let option = json!({
        "dataset": { "source": [["Mois", "Ventes"], ["Jan", 10.5], ["Fév", 14]], "ids": ["h", "jan", "fev"],
            "columns": ["mois", "ventes"] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar" }]
    });
    let add = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s" }, "param": "blocks" },
        "element": { "id": "g", "kind": "chart", "params": { "option": option } } }));
    let deck = with_ops(&empty_deck(), vec![add]);
    let back = round_trip(&deck);
    assert_that!(back).is_equal_to(&deck);
    let read = back.slides[0].locate("g").unwrap().element.params.value("option").unwrap().clone();
    assert_that!(read["dataset"]["columns"]).is_equal_to(json!(["mois", "ventes"]));
    let r = DeckReplica::new(&deck, registry()).unwrap();
    assert_that!(keys(&as_map(host_map(&r, "g", "option").get(schema::DATASET))))
        .is_equal_to(vec!["columns".to_owned(), "rows".to_owned()]);
}

#[test]
fn host_dataset_is_stored_as_columns_and_rows() {
    let r = DeckReplica::new(&table_deck(data()), registry()).unwrap();
    assert_that!(keys(&dataset(&r))).is_equal_to(vec!["columns".to_owned(), "rows".to_owned()]);
    assert_that!(columns(&r)).is_equal_to(vec!["pays".to_owned(), "ventes".to_owned(), "actif".to_owned()]);
    assert_that!(rows(&r).len()).is_equal_to(3);
    assert_that!(keys(&row(&r, "fr"))).is_equal_to(vec!["cells".to_owned(), "id".to_owned()]);
    let fr = cells(&r, "fr");
    assert_that!(keys(&fr)).is_equal_to(vec!["actif".to_owned(), "pays".to_owned(), "ventes".to_owned()]);
    assert_that!(fr.get("pays").and_then(|v| v.into_value().ok())).is_equal_to(Some(LoroValue::from("FR")));
    assert_that!(fr.get("actif").and_then(|v| v.into_value().ok())).is_equal_to(Some(LoroValue::Bool(true)));
    // Les autres clés de l'option restent des valeurs.
    assert_that!(matches!(host_map(&r, "t", "data").get("align"), Some(ValueOrContainer::Value(_)))).is_true();
}

#[test]
fn set_cell_writes_one_entry_in_the_same_row_container() {
    let deck = table_deck(data());
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let before = (row(&r, "fr").id(), r.len_ops());
    let edited = with_ops(&deck, vec![data_op(DataOp::SetCell { row: 1, col: 1, value: Cell::Number(7.0) })]);
    r.apply(&edited, "Cellule", None).unwrap();
    assert_that!(row(&r, "fr").id()).is_equal_to(&before.0);
    assert_that!(r.len_ops()).is_equal_to(before.1 + 1);
    assert_that!(read(&r).0).is_equal_to(stored(&edited));
}

#[test]
fn rewriting_an_unchanged_deck_writes_nothing() {
    let deck = table_deck(data());
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let before = r.len_ops();
    r.apply(&deck, "Rien", None).unwrap();
    r.apply(&r.deck().0, "Rien", None).unwrap();
    assert_that!(r.len_ops()).is_equal_to(before);
}

#[test]
fn row_insert_remove_reorder_keep_surviving_row_containers() {
    let deck = table_deck(data());
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let before = row_containers(&r);
    let next = set_data(
        &deck,
        json!({ "dataset": {
            "source": [["Pays", "Ventes", "Actif"], ["DE", null, false], ["IT", 3, true]],
            "ids": ["h", "de", "it"],
            "columns": ["pays", "ventes", "actif"],
        }, "align": ["start", "end", "center"] }),
    );
    r.apply(&next, "Lignes", None).unwrap();
    let after = row_containers(&r);
    assert_that!(after.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["h", "de", "it"]);
    assert_that!(after[0]).is_equal_to(&before[0]);
    assert_that!(after[1]).is_equal_to(&before[2]);
    assert_that!(read(&r).0).is_equal_to(stored(&next));
    // Réordonner seulement : chaque ligne garde son conteneur.
    let reordered = set_data(
        &next,
        json!({ "dataset": {
            "source": [["Pays", "Ventes", "Actif"], ["IT", 3, true], ["DE", null, false]],
            "ids": ["h", "it", "de"],
            "columns": ["pays", "ventes", "actif"],
        }, "align": ["start", "end", "center"] }),
    );
    r.apply(&reordered, "Ordre", None).unwrap();
    let moved = row_containers(&r);
    assert_that!(moved).is_equal_to(vec![after[0].clone(), after[2].clone(), after[1].clone()]);
    assert_that!(read(&r).0).is_equal_to(stored(&reordered));
}

#[test]
fn column_insert_remove_reorder_keep_cells_by_column_id() {
    let deck = table_deck(data());
    let mut r = DeckReplica::new(&deck, registry()).unwrap();
    let fr_cells = cells(&r, "fr").id();
    let next = set_data(
        &deck,
        json!({ "dataset": {
            "source": [["Actif", "Pays", "Stock"], [true, "FR", 4], [false, "DE", 5]],
            "ids": ["h", "fr", "de"],
            "columns": ["actif", "pays", "stock"],
        } }),
    );
    r.apply(&next, "Colonnes", None).unwrap();
    assert_that!(columns(&r)).is_equal_to(vec!["actif".to_owned(), "pays".to_owned(), "stock".to_owned()]);
    let fr = cells(&r, "fr");
    assert_that!(fr.id()).is_equal_to(&fr_cells);
    assert_that!(keys(&fr)).is_equal_to(vec!["actif".to_owned(), "pays".to_owned(), "stock".to_owned()]);
    assert_that!(fr.get("pays").and_then(|v| v.into_value().ok())).is_equal_to(Some(LoroValue::from("FR")));
    assert_that!(read(&r).0).is_equal_to(stored(&next));
}

#[test]
fn dataset_of_unexpected_shape_round_trips_unchanged() {
    let cases = [
        json!({ "dataset": { "source": [["A"], 5], "ids": ["h", "x"], "columns": ["a"] } }),
        json!({ "dataset": { "source": [["A"]], "ids": ["h"], "columns": ["a"], "extra": 1 } }),
        json!({ "dataset": { "source": [["A"], ["b"]], "ids": ["h"], "columns": ["a"] } }),
        json!({ "dataset": { "source": [["A"], ["b"]], "ids": ["h", "h"], "columns": ["a"] } }),
        json!({ "dataset": { "source": [["A", "B"], ["b", "c"]], "ids": ["h", "x"], "columns": ["a", "a"] } }),
        json!({ "dataset": { "source": [["A"], ["b"]], "ids": ["h", "x y"], "columns": ["a"] } }),
    ];
    for data in cases {
        let deck = raw_table_deck(data.clone());
        assert_that!(stored(&round_trip(&deck))).is_equal_to(&data);
        let r = DeckReplica::new(&deck, registry()).unwrap();
        // Écrit opaque : pas de colonnes ni de lignes en conteneurs.
        assert_that!(dataset(&r).get(schema::ROWS)).is_none();
    }
}

#[test]
fn forged_duplicate_row_ids_are_repaired_on_read() {
    let r = DeckReplica::new(&table_deck(data()), registry()).unwrap();
    row(&r, "de").insert("id", "fr").unwrap();
    r.doc.commit();
    let (data, issues) = read(&r);
    assert_that!(issues).is_empty();
    assert_that!(data["dataset"]["ids"]).is_equal_to(json!(["h", "fr", "r2"]));
    assert_that!(data["dataset"]["source"][2][0]).is_equal_to(json!("DE"));
}

#[test]
fn forged_twin_row_with_same_id_and_cells_is_dropped_on_read() {
    let r = DeckReplica::new(&table_deck(data()), registry()).unwrap();
    let twin = rows(&r).insert_container(2, LoroMap::new()).unwrap();
    twin.insert("id", "fr").unwrap();
    let cells = twin.insert_container(schema::CELLS, LoroMap::new()).unwrap();
    cells.insert("pays", "FR").unwrap();
    cells.insert("ventes", 1.0).unwrap();
    cells.insert("actif", true).unwrap();
    r.doc.commit();
    let (got, issues) = read(&r);
    assert_that!(issues).is_empty();
    assert_that!(got).is_equal_to(stored(&table_deck(data())));
}

#[test]
fn forged_many_rows_with_one_id_and_distinct_cells_are_kept_with_repaired_ids() {
    let r = DeckReplica::new(&table_deck(data()), registry()).unwrap();
    let list = rows(&r);
    for i in 0..2_000 {
        let m = list.push_container(LoroMap::new()).unwrap();
        m.insert("id", "fr").unwrap();
        let cells = m.insert_container(schema::CELLS, LoroMap::new()).unwrap();
        cells.insert("pays", "FR").unwrap();
        cells.insert("actif", i).unwrap();
    }
    r.doc.commit();
    let started = std::time::Instant::now();
    let (got, issues) = read(&r);
    assert_that!(started.elapsed() < std::time::Duration::from_secs(5)).is_true();
    assert_that!(issues).is_empty();
    let ids: Vec<&str> = got["dataset"]["ids"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
    assert_that!(ids.len()).is_equal_to(2_003);
    assert_that!(ids.iter().collect::<std::collections::HashSet<_>>().len()).is_equal_to(2_003);
    assert_that!(got["dataset"]["source"][2_002]).is_equal_to(json!(["FR", null, 1_999]));
}

#[test]
fn forged_duplicate_column_id_is_dropped_on_read() {
    let r = DeckReplica::new(&table_deck(data()), registry()).unwrap();
    as_list(dataset(&r).get(schema::COLUMNS)).push("pays").unwrap();
    r.doc.commit();
    let (got, issues) = read(&r);
    assert_that!(issues).is_empty();
    assert_that!(got).is_equal_to(stored(&table_deck(data())));
}

#[test]
fn forged_cell_container_reads_as_null() {
    let r = DeckReplica::new(&table_deck(data()), registry()).unwrap();
    cells(&r, "fr").insert_container("ventes", LoroMap::new()).unwrap();
    r.doc.commit();
    let (data, issues) = read(&r);
    assert_that!(issues).is_empty();
    assert_that!(data["dataset"]["source"][1]).is_equal_to(json!(["FR", null, true]));
}

#[test]
fn forged_rows_beyond_cap_are_truncated_and_reported() {
    let r = DeckReplica::new(&table_deck(data()), registry()).unwrap();
    let list = rows(&r);
    for i in list.len()..=host::MAX_ROWS {
        let m = list.push_container(LoroMap::new()).unwrap();
        m.insert("id", format!("x{i}")).unwrap();
        m.insert_container(schema::CELLS, LoroMap::new()).unwrap();
    }
    r.doc.commit();
    let (data, issues) = read(&r);
    assert_that!(issues).is_equal_to(vec!["issue-data-truncated".to_owned()]);
    assert_that!(data["dataset"]["source"].as_array().map(Vec::len)).is_equal_to(Some(host::MAX_ROWS));
    assert_that!(data["dataset"]["ids"].as_array().map(Vec::len)).is_equal_to(Some(host::MAX_ROWS));
}

#[test]
fn forged_columns_beyond_cap_are_truncated_and_reported() {
    let r = DeckReplica::new(&table_deck(data()), registry()).unwrap();
    let list = as_list(dataset(&r).get(schema::COLUMNS));
    for j in list.len()..=host::MAX_COLUMNS {
        list.push(format!("c-{j}")).unwrap();
    }
    r.doc.commit();
    let (data, issues) = read(&r);
    assert_that!(issues).is_equal_to(vec!["issue-data-truncated".to_owned()]);
    assert_that!(data["dataset"]["columns"].as_array().map(Vec::len)).is_equal_to(Some(host::MAX_COLUMNS));
    assert_that!(data["dataset"]["source"][1].as_array().map(Vec::len)).is_equal_to(Some(host::MAX_COLUMNS));
}

/// Pose une carte `{columns, rows}` sous `key` de `map`.
fn forge_marker(map: &LoroMap, key: &str) {
    let m = ok(map.insert_container(key, LoroMap::new()));
    ok(ok(m.insert_container(schema::COLUMNS, LoroMovableList::new())).push("a"));
    ok(m.insert_container(schema::ROWS, LoroMovableList::new()));
}

#[test]
fn forged_marker_outside_dataset_is_not_read_as_dataset() {
    let r = DeckReplica::new(&table_deck(data()), registry()).unwrap();
    let host = host_map(&r, "t", "data");
    forge_marker(&host, "autre");
    let nested = host.insert_container("chart", LoroMap::new()).unwrap();
    forge_marker(&nested, schema::DATASET);
    r.doc.commit();
    let (data, _) = read(&r);
    assert_that!(data["autre"]).is_equal_to(json!({}));
    assert_that!(data["chart"]["dataset"]).is_equal_to(json!({}));
    assert_that!(data["dataset"]["ids"]).is_equal_to(json!(["h", "fr", "de"]));
}

#[test]
fn forged_deeply_nested_composite_cell_reads_as_null() {
    let r = DeckReplica::new(&table_deck(data()), registry()).unwrap();
    let mut deep = LoroValue::from(1);
    for _ in 0..(read::MAX_DEPTH * 4) {
        deep = LoroValue::from(vec![deep]);
    }
    cells(&r, "fr").insert("ventes", deep).unwrap();
    r.doc.commit();
    let (data, issues) = read(&r);
    assert_that!(issues).is_empty();
    assert_that!(data["dataset"]["source"][1]).is_equal_to(json!(["FR", null, true]));
}

#[test]
fn forged_unreadable_row_is_set_aside_and_reported() {
    let r = DeckReplica::new(&table_deck(data()), registry()).unwrap();
    rows(&r).push_container(LoroMap::new()).unwrap().insert("id", "x").unwrap();
    r.doc.commit();
    let (data, issues) = read(&r);
    assert_that!(issues).is_equal_to(vec!["issue-unreadable-row".to_owned()]);
    assert_that!(oku_slides_i18n::Localizer::new(Some("fr")).tr("issue-unreadable-row"))
        .is_equal_to("ligne de tableau illisible, mise de côté".to_owned());
    assert_that!(data["dataset"]["ids"]).is_equal_to(json!(["h", "fr", "de"]));
}

#[test]
fn ragged_merged_rows_read_with_nulls() {
    let r = DeckReplica::new(&table_deck(data()), registry()).unwrap();
    cells(&r, "de").delete("pays").unwrap();
    r.doc.commit();
    let (data, issues) = read(&r);
    assert_that!(issues).is_empty();
    assert_that!(data["dataset"]["source"][2]).is_equal_to(json!([null, null, false]));
}

#[test]
fn repair_duplicate_and_invalid_ids_uses_position_then_suffix() {
    let got = host::repair(vec![Some("a".into()), Some("a".into()), Some("r1".into()), Some("x y".into()), None], 'r');
    assert_that!(got).is_equal_to(["a", "r1-1", "r1", "r3", "r4"].iter().map(|s| (*s).to_owned()).collect::<Vec<_>>());
}

#[test]
fn repair_valid_unique_ids_keeps_them() {
    let got = host::repair(vec![Some("h".into()), Some("fr".into())], 'r');
    assert_that!(got).is_equal_to(vec!["h".to_owned(), "fr".to_owned()]);
}

#[test]
fn dataset_beyond_read_caps_is_written_opaque_and_round_trips_unchanged() {
    let rows = host::MAX_ROWS + 1;
    let source: Vec<Value> = (0..rows).map(|i| json!([format!("r{i}")])).collect();
    let ids: Vec<String> = (0..rows).map(|i| format!("r{i}")).collect();
    let deck = raw_table_deck(json!({ "dataset": { "source": source, "ids": ids, "columns": ["a"] } }));
    assert_that!(round_trip(&deck)).is_equal_to(&deck);
    let r = DeckReplica::new(&deck, registry()).unwrap();
    assert_that!(dataset(&r).get(schema::ROWS)).is_none();
    let (back, issues) = r.deck();
    assert_that!(issues).is_empty();
    assert_that!(back).is_equal_to(&deck);
}

#[test]
fn forged_dataset_beyond_cell_budget_is_truncated_and_reported() {
    let r = DeckReplica::new(&table_deck(data()), registry()).unwrap();
    let columns = as_list(dataset(&r).get(schema::COLUMNS));
    for j in columns.len()..host::MAX_COLUMNS {
        columns.push(format!("c-{j}")).unwrap();
    }
    let list = rows(&r);
    for i in list.len()..host::MAX_ROWS {
        let m = list.push_container(LoroMap::new()).unwrap();
        m.insert("id", format!("x{i}")).unwrap();
        m.insert_container(schema::CELLS, LoroMap::new()).unwrap();
    }
    r.doc.commit();
    let (data, issues) = read(&r);
    assert_that!(issues).is_equal_to(vec!["issue-data-truncated".to_owned()]);
    let source = data["dataset"]["source"].as_array().unwrap();
    let cells: usize = source.iter().map(|row| row.as_array().map_or(0, Vec::len)).sum();
    assert_that!(cells <= host::MAX_CELLS).is_true();
    assert_that!(source.len()).is_equal_to(host::MAX_CELLS / host::MAX_COLUMNS);
    assert_that!(data["dataset"]["ids"].as_array().map(Vec::len)).is_equal_to(Some(source.len()));
}

#[test]
fn dataset_beyond_cell_budget_is_written_opaque_and_round_trips_unchanged() {
    let (rows, cols) = (host::MAX_CELLS / host::MAX_COLUMNS + 1, host::MAX_COLUMNS);
    let columns: Vec<String> = (0..cols).map(|j| format!("c{j}")).collect();
    let source: Vec<Value> = (0..rows).map(|_| Value::Array(vec![Value::Null; cols])).collect();
    let ids: Vec<String> = (0..rows).map(|i| format!("r{i}")).collect();
    let deck = raw_table_deck(json!({ "dataset": { "source": source, "ids": ids, "columns": columns } }));
    let r = DeckReplica::new(&deck, registry()).unwrap();
    assert_that!(dataset(&r).get(schema::ROWS)).is_none();
    let (back, issues) = r.deck();
    assert_that!(issues).is_empty();
    assert_that!(back).is_equal_to(&deck);
}
