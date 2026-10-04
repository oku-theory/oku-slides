use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use crate::SessionErrorKind;
use crate::test_support::{fresh_dir, opened_on};
use crate::widgets_edit::parse_name;

const ALL: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("tout", params: (
  title: p.plain(), body: p.rich(), count: p.number(min: 1, max: 9), size: p.number(min: 0),
  tone: p.choice("chaud", "froid"), on: p.flag(), logo: p.asset()),
  (args, ctx) => field(ctx, "title", args.title))
"#;

fn templates() -> (tempfile::TempDir, crate::Session) {
    let data = fresh_dir("oku-slides-params-");
    let mut s = opened_on(data.path());
    let set = s.deck().templates.clone().unwrap().id;
    s.edit_templates(set).unwrap();
    s.set_widget("tout", ALL.into()).unwrap();
    (data, s)
}

fn values(v: Value) -> Map<String, Value> {
    serde_json::from_value(v).unwrap()
}

fn trial(s: &crate::Session, param: &str) -> Option<Value> {
    s.scratch().unwrap().trials_of(&parse_name("tout").unwrap()).and_then(|m| m.get(param).cloned())
}

fn para(t: &str) -> Value {
    json!([{ "runs": [{ "t": t }] }])
}

#[test]
fn set_widget_trials_stores_a_plain_value() {
    let (_d, mut s) = templates();
    s.set_widget_trials("tout", values(json!({ "title": "Météo" }))).unwrap();
    assert_that!(trial(&s, "title")).is_equal_to(Some(json!("Météo")));
}

#[test]
fn set_widget_trials_reads_a_flag_option_as_a_boolean() {
    let (_d, mut s) = templates();
    s.set_widget_trials("tout", values(json!({ "on": "true" }))).unwrap();
    assert_that!(trial(&s, "on")).is_equal_to(Some(json!(true)));
}

#[test]
fn set_widget_trials_reads_number_text_as_a_number() {
    let (_d, mut s) = templates();
    s.set_widget_trials("tout", values(json!({ "size": "12.5" }))).unwrap();
    assert_that!(trial(&s, "size")).is_equal_to(Some(json!(12.5)));
}

#[test]
fn set_widget_trials_stores_rich_paragraphs() {
    let (_d, mut s) = templates();
    s.set_widget_trials("tout", values(json!({ "body": para("Un") }))).unwrap();
    assert_that!(trial(&s, "body")).is_equal_to(Some(para("Un")));
}

#[rstest]
#[case::number_as_integer("count", json!(5), json!(1))]
#[case::flag_as_text("on", json!("true"), json!("false"))]
#[case::choice("tone", json!("froid"), json!("chaud"))]
#[case::null("title", json!("x"), Value::Null)]
#[case::empty_text("title", json!("x"), json!(""))]
#[case::empty_rich("body", para("y"), json!([]))]
fn set_widget_trials_preview_or_empty_value_erases(#[case] param: &str, #[case] set: Value, #[case] reset: Value) {
    let (_d, mut s) = templates();
    s.set_widget_trials("tout", values(json!({ param: set }))).unwrap();
    s.set_widget_trials("tout", values(json!({ param: reset }))).unwrap();
    assert_that!(trial(&s, param)).is_none();
}

#[rstest]
#[case::number_above_max("count", json!(12), SessionErrorKind::TrialValue)]
#[case::number_as_word("count", json!("trois"), SessionErrorKind::TrialValue)]
#[case::unknown_choice("tone", json!("tiède"), SessionErrorKind::TrialValue)]
#[case::rich_as_text("body", json!("texte"), SessionErrorKind::TrialValue)]
#[case::asset("logo", json!("sha256:00"), SessionErrorKind::TrialFixed)]
#[case::undeclared("absent", json!("x"), SessionErrorKind::TrialUndeclared)]
#[case::plain_too_long("title", json!("x".repeat(4097)), SessionErrorKind::TrialTooLong)]
#[case::rich_too_many("body", json!(vec![json!({ "runs": [{ "t": "p" }] }); 65]), SessionErrorKind::TrialTooLong)]
#[case::rich_too_big("body", para(&"x".repeat(16400)), SessionErrorKind::TrialTooLong)]
fn set_widget_trials_bad_value_is_refused(#[case] param: &str, #[case] value: Value, #[case] kind: SessionErrorKind) {
    let (_d, mut s) = templates();
    let err = s.set_widget_trials("tout", values(json!({ param: value }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(kind);
}

#[test]
fn set_widget_trials_refusal_names_the_param() {
    let (_d, mut s) = templates();
    let err = s.set_widget_trials("tout", values(json!({ "count": 12 }))).unwrap_err();
    assert_that!(err.to_string().as_str()).contains("count");
}

#[test]
fn set_widget_trials_undeclared_key_is_clipped_in_the_error() {
    let (_d, mut s) = templates();
    let err = s.set_widget_trials("tout", values(json!({ "k".repeat(300): "x" }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::TrialUndeclared);
    assert_that!(err.to_string().chars().count()).is_less_than(200);
}

#[rstest]
#[case::min(1)]
#[case::max(9)]
fn set_widget_trials_number_at_a_bound_is_accepted(#[case] n: i64) {
    let (_d, mut s) = templates();
    assert_that!(s.set_widget_trials("tout", values(json!({ "count": n })))).is_ok();
}

#[test]
fn set_widget_trials_plain_at_the_limit_is_accepted() {
    let (_d, mut s) = templates();
    assert_that!(s.set_widget_trials("tout", values(json!({ "title": "x".repeat(4096) })))).is_ok();
}

#[test]
fn set_widget_trials_with_one_bad_value_stores_nothing() {
    let (_d, mut s) = templates();
    assert_that!(s.set_widget_trials("tout", values(json!({ "title": "A", "count": 99 })))).is_err();
    assert_that!(trial(&s, "title")).is_none();
}

#[test]
fn set_widget_trials_on_a_broken_file_is_refused_as_invalid() {
    let (_d, mut s) = templates();
    s.set_widget("tout", "#let widget = (".into()).unwrap();
    let err = s.set_widget_trials("tout", values(json!({ "title": "x" }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::WidgetInvalid);
}

#[rstest]
#[case::common("_commun")]
#[case::unknown("absent")]
fn set_widget_trials_on_a_non_widget_is_refused_as_unknown(#[case] name: &str) {
    let (_d, mut s) = templates();
    s.set_widget("_commun", "#let x = 1\n".into()).unwrap();
    let err = s.set_widget_trials(name, values(json!({ "title": "x" }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWidget);
}

#[test]
fn set_widget_trials_outside_templates_mode_is_refused() {
    let data = fresh_dir("oku-slides-params-");
    let mut s = opened_on(data.path());
    s.set_widget("tout", ALL.into()).unwrap();
    let err = s.set_widget_trials("tout", values(json!({ "title": "x" }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NotEditingTemplates);
}

#[test]
fn set_widget_trials_bumps_the_snapshot_revision() {
    let (_d, mut s) = templates();
    let before = s.snapshot().widget_trials;
    s.set_widget_trials("tout", values(json!({ "title": "x" }))).unwrap();
    assert_that!(s.snapshot().widget_trials).is_greater_than(before);
}

fn settings(s: &crate::Session, name: &str) -> crate::panel::HostParams {
    s.widget_settings(name).unwrap().unwrap()
}

#[test]
fn widget_settings_keys_follow_the_schema_order() {
    let (_d, s) = templates();
    assert_that!(settings(&s, "tout").keys)
        .is_equal_to(["title", "body", "count", "size", "tone", "on"].map(String::from).to_vec());
}

#[rstest]
#[case::plain(0, crate::agent::FieldKind::Text)]
#[case::rich(1, crate::agent::FieldKind::Rich)]
#[case::bounded_number(2, crate::agent::FieldKind::Slider)]
#[case::open_number(3, crate::agent::FieldKind::Text)]
#[case::choice(4, crate::agent::FieldKind::Select)]
#[case::flag(5, crate::agent::FieldKind::Select)]
fn widget_settings_field_kind_matches_the_param_type(#[case] i: usize, #[case] kind: crate::agent::FieldKind) {
    let (_d, s) = templates();
    assert_that!(settings(&s, "tout").spec.fields.get(i).map(|f| f.kind)).is_equal_to(Some(kind));
}

#[test]
fn widget_settings_lists_an_unsettable_param_as_a_note() {
    let (_d, s) = templates();
    assert_that!(settings(&s, "tout").notes.join(" ").as_str()).contains("logo");
}

#[test]
fn widget_settings_value_is_the_preview_value() {
    let (_d, s) = templates();
    let l = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(settings(&s, "tout").spec.fields[0].value.clone()).is_equal_to(json!(l.tr("widget-preview-plain")));
}

#[test]
fn widget_settings_value_shows_the_trial() {
    let (_d, mut s) = templates();
    s.set_widget_trials("tout", values(json!({ "on": "true" }))).unwrap();
    assert_that!(settings(&s, "tout").spec.fields[5].value.clone()).is_equal_to(json!("true"));
}

#[test]
fn widget_settings_slider_step_fits_a_small_range() {
    let (_d, s) = templates();
    assert_that!(settings(&s, "tout").spec.fields[2].step).is_equal_to(Some(0.08));
}

#[test]
fn widget_settings_of_common_is_none() {
    let (_d, mut s) = templates();
    s.set_widget("_commun", "#let x = 1\n".into()).unwrap();
    assert_that!(s.widget_settings("_commun").unwrap()).is_none();
}

#[test]
fn widget_settings_of_a_broken_file_keeps_one_note() {
    let (_d, mut s) = templates();
    s.set_widget("tout", "#let widget = (".into()).unwrap();
    let p = settings(&s, "tout");
    assert_that!((p.spec.fields.len(), p.notes.len())).is_equal_to((0, 1));
}

/// Le champ d'un nombre borné `min..max`, montré à `min`.
fn bounded(min: f64, max: f64) -> crate::agent::ParamField {
    let p = oku_slides_core::ParamSchema {
        name: "n".into(),
        label: None,
        ty: oku_slides_core::ParamType::Number { min: Some(min), max: Some(max) },
        default: None,
        bind: None,
    };
    let shown = oku_slides_core::ParamValue::Value(json!(min));
    super::field(&p, Some(&shown), &oku_slides_i18n::Localizer::new(Some("fr"))).unwrap()
}

#[rstest]
#[case::empty_range(3.0, 3.0)]
#[case::reversed_range(5.0, 1.0)]
fn widget_settings_bounds_without_a_range_give_a_text_field(#[case] min: f64, #[case] max: f64) {
    assert_that!(bounded(min, max).kind).is_equal_to(crate::agent::FieldKind::Text);
}

#[test]
fn widget_settings_slider_step_has_no_float_noise() {
    assert_that!(bounded(0.3, 1.0).step).is_equal_to(Some(0.007));
}
