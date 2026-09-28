use serde_json::json;
use speculoos::prelude::*;

use crate::{Element, Paragraph, ParamValue, Params};

fn params(v: serde_json::Value) -> Params {
    serde_json::from_value(v).unwrap()
}

#[test]
fn deserialize_string_gives_text() {
    let p = params(json!({ "title": "Titre" }));
    assert_that!(p.get("title")).is_equal_to(Some(&ParamValue::Text("Titre".into())));
}

#[test]
fn deserialize_paragraphs_gives_rich() {
    let p = params(json!({ "body": [{ "runs": [{ "t": "a" }] }] }));
    assert_that!(p.rich("body")).is_equal_to(Some(&[Paragraph::plain("a")][..]));
}

#[test]
fn deserialize_elements_gives_slot() {
    let p = params(json!({ "children": [{ "id": "e1", "kind": "text" }] }));
    assert_that!(p.slot("children").map(|s| s[0].id.as_str())).is_equal_to(Some("e1"));
}

#[test]
fn deserialize_empty_array_is_an_empty_rich_and_an_empty_slot() {
    let p = params(json!({ "x": [] }));
    assert_that!(p.rich("x")).is_equal_to(Some(&[][..]));
    assert_that!(p.slot("x")).is_equal_to(Some(&[][..]));
}

#[test]
fn deserialize_other_values_gives_value() {
    let p = params(json!({ "highlight": true, "grid": { "columns": 2, "rows": 1 } }));
    assert_that!(p.get("highlight")).is_equal_to(Some(&ParamValue::Value(json!(true))));
    assert_that!(p.value("grid")).is_equal_to(Some(&json!({ "columns": 2, "rows": 1 })));
}

#[test]
fn serialize_round_trips_every_shape() {
    let v = json!({ "a": "t", "b": [{ "runs": [{ "t": "x" }] }], "c": [{ "id": "e", "kind": "text" }], "d": 3 });
    let p = params(v.clone());
    assert_that!(serde_json::to_value(&p).unwrap()).is_equal_to(v);
}

#[test]
fn deserialize_paragraph_is_never_a_slot() {
    // Garde-fou : si `Paragraph` gagne un jour `id` et `kind`, ce test casse.
    let p = params(json!({ "body": [{ "runs": [{ "t": "a" }], "list": "bullet" }] }));
    assert_that!(matches!(p.get("body"), Some(ParamValue::Rich(_)))).is_true();
}

#[test]
fn deserialize_elements_missing_an_id_gives_value_and_round_trips() {
    // Rien ne se perd : un tableau d'objets dont certains ont `kind` mais pas
    // `id` n'est ni une liste d'éléments ni du texte riche ; il reste tel quel.
    let v = json!({ "children": [{ "id": "e1", "kind": "text" }, { "kind": "text" }] });
    let p = params(v.clone());
    assert_that!(matches!(p.get("children"), Some(ParamValue::Value(_)))).is_true();
    assert_that!(serde_json::to_value(&p).unwrap()).is_equal_to(v);
}

#[rstest::rstest]
#[case::foreign_objects(json!([{ "x": 1 }]))]
#[case::foreign_run_key(json!([{ "runs": [{ "t": "a", "x": 1 }] }]))]
#[case::runs_not_a_list(json!([{ "runs": "a" }]))]
#[case::mixed_with_a_paragraph(json!([{ "runs": [{ "t": "a" }] }, { "x": 1 }]))]
fn deserialize_array_of_non_paragraph_objects_gives_value_and_round_trips(#[case] value: serde_json::Value) {
    let v = json!({ "data": value });
    let p = params(v.clone());
    assert_that!(matches!(p.get("data"), Some(ParamValue::Value(_)))).is_true();
    assert_that!(serde_json::to_value(&p).unwrap()).is_equal_to(v);
}

#[test]
fn slot_mut_turns_an_absent_or_empty_param_into_a_slot() {
    let mut p = Params::default();
    p.slot_mut("children").unwrap().push(Element::new("e1", "text"));
    assert_that!(p.slot("children").map(<[Element]>::len)).is_equal_to(Some(1));
}

#[test]
fn json_schema_is_a_free_object_and_keeps_ops_schema_generable() {
    let schema = serde_json::to_value(schemars::schema_for!(Params)).unwrap();
    assert_that!(schema.get("type")).is_equal_to(Some(&json!("object")));
    assert_that!(schema.get("additionalProperties")).is_equal_to(Some(&json!(true)));
    assert_that!(crate::ops_schema().is_ok()).is_true();
}
