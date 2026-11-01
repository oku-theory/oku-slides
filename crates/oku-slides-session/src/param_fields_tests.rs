use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use oku_slides_core::{LocalizedText, ParamSchema, ParamType, ParamValue, kinds};
use oku_slides_i18n::Localizer;

use super::{coerce, field, kind_key, kind_name, param_help, param_label, schema_default};

fn schema(ty: ParamType, default: Option<Value>) -> ParamSchema {
    ParamSchema { name: "p".into(), label: None, help: None, ty, default, bind: None }
}

#[rstest]
#[case(ParamType::Flag, json!(true), Some(json!(true)))]
#[case(ParamType::Flag, json!(false), Some(json!(false)))]
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

fn plain_with(label: Option<LocalizedText>, help: Option<LocalizedText>) -> ParamSchema {
    ParamSchema { name: "title".into(), label, help, ty: ParamType::Plain, default: None, bind: None }
}

#[test]
fn param_label_by_locale_picks_the_ui_language() {
    let p = plain_with(Some(serde_json::from_value(json!({"en": "Title", "fr": "Titre"})).unwrap()), None);
    assert_that!(param_label("acme/card", &p, &Localizer::new(Some("fr")))).is_equal_to("Titre".to_owned());
}

#[test]
fn param_label_without_label_is_the_name() {
    assert_that!(param_label("acme/card", &plain_with(None, None), &Localizer::new(Some("fr"))))
        .is_equal_to("title".to_owned());
}

#[test]
fn field_with_help_carries_it_in_the_ui_language() {
    let help = serde_json::from_value(json!({"en": "Above the title", "fr": "Au-dessus du titre"})).unwrap();
    let f = field("acme/card", &plain_with(None, Some(help)), None, &Localizer::new(Some("fr"))).unwrap();
    assert_that!(f.help).is_equal_to(Some("Au-dessus du titre".to_owned()));
}

fn flag_named(name: &str) -> ParamSchema {
    ParamSchema { name: name.into(), label: None, help: None, ty: ParamType::Flag, default: None, bind: None }
}

#[rstest]
#[case::label("fr", "Mise en avant")]
#[case::label_en("en", "Highlight")]
fn param_label_of_a_builtin_widget_reads_its_fluent_key(#[case] locale: &str, #[case] shown: &str) {
    let p = flag_named("highlight");
    assert_that!(param_label("base/card", &p, &Localizer::new(Some(locale)))).is_equal_to(shown.to_owned());
}

#[test]
fn param_help_of_a_builtin_widget_reads_its_fluent_key() {
    let p = flag_named("highlight");
    assert_that!(param_help("base/card", &p, &Localizer::new(Some("fr"))))
        .is_equal_to(Some("Passe la carte sur le fond sombre du thème".to_owned()));
}

#[test]
fn param_label_of_a_pack_widget_without_label_is_the_name() {
    let p = flag_named("highlight");
    assert_that!(param_label("acme/card", &p, &Localizer::new(Some("fr")))).is_equal_to("highlight".to_owned());
}

#[test]
fn field_of_a_flag_is_a_switch_with_a_bool_value() {
    let p = ParamSchema { name: "on".into(), label: None, help: None, ty: ParamType::Flag, default: None, bind: None };
    let f = field("acme/w", &p, Some(&ParamValue::Value(json!(true))), &Localizer::new(None)).unwrap();
    assert_that!((f.kind, f.value)).is_equal_to((crate::agent::FieldKind::Switch, json!(true)));
}
