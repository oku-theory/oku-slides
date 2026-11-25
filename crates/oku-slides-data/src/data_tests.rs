use speculoos::prelude::*;

use super::*;
use crate::test_support::small;

#[test]
fn data_id_deserialize_rejects_invalid_id() {
    for id in ["", "a/b", "é", &"a".repeat(65)] {
        let parsed = serde_json::from_value::<DataId>(serde_json::Value::String(id.to_owned()));
        assert_that!(parsed).named(id).is_err();
    }
    let parsed = serde_json::from_value::<DataId>(serde_json::Value::String("ventes-1".to_owned()));
    assert_that!(parsed.map(|id| id.as_str().to_owned())).is_ok_containing("ventes-1".to_owned());
}

#[test]
fn data_json_round_trip_omits_none_label_false_inline_and_no_base() {
    let data = Data { name: "ventes".to_owned(), label: None, inline: false, kind: DataKind::Manual, table: small() };
    let json = serde_json::to_value(&data).unwrap();
    let object = json.as_object().unwrap();
    assert_that!(object.contains_key("label")).is_false();
    assert_that!(object.contains_key("inline")).is_false();
    assert_that!(object["table"].as_object().unwrap().contains_key("base")).is_false();
    assert_that!(object["kind"]).is_equal_to(serde_json::json!("manual"));
    let back: Data = serde_json::from_value(json).unwrap();
    assert_that!(back).is_equal_to(data);
}

#[test]
fn data_json_with_label_and_inline_round_trips() {
    let data = Data {
        name: "ventes".to_owned(),
        label: Some("Ventes".to_owned()),
        inline: true,
        kind: DataKind::Manual,
        table: small(),
    };
    let back: Data = serde_json::from_str(&serde_json::to_string(&data).unwrap()).unwrap();
    assert_that!(back).is_equal_to(data);
}

#[test]
fn data_json_with_unknown_field_fails_to_parse() {
    let mut json = serde_json::to_value(Data {
        name: "ventes".to_owned(),
        label: None,
        inline: false,
        kind: DataKind::Manual,
        table: small(),
    })
    .unwrap();
    json["extra"] = serde_json::json!(1);
    assert_that!(serde_json::from_value::<Data>(json)).is_err();
}
