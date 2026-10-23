use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use oku_slides_core::{ParamSchema, ParamType, ParamValue, kinds};
use oku_slides_i18n::Localizer;

use super::{coerce, kind_key, kind_name, schema_default};

fn schema(ty: ParamType, default: Option<Value>) -> ParamSchema {
    ParamSchema { name: "p".into(), label: None, ty, default, bind: None }
}

#[rstest]
#[case(ParamType::Flag, json!("true"), Some(json!(true)))]
#[case(ParamType::Flag, json!("false"), Some(json!(false)))]
#[case(ParamType::Number { min: None, max: None }, json!(" 4.5 "), Some(json!(4.5)))]
#[case(ParamType::Number { min: None, max: None }, json!("abc"), Some(json!("abc")))]
#[case(ParamType::Plain, json!("   "), None)]
#[case(ParamType::Plain, Value::Null, None)]
#[case(ParamType::Plain, json!("Lyon"), Some(json!("Lyon")))]
fn coerce_converts_select_and_text_input(#[case] ty: ParamType, #[case] input: Value, #[case] expected: Option<Value>) {
    assert_that!(coerce(&schema(ty, None), input)).is_equal_to(expected);
}

#[test]
fn schema_default_plain_returns_the_default_text() {
    assert_that!(schema_default(&schema(ParamType::Plain, Some(json!("Titre")))))
        .is_equal_to(Some(ParamValue::Text("Titre".into())));
}

#[test]
fn schema_default_null_or_absent_returns_none() {
    assert_that!(schema_default(&schema(ParamType::Plain, Some(Value::Null)))).is_none();
    assert_that!(schema_default(&schema(ParamType::Plain, None))).is_none();
}

#[rstest]
#[case(kinds::base::CARD, "Carte")]
#[case(kinds::base::TEXT, "Texte")]
#[case(kinds::base::CHART, "Graphique")]
#[case(kinds::base::TABLE, "Tableau")]
#[case(kinds::essentiel::CONTENT, "Contenu")]
#[case("content", "content")]
#[case("acme/card", "acme/card")]
#[case("base-x/card", "base-x/card")]
fn kind_name_in_french(#[case] kind: &str, #[case] expected: &str) {
    assert_that!(kind_name(kind, None, &Localizer::new(Some("fr")))).is_equal_to(expected.to_owned());
}

#[rstest]
#[case(kinds::essentiel::CONTENT, Some("session-widget-kind-essentiel-content"))]
#[case(kinds::base::CARD, Some("session-widget-kind-base-card"))]
#[case("acme/card", None)]
#[case("content", None)]
fn widget_key_only_for_builtin_kinds(#[case] kind: &str, #[case] expected: Option<&str>) {
    let key = kind_key(kind).map(|k| format!("session-widget-kind-{k}"));
    assert_that!(key.as_deref()).is_equal_to(expected);
}

#[test]
fn kind_name_of_essentiel_title_is_localized_in_french() {
    let fr = kind_name(kinds::essentiel::TITLE, None, &Localizer::new(Some("fr")));
    assert_that!(fr).is_not_equal_to(kinds::essentiel::TITLE.to_owned());
}
