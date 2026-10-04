use super::*;
use crate::{ListItem, ParamSchema, ParamType, Params};
use serde_json::{Map, json};
use speculoos::prelude::*;

fn column(name: &str, ty: ParamType, default: Option<serde_json::Value>) -> ParamSchema {
    ParamSchema { name: name.into(), label: None, ty, default, bind: None }
}
fn schema(min: usize, max: Option<usize>) -> ParamType {
    ParamType::List {
        item: vec![
            column("date", ParamType::Plain, None),
            column("texte", ParamType::Plain, Some(json!("Nouvelle étape"))),
            column("on", ParamType::Flag, None),
        ],
        item_label: None,
        min,
        max,
    }
}
fn items(ids: &[&str]) -> Vec<ListItem> {
    ids.iter().map(|id| ListItem { id: (*id).into(), values: Params::default() }).collect()
}
fn ids(l: &[ListItem]) -> Vec<&str> {
    l.iter().map(|i| i.id.as_str()).collect()
}
fn insert(index: Option<usize>, id: &str) -> ListOp {
    ListOp::Insert { index, id: id.into(), values: Map::new() }
}

#[test]
fn apply_insert_at_index_fills_column_defaults() {
    let ty = schema(0, None);
    let spec = ListSpec::of("etapes", &ty).unwrap();
    let mut l = items(&["a", "b"]);
    insert(Some(1), "n").apply(&mut l, &spec).unwrap();
    assert_that!(ids(&l)).is_equal_to(vec!["a", "n", "b"]);
    assert_that!(l[1].values.text("texte")).is_equal_to(Some("Nouvelle étape"));
}

#[test]
fn apply_insert_without_index_appends() {
    let ty = schema(0, None);
    let mut l = items(&["a"]);
    insert(None, "n").apply(&mut l, &ListSpec::of("l", &ty).unwrap()).unwrap();
    assert_that!(ids(&l)).is_equal_to(vec!["a", "n"]);
}

#[rstest::rstest]
#[case(insert(None, "a"))]
#[case(insert(None, ""))]
#[case(insert(None, "x/y"))]
#[case(insert(Some(9), "n"))]
fn apply_insert_invalid_fails_and_keeps_items(#[case] op: ListOp) {
    let ty = schema(0, None);
    let mut l = items(&["a"]);
    assert_that!(op.apply(&mut l, &ListSpec::of("l", &ty).unwrap())).is_err();
    assert_that!(ids(&l)).is_equal_to(vec!["a"]);
}

#[test]
fn apply_insert_beyond_max_fails() {
    let ty = schema(0, Some(1));
    let mut l = items(&["a"]);
    assert_that!(insert(None, "n").apply(&mut l, &ListSpec::of("l", &ty).unwrap())).is_err();
}

#[test]
fn apply_remove_below_min_fails() {
    let ty = schema(1, None);
    let mut l = items(&["a"]);
    let r = ListOp::Remove { item: "a".into() }.apply(&mut l, &ListSpec::of("l", &ty).unwrap());
    assert_that!(r).is_err();
}

#[test]
fn apply_remove_unknown_item_is_item_not_found() {
    let ty = schema(0, None);
    let mut l = items(&["a"]);
    let e = ListOp::Remove { item: "z".into() }.apply(&mut l, &ListSpec::of("l", &ty).unwrap()).unwrap_err();
    assert_that!(e.kind()).is_equal_to(crate::OpErrorKind::ItemNotFound);
}

#[test]
fn apply_move_puts_the_item_at_its_new_index() {
    let ty = schema(0, None);
    let mut l = items(&["a", "b", "c"]);
    ListOp::Move { item: "a".into(), to: 2 }.apply(&mut l, &ListSpec::of("l", &ty).unwrap()).unwrap();
    assert_that!(ids(&l)).is_equal_to(vec!["b", "c", "a"]);
}

#[test]
fn apply_set_field_converts_by_column_type() {
    let ty = schema(0, None);
    let spec = ListSpec::of("l", &ty).unwrap();
    let mut l = items(&["a"]);
    ListOp::SetField { item: "a".into(), field: "date".into(), value: json!(" 2019 ") }.apply(&mut l, &spec).unwrap();
    assert_that!(l[0].values.text("date")).is_equal_to(Some("2019"));
    let bad = ListOp::SetField { item: "a".into(), field: "on".into(), value: json!("oui") };
    assert_that!(bad.apply(&mut l, &spec)).is_err();
    let unknown = ListOp::SetField { item: "a".into(), field: "zz".into(), value: json!("x") };
    assert_that!(unknown.apply(&mut l, &spec)).is_err();
}

#[test]
fn default_items_gives_deterministic_ids() {
    let p = ParamSchema {
        name: "l".into(),
        label: None,
        ty: schema(0, None),
        default: Some(json!([{ "date": "2019" }, { "date": "2021" }])),
        bind: None,
    };
    let l = default_items(&p).unwrap();
    assert_that!(ids(&l)).is_equal_to(vec!["d0", "d1"]);
    assert_that!(l[1].values.text("date")).is_equal_to(Some("2021"));
}

#[test]
fn list_value_null_clears_and_duplicates_fail() {
    let ty = schema(0, None);
    let spec = ListSpec::of("l", &ty).unwrap();
    assert_that!(list_value(&spec, json!(null)).unwrap()).is_none();
    assert_that!(list_value(&spec, json!([{ "id": "a" }, { "id": "a" }]))).is_err();
    assert_that!(list_value(&spec, json!([{ "date": "x" }]))).is_err();
}
