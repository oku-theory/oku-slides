use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::param_value;
use crate::error::OpErrorKind;
use crate::model::Paragraph;
use crate::params::ParamValue;
use crate::widget::{HostType, ParamType, SlotMode};

fn choice() -> ParamType {
    ParamType::Choice { values: vec!["cover".into(), "contain".into()] }
}

fn number() -> ParamType {
    ParamType::Number { min: Some(1.0), max: Some(10.0) }
}

fn chart() -> ParamType {
    ParamType::Host { host: HostType::Chart }
}

fn slot() -> ParamType {
    ParamType::Slot { mode: SlotMode::Flow, accepts: None }
}

#[rstest]
#[case::plain_null(ParamType::Plain, json!(null))]
#[case::plain_blank(ParamType::Plain, json!("  "))]
#[case::asset_empty(ParamType::Asset, json!(""))]
#[case::choice_null(choice(), json!(null))]
#[case::rich_null(ParamType::Rich, json!(null))]
#[case::rich_empty(ParamType::Rich, json!([]))]
#[case::flag_null(ParamType::Flag, json!(null))]
#[case::number_null(number(), json!(null))]
#[case::grid_null(ParamType::Grid, json!(null))]
fn param_value_empty_returns_none(#[case] ty: ParamType, #[case] value: Value) {
    assert_that!(param_value(&ty, "p", value).unwrap()).is_none();
}

#[rstest]
#[case::plain_trimmed(ParamType::Plain, json!(" Titre "), ParamValue::Text("Titre".into()))]
#[case::asset(ParamType::Asset, json!("sha256:ab"), ParamValue::Text("sha256:ab".into()))]
#[case::choice(choice(), json!("contain"), ParamValue::Text("contain".into()))]
#[case::rich(ParamType::Rich, json!([{ "runs": [{ "t": "a" }] }]), ParamValue::Rich(vec![Paragraph::plain("a")]))]
#[case::flag(ParamType::Flag, json!(true), ParamValue::Value(json!(true)))]
#[case::number(number(), json!(4), ParamValue::Value(json!(4)))]
#[case::grid(ParamType::Grid, json!({ "columns": 2, "rows": 3 }), ParamValue::Value(json!({ "columns": 2, "rows": 3 })))]
fn param_value_accepted_returns_the_value(#[case] ty: ParamType, #[case] value: Value, #[case] want: ParamValue) {
    assert_that!(param_value(&ty, "p", value).unwrap()).is_equal_to(Some(want));
}

#[test]
fn param_value_chart_option_returns_its_json() {
    let option = json!({ "xAxis": { "data": ["a"] }, "series": [{ "type": "bar", "data": [1] }] });
    let got = param_value(&chart(), "option", option).unwrap();
    assert_that!(matches!(got, Some(ParamValue::Value(Value::Object(_))))).is_true();
}

#[rstest]
#[case::plain_number(ParamType::Plain, json!(3), OpErrorKind::ParamType)]
#[case::choice_number(choice(), json!(1), OpErrorKind::ParamType)]
#[case::choice_outside(choice(), json!("stretch"), OpErrorKind::ChoiceOutOfRange)]
#[case::rich_string(ParamType::Rich, json!("a"), OpErrorKind::ParamType)]
#[case::rich_bad_paragraph(ParamType::Rich, json!([{ "runs": 3 }]), OpErrorKind::InvalidValue)]
#[case::flag_string(ParamType::Flag, json!("yes"), OpErrorKind::ParamType)]
#[case::number_string(number(), json!("4"), OpErrorKind::ParamType)]
#[case::number_below(number(), json!(0), OpErrorKind::InvalidValue)]
#[case::number_above(number(), json!(11), OpErrorKind::InvalidValue)]
#[case::grid_list(ParamType::Grid, json!([2, 3]), OpErrorKind::ParamType)]
#[case::grid_unreadable(ParamType::Grid, json!({ "columns": "x" }), OpErrorKind::InvalidValue)]
#[case::grid_too_wide(ParamType::Grid, json!({ "columns": 25, "rows": 1 }), OpErrorKind::InvalidValue)]
#[case::chart_null(chart(), json!(null), OpErrorKind::ParamType)]
#[case::chart_inconsistent(chart(), json!({ "series": [] }), OpErrorKind::InvalidChart)]
#[case::chart_unreadable(chart(), json!({ "series": 3 }), OpErrorKind::InvalidChart)]
#[case::slot(slot(), json!([]), OpErrorKind::ParamType)]
fn param_value_refused_returns_error(#[case] ty: ParamType, #[case] value: Value, #[case] kind: OpErrorKind) {
    let err = param_value(&ty, "p", value).unwrap_err();
    assert_that!(err.kind()).is_equal_to(kind);
    assert_that!(err.subject()).is_equal_to(Some("p"));
}
