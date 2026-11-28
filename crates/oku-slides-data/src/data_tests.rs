use speculoos::prelude::*;

use super::*;
use crate::DataErrorKind;
use crate::limits::MAX_SQL_BYTES;
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
    let data = Data { name: "ventes".to_owned(), label: None, inline: false, source: Source::Manual(small()) };
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
        source: Source::Manual(small()),
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
        source: Source::Manual(small()),
    })
    .unwrap();
    json["extra"] = serde_json::json!(1);
    assert_that!(serde_json::from_value::<Data>(json)).is_err();
}

fn query(sql: &str) -> Data {
    Data { name: "top".to_owned(), label: None, inline: false, source: Source::Query { sql: sql.to_owned() } }
}

#[test]
fn query_data_json_round_trips_without_table() {
    let data = query("SELECT * FROM ventes");
    let json = serde_json::to_value(&data).unwrap();
    assert_that!(json)
        .is_equal_to(serde_json::json!({ "name": "top", "kind": "query", "sql": "SELECT * FROM ventes" }));
    let back: Data = serde_json::from_value(json).unwrap();
    assert_that!(back.kind()).is_equal_to(DataKind::Query);
    assert_that!(back.sql()).is_equal_to(Some("SELECT * FROM ventes"));
    assert_that!(back.table()).is_none();
    assert_that!(back).is_equal_to(data);
}

#[test]
fn manual_data_reads_its_table_and_no_sql() {
    let mut data = Data { name: "ventes".to_owned(), label: None, inline: false, source: Source::Manual(small()) };
    assert_that!(data.kind()).is_equal_to(DataKind::Manual);
    assert_that!(data.sql()).is_none();
    assert_that!(data.table()).is_equal_to(Some(&small()));
    assert_that!(data.table_mut().is_some()).is_true();
    assert_that!(query("SELECT 1").table_mut().is_none()).is_true();
}

/// Each JSON whose source does not match its kind.
fn mismatched() -> Vec<(&'static str, serde_json::Value)> {
    let table = serde_json::to_value(small()).unwrap();
    vec![
        ("table with kind query", serde_json::json!({ "name": "t", "kind": "query", "table": table })),
        ("sql with kind manual", serde_json::json!({ "name": "t", "kind": "manual", "sql": "SELECT 1" })),
        ("both", serde_json::json!({ "name": "t", "kind": "query", "table": table, "sql": "SELECT 1" })),
        ("neither", serde_json::json!({ "name": "t", "kind": "query" })),
        ("manual without table", serde_json::json!({ "name": "t", "kind": "manual" })),
    ]
}

#[test]
fn data_with_table_and_kind_query_is_refused() {
    for (case, json) in mismatched() {
        assert_that!(serde_json::from_value::<Data>(json.clone())).named(case).is_err();
        let repr: DataRepr = serde_json::from_value(json).unwrap();
        let error = Data::try_from(repr).unwrap_err();
        assert_that!(error.kind()).named(case).is_equal_to(DataErrorKind::SourceMismatch);
    }
}

#[test]
fn source_mismatch_localizes_with_its_key_in_french() {
    use oku_slides_i18n::{Localize, Localizer};
    let json = serde_json::json!({ "name": "t", "kind": "manual", "sql": "SELECT 1" });
    let error = Data::try_from(serde_json::from_value::<DataRepr>(json).unwrap()).unwrap_err();
    let fr = Localizer::new(Some("fr"));
    assert_that!(error.localize(&fr)).is_equal_to(fr.tr("data-source-mismatch"));
    assert_that!(error.localize(&fr)).is_not_equal_to(error.localize(&Localizer::new(Some("en"))));
}

#[test]
fn sql_over_max_bytes_is_refused_while_parsing() {
    let at_max = serde_json::json!({ "name": "t", "kind": "query", "sql": "x".repeat(MAX_SQL_BYTES) });
    assert_that!(serde_json::from_value::<Data>(at_max)).is_ok();
    let over = format!(r#"{{"name":"t","kind":"query","sql":"{}"}}"#, "é".repeat(MAX_SQL_BYTES / 2 + 1));
    let error = serde_json::from_str::<Data>(&over).unwrap_err();
    assert_that!(error.to_string()).contains(MAX_SQL_BYTES.to_string());
}

#[test]
fn inline_query_data_is_refused_in_french() {
    use oku_slides_i18n::{Localize, Localizer};
    let json = serde_json::json!({ "name": "t", "inline": true, "kind": "query", "sql": "SELECT 1" });
    assert_that!(serde_json::from_value::<Data>(json.clone())).is_err();
    let error = Data::try_from(serde_json::from_value::<DataRepr>(json).unwrap()).unwrap_err();
    assert_that!(error.kind()).is_equal_to(DataErrorKind::InlineQuery);
    let fr = Localizer::new(Some("fr"));
    assert_that!(error.localize(&fr)).is_equal_to(fr.tr("data-inline-query"));
    assert_that!(fr.has("data-inline-query")).is_true();
}
