use serde_json::json;
use speculoos::prelude::*;

use crate::{ColumnKind, ListColumn, SubColumn, SubList, SubListFault};

#[test]
fn list_column_of_a_nested_list_carries_its_sub_columns_and_bounds() {
    let texte = SubColumn { key: "texte".into(), label: "Tâche".into(), kind: ColumnKind::Text, options: vec![] };
    let c = ListColumn {
        key: "taches".into(),
        label: "Tâches".into(),
        kind: ColumnKind::List,
        options: vec![],
        list: Some(Box::new(SubList::new(vec![texte], 0, Some(8), Some("tâche".into())).unwrap())),
    };
    let v = serde_json::to_value(&c).unwrap();
    assert_that!(v.clone()).is_equal_to(json!({
        "key": "taches", "label": "Tâches", "kind": "list", "options": [],
        "list": { "columns": [{ "key": "texte", "label": "Tâche", "kind": "text", "options": [] }],
            "min": 0, "max": 8, "item_label": "tâche" }
    }));
    assert_that!(serde_json::from_value::<ListColumn>(v).unwrap()).is_equal_to(c);
}

#[test]
fn sub_list_with_a_list_column_is_refused() {
    let deep = json!({ "columns": [{ "key": "x", "label": "x", "kind": "list", "options": [] }], "min": 0 });
    assert_that!(serde_json::from_value::<SubList>(deep).is_err()).is_true();
    let col = SubColumn { key: "x".into(), label: "x".into(), kind: ColumnKind::List, options: vec![] };
    assert_that!(SubList::new(vec![col], 0, None, None)).is_equal_to(Err(SubListFault::NestedList));
}

#[test]
fn sub_column_with_its_own_list_is_refused() {
    let deep = json!({ "columns": [{ "key": "x", "label": "x", "kind": "text", "options": [],
        "list": { "columns": [], "min": 0 } }], "min": 0 });
    assert_that!(serde_json::from_value::<SubList>(deep).is_err()).is_true();
}

#[test]
fn sub_list_with_max_below_min_is_refused() {
    assert_that!(serde_json::from_value::<SubList>(json!({ "columns": [], "min": 3, "max": 2 })).is_err()).is_true();
    assert_that!(SubList::new(vec![], 3, Some(2), None)).is_equal_to(Err(SubListFault::MaxBelowMin));
}
