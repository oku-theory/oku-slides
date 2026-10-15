use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use super::*;
use crate::test_registry::{PLAN, with_list};
use crate::{OpErrorKind, WidgetSchema};

fn plan() -> WidgetSchema {
    serde_json::from_str(PLAN).unwrap_or_else(|e| panic!("PLAN : {e}"))
}

fn etapes(w: &WidgetSchema) -> ListSpec<'_> {
    let p = w.param("etapes").unwrap();
    ListSpec::of(&p.name, &p.ty).unwrap()
}

/// Deux étapes : `a` à deux tâches (`t1`, `t2`), `b` sans tâche.
fn items() -> Vec<ListItem> {
    serde_json::from_value(json!([
        { "id": "a", "titre": "A", "taches": [{ "id": "t1", "texte": "un" }, { "id": "t2", "texte": "deux" }] },
        { "id": "b", "titre": "B" }
    ]))
    .unwrap()
}

fn task_ids(items: &[ListItem], step: &str) -> Vec<String> {
    let it = items.iter().find(|i| i.id == step).unwrap();
    it.values.list("taches").unwrap_or_default().iter().map(|t| t.id.clone()).collect()
}

fn apply(path: &str, change: Value) -> Result<Vec<ListItem>, OpError> {
    let w = plan();
    let mut l = items();
    let change: ListOp = serde_json::from_value(change).unwrap();
    ListPath::parse(path).unwrap().apply(&mut l, &etapes(&w), change)?;
    Ok(l)
}

#[rstest]
#[case("etapes", Some(("etapes", None)))]
#[case("etapes/a/taches", Some(("etapes", Some(("a", "taches")))))]
#[case("etapes/a", None)]
#[case("etapes/a/taches/t1", None)]
#[case("etapes//taches", None)]
#[case("", None)]
fn parse_path_reads_param_and_nested_list(#[case] path: &str, #[case] expected: Option<(&str, Option<(&str, &str)>)>) {
    assert_that!(ListPath::parse(path).map(|p| (p.param, p.nested))).is_equal_to(expected);
}

#[test]
fn schema_of_a_nested_path_is_the_list_column() {
    let w = plan();
    let s = ListPath::parse("etapes/x/taches").unwrap().schema(&w.params).map(|c| c.name.as_str());
    assert_that!(s).is_equal_to(Some("taches"));
}

#[test]
fn schema_of_a_path_to_a_text_column_is_none() {
    let w = plan();
    assert_that!(ListPath::parse("etapes/x/titre").unwrap().schema(&w.params)).is_none();
}

#[test]
fn items_of_an_absent_nested_list_is_empty() {
    let mut p = Params::default();
    p.set("etapes", ParamValue::List(items()));
    assert_that!(ListPath::parse("etapes/b/taches").unwrap().items(&p).map(<[_]>::len)).is_equal_to(Some(0));
    assert_that!(ListPath::parse("etapes/z/taches").unwrap().items(&p)).is_none();
}

#[test]
fn apply_nested_insert_fills_the_nested_column_defaults() {
    let l = apply("etapes/a/taches", json!({ "op": "insert", "index": 1, "id": "n" })).unwrap();
    assert_that!(task_ids(&l, "a")).is_equal_to(vec!["t1".to_owned(), "n".into(), "t2".into()]);
    let n = &l[0].values.list("taches").unwrap()[1];
    assert_that!(n.values.text("texte")).is_equal_to(Some("Tâche"));
}

#[test]
fn apply_nested_insert_into_an_absent_list_creates_it() {
    let l = apply("etapes/b/taches", json!({ "op": "insert", "id": "n" })).unwrap();
    assert_that!(task_ids(&l, "b")).is_equal_to(vec!["n".to_owned()]);
}

#[test]
fn apply_nested_move_reorders_only_the_nested_list() {
    let l = apply("etapes/a/taches", json!({ "op": "move", "item": "t2", "to": 0 })).unwrap();
    assert_that!(task_ids(&l, "a")).is_equal_to(vec!["t2".to_owned(), "t1".into()]);
    assert_that!(l.iter().map(|i| i.id.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["a", "b"]);
}

#[test]
fn apply_nested_remove_of_the_last_item_removes_the_column() {
    let l = apply("etapes/a/taches", json!({ "op": "remove", "item": "t1" })).unwrap();
    let l = {
        let w = plan();
        let mut l = l;
        ListPath::parse("etapes/a/taches")
            .unwrap()
            .apply(&mut l, &etapes(&w), ListOp::Remove { item: "t2".into() })
            .unwrap();
        l
    };
    assert_that!(l[0].values.get("taches")).is_none();
}

#[test]
fn apply_nested_set_field_converts_the_value() {
    let l =
        apply("etapes/a/taches", json!({ "op": "set_field", "item": "t2", "field": "fait", "value": true })).unwrap();
    let t2 = &l[0].values.list("taches").unwrap()[1];
    assert_that!(t2.values.get("fait")).is_equal_to(Some(&ParamValue::Value(json!(true))));
}

#[test]
fn apply_nested_insert_beyond_the_nested_max_is_list_bound() {
    let w = plan();
    let mut l = items();
    let path = ListPath::parse("etapes/a/taches").unwrap();
    path.apply(&mut l, &etapes(&w), ListOp::Insert { index: None, id: "n".into(), values: Map::new() }).unwrap();
    let e = path.apply(&mut l, &etapes(&w), ListOp::Insert { index: None, id: "m".into(), values: Map::new() });
    assert_that!(e.map_err(|e| e.kind())).is_equal_to(Err(OpErrorKind::ListBound));
}

#[rstest]
#[case("etapes/z/taches", OpErrorKind::ItemNotFound)]
#[case("etapes/a/inconnue", OpErrorKind::UnknownParam)]
#[case("etapes/a/titre", OpErrorKind::ParamType)]
fn apply_nested_bad_path_fails_and_keeps_items(#[case] path: &str, #[case] kind: OpErrorKind) {
    let e = apply(path, json!({ "op": "remove", "item": "t1" })).unwrap_err();
    assert_that!(e.kind()).is_equal_to(kind);
}

#[test]
fn apply_insert_of_a_top_item_seeds_its_nested_list_default() {
    let l = apply("etapes", json!({ "op": "insert", "id": "c" })).unwrap();
    let c = l.iter().find(|i| i.id == "c").unwrap();
    let tasks = c.values.list("taches").unwrap();
    assert_that!(tasks.iter().map(|t| t.id.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["d0"]);
    assert_that!(tasks[0].values.text("texte")).is_equal_to(Some("Première tâche"));
}

#[test]
fn default_items_of_a_nested_default_assigns_d_ids_at_both_levels() {
    let w = plan();
    let d = crate::default_items(w.param("etapes").unwrap()).unwrap();
    assert_that!(d[0].id.as_str()).is_equal_to("d0");
    assert_that!(task_ids(&d, "d0")).is_equal_to(vec!["d0".to_owned(), "d1".into()]);
}

#[test]
fn plan_widget_is_in_the_list_registry() {
    assert_that!(with_list().get("plan")).is_some();
}

#[test]
fn set_param_of_a_list_never_seeds_an_absent_nested_list() {
    let w = plan();
    let ty = &w.param("etapes").unwrap().ty;
    let v = crate::param_value(ty, "etapes", json!([{ "id": "a", "titre": "A" }])).unwrap();
    let Some(ParamValue::List(l)) = v else { panic!("liste attendue") };
    assert_that!(l[0].values.get("taches")).is_none();
}

#[test]
fn set_param_of_a_list_refuses_nested_items_without_id() {
    let w = plan();
    let ty = &w.param("etapes").unwrap().ty;
    let r = crate::param_value(ty, "etapes", json!([{ "id": "a", "taches": [{ "texte": "x" }] }]));
    assert_that!(r.map_err(|e| e.kind())).is_equal_to(Err(OpErrorKind::InvalidValue));
}

// Bornes des listes imbriquées (suites de revue, t-495).

#[test]
fn max_items_is_capped_at_the_core_bound() {
    let ty = ParamType::List {
        item: vec![ParamSchema { name: "t".into(), label: None, ty: ParamType::Plain, default: None, bind: None }],
        item_label: None,
        min: 0,
        max: Some(crate::LIST_ITEMS_MAX * 4),
    };
    assert_that!(ListSpec::of("l", &ty).unwrap().max_items()).is_equal_to(crate::LIST_ITEMS_MAX);
}

/// `n` étapes de 3 tâches chacune (le max de `taches` du widget `plan`).
fn full_steps(n: usize) -> Vec<ListItem> {
    (0..n)
        .map(|i| {
            let tasks: Vec<Value> = (0..3).map(|j| json!({ "id": format!("t{j}") })).collect();
            serde_json::from_value(json!({ "id": format!("e{i}"), "taches": tasks })).unwrap()
        })
        .collect()
}

#[test]
fn apply_nested_insert_beyond_the_total_nested_cap_is_list_bound_and_keeps_items() {
    let w = plan();
    let steps = crate::NESTED_ITEMS_MAX / 3;
    let mut l = full_steps(steps);
    l.push(serde_json::from_value(json!({ "id": "last" })).unwrap());
    let before = l.clone();
    let path = ListPath::parse("etapes/last/taches").unwrap();
    for id in ["a", "b"] {
        let _ = path.apply(&mut l, &etapes(&w), ListOp::Insert { index: None, id: id.into(), values: Map::new() });
    }
    let total: usize = l.iter().map(|i| i.values.list("taches").map_or(0, <[_]>::len)).sum();
    assert_that!(total).is_less_than_or_equal_to(crate::NESTED_ITEMS_MAX);
    let e = path.apply(&mut l, &etapes(&w), ListOp::Insert { index: None, id: "z".into(), values: Map::new() });
    assert_that!(e.map_err(|e| e.kind())).is_equal_to(Err(OpErrorKind::ListBound));
    assert_that!(l.len()).is_equal_to(before.len());
}

#[test]
fn set_param_beyond_the_total_nested_cap_is_list_bound() {
    let w = plan();
    let ty = &w.param("etapes").unwrap().ty;
    let steps = serde_json::to_value(full_steps(crate::NESTED_ITEMS_MAX / 3 + 1)).unwrap();
    let r = crate::param_value(ty, "etapes", steps);
    assert_that!(r.map_err(|e| e.kind())).is_equal_to(Err(OpErrorKind::ListBound));
}

#[test]
fn set_param_with_a_slash_in_a_nested_item_id_is_refused() {
    let w = plan();
    let ty = &w.param("etapes").unwrap().ty;
    let r = crate::param_value(ty, "etapes", json!([{ "id": "a", "taches": [{ "id": "x/y" }] }]));
    assert_that!(r.map_err(|e| e.kind())).is_equal_to(Err(OpErrorKind::InvalidValue));
}
