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
