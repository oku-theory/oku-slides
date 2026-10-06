use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use oku_slides_core::{ParamSchema, ParamType, ParamValue};

use super::{coerce, schema_default};

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
