use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use oku_slides_core::ListOp;

use crate::SessionErrorKind;
use crate::test_support::{fresh_dir, opened_on};
use crate::widgets_edit::parse_name;

const ALL: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("tout", params: (
  title: p.plain(), body: p.rich(), count: p.number(min: 1, max: 9), size: p.number(min: 0),
  tone: p.choice("chaud", "froid"), on: p.flag(), logo: p.asset()),
  (args, ctx) => field(ctx, "title", args.title))
"#;

fn pack() -> (tempfile::TempDir, crate::Session) {
    let data = fresh_dir("oku-slides-params-");
    let mut s = opened_on(data.path());
    let set = crate::BASE_PACK;
    s.edit_pack(set).unwrap();
    s.set_widget(None, "tout", ALL.into()).unwrap();
    (data, s)
}

fn values(v: Value) -> Map<String, Value> {
    serde_json::from_value(v).unwrap()
}

fn stored(s: &crate::Session, param: &str) -> Option<Value> {
    s.example_values(None, &parse_name("tout").unwrap()).unwrap().get(param).cloned()
}

fn para(t: &str) -> Value {
    json!([{ "runs": [{ "t": t }] }])
}

#[test]
fn set_example_values_stores_a_plain_value() {
    let (_d, mut s) = pack();
    s.set_example_values("tout", values(json!({ "title": "Météo" }))).unwrap();
    assert_that!(stored(&s, "title")).is_equal_to(Some(json!("Météo")));
}

#[test]
fn set_example_values_reads_a_flag_option_as_a_boolean() {
    let (_d, mut s) = pack();
    s.set_example_values("tout", values(json!({ "on": "true" }))).unwrap();
    assert_that!(stored(&s, "on")).is_equal_to(Some(json!(true)));
}

#[test]
fn set_example_values_reads_number_text_as_a_number() {
    let (_d, mut s) = pack();
    s.set_example_values("tout", values(json!({ "size": "12.5" }))).unwrap();
    assert_that!(stored(&s, "size")).is_equal_to(Some(json!(12.5)));
}

#[test]
fn set_example_values_stores_rich_paragraphs() {
    let (_d, mut s) = pack();
    s.set_example_values("tout", values(json!({ "body": para("Un") }))).unwrap();
    assert_that!(stored(&s, "body")).is_equal_to(Some(para("Un")));
}

/// Ce que la scène de l'aperçu envoie pour un `rich` (`fieldValue` côté
/// interface) : puces, numéros et runs mis en forme passent tels quels.
#[test]
fn set_example_values_stores_rich_paragraphs_as_the_preview_stage_writes_them() {
    let (_d, mut s) = pack();
    let body = json!([
        { "runs": [{ "t": "Un " }, { "t": "gras", "bold": true }, { "t": " et " }, { "t": "penché", "italic": true }] },
        { "runs": [{ "t": "Point" }], "list": "bullet" },
        { "runs": [{ "t": "lien", "link": "https://exemple.fr" }, { "t": " marqué", "highlight": 1 }], "list": "number" },
    ]);
    s.set_example_values("tout", values(json!({ "body": body.clone() }))).unwrap();
    assert_that!(stored(&s, "body")).is_equal_to(Some(body));
}

#[rstest]
#[case::number_as_integer("count", json!(5), json!(1))]
#[case::flag_as_text("on", json!("true"), json!("false"))]
#[case::choice("tone", json!("froid"), json!("chaud"))]
#[case::null("title", json!("x"), Value::Null)]
fn set_example_values_preview_or_null_value_erases(#[case] param: &str, #[case] set: Value, #[case] reset: Value) {
    let (_d, mut s) = pack();
    s.set_example_values("tout", values(json!({ param: set }))).unwrap();
    s.set_example_values("tout", values(json!({ param: reset }))).unwrap();
    assert_that!(stored(&s, param)).is_none();
}

/// Un texte vidé reste vide dans les valeurs d'exemple (t-497), au lieu d'être effacé.
#[rstest]
#[case::empty_text("title", json!("x"), json!(""), json!(""))]
#[case::blank_text("title", json!("x"), json!("  "), json!(""))]
#[case::empty_rich("body", para("y"), json!([]), json!([]))]
#[case::empty_paragraph("body", para("y"), para(""), json!([]))]
fn set_example_values_emptied_text_stays_empty(
    #[case] param: &str,
    #[case] set: Value,
    #[case] emptied: Value,
    #[case] kept: Value,
) {
    let (_d, mut s) = pack();
    s.set_example_values("tout", values(json!({ param: set }))).unwrap();
    s.set_example_values("tout", values(json!({ param: emptied }))).unwrap();
    assert_that!(stored(&s, param)).is_equal_to(Some(kept));
}

#[rstest]
#[case::number_above_max("count", json!(12), SessionErrorKind::TrialValue)]
#[case::number_as_word("count", json!("trois"), SessionErrorKind::TrialValue)]
#[case::unknown_choice("tone", json!("tiède"), SessionErrorKind::TrialValue)]
#[case::rich_as_text("body", json!("texte"), SessionErrorKind::TrialValue)]
#[case::rich_not_paragraphs("body", json!([1]), SessionErrorKind::TrialValue)]
#[case::asset("logo", json!("sha256:00"), SessionErrorKind::TrialFixed)]
#[case::undeclared("absent", json!("x"), SessionErrorKind::TrialUndeclared)]
#[case::plain_too_long("title", json!("x".repeat(4097)), SessionErrorKind::TrialTooLong)]
#[case::rich_too_many("body", json!(vec![json!({ "runs": [{ "t": "p" }] }); 65]), SessionErrorKind::TrialTooLong)]
#[case::rich_too_big("body", para(&"x".repeat(16400)), SessionErrorKind::TrialTooLong)]
fn set_example_values_bad_value_is_refused(#[case] param: &str, #[case] value: Value, #[case] kind: SessionErrorKind) {
    let (_d, mut s) = pack();
    let err = s.set_example_values("tout", values(json!({ param: value }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(kind);
}

#[test]
fn set_example_values_plain_of_max_chars_padded_with_spaces_is_accepted() {
    let (_d, mut s) = pack();
    let padded = format!("  {}  ", "x".repeat(oku_slides_core::PLAIN_MAX));
    s.set_example_values("tout", values(json!({ "title": padded }))).unwrap();
    assert_that!(stored(&s, "title")).is_equal_to(Some(json!("x".repeat(oku_slides_core::PLAIN_MAX))));
}

#[test]
fn set_example_values_refusal_names_the_param() {
    let (_d, mut s) = pack();
    let err = s.set_example_values("tout", values(json!({ "count": 12 }))).unwrap_err();
    assert_that!(err.to_string().as_str()).contains("count");
}

#[test]
fn set_example_values_undeclared_key_is_clipped_in_the_error() {
    let (_d, mut s) = pack();
    let err = s.set_example_values("tout", values(json!({ "k".repeat(300): "x" }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::TrialUndeclared);
    assert_that!(err.to_string().chars().count()).is_less_than(200);
}

#[rstest]
#[case::min(1)]
#[case::max(9)]
fn set_example_values_number_at_a_bound_is_accepted(#[case] n: i64) {
    let (_d, mut s) = pack();
    assert_that!(s.set_example_values("tout", values(json!({ "count": n })))).is_ok();
}

#[test]
fn set_example_values_plain_at_the_limit_is_accepted() {
    let (_d, mut s) = pack();
    assert_that!(s.set_example_values("tout", values(json!({ "title": "x".repeat(4096) })))).is_ok();
}

#[test]
fn set_example_values_with_one_bad_value_stores_nothing() {
    let (_d, mut s) = pack();
    assert_that!(s.set_example_values("tout", values(json!({ "title": "A", "count": 99 })))).is_err();
    assert_that!(stored(&s, "title")).is_none();
}

#[test]
fn set_example_values_on_a_broken_file_is_refused_as_invalid() {
    let (_d, mut s) = pack();
    s.set_widget(None, "tout", "#let widget = (".into()).unwrap();
    let err = s.set_example_values("tout", values(json!({ "title": "x" }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::WidgetInvalid);
}

#[rstest]
#[case::common("_commun")]
#[case::unknown("absent")]
fn set_example_values_on_a_non_widget_is_refused_as_unknown(#[case] name: &str) {
    let (_d, mut s) = pack();
    s.set_widget(None, "_commun", "#let x = 1\n".into()).unwrap();
    let err = s.set_example_values(name, values(json!({ "title": "x" }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWidget);
}

#[test]
fn set_example_values_outside_pack_mode_is_refused() {
    let data = fresh_dir("oku-slides-params-");
    let mut s = opened_on(data.path());
    s.set_widget(None, "tout", ALL.into()).unwrap();
    let err = s.set_example_values("tout", values(json!({ "title": "x" }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NotEditingPack);
}

#[test]
fn set_example_values_changes_the_snapshot_revision() {
    let (_d, mut s) = pack();
    let before = s.snapshot().widget_trials;
    s.set_example_values("tout", values(json!({ "title": "x" }))).unwrap();
    assert_that!(s.snapshot().widget_trials).is_not_equal_to(before);
}

fn settings(s: &crate::Session, name: &str) -> crate::panel::HostParams {
    s.widget_settings(name).unwrap().unwrap()
}

#[test]
fn widget_settings_keys_follow_the_schema_order() {
    let (_d, s) = pack();
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
    let (_d, s) = pack();
    assert_that!(settings(&s, "tout").spec.fields.get(i).map(|f| f.kind)).is_equal_to(Some(kind));
}

#[test]
fn widget_settings_lists_an_unsettable_param_as_a_note() {
    let (_d, s) = pack();
    assert_that!(settings(&s, "tout").notes.join(" ").as_str()).contains("logo");
}

#[test]
fn widget_settings_value_is_the_preview_value() {
    let (_d, s) = pack();
    let l = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(settings(&s, "tout").spec.fields[0].value.clone()).is_equal_to(json!(l.tr("widget-preview-plain")));
}

#[test]
fn widget_settings_value_shows_the_example_value() {
    let (_d, mut s) = pack();
    s.set_example_values("tout", values(json!({ "on": "true" }))).unwrap();
    assert_that!(settings(&s, "tout").spec.fields[5].value.clone()).is_equal_to(json!("true"));
}

#[test]
fn widget_settings_slider_step_fits_a_small_range() {
    let (_d, s) = pack();
    assert_that!(settings(&s, "tout").spec.fields[2].step).is_equal_to(Some(0.08));
}

#[test]
fn widget_settings_of_common_is_none() {
    let (_d, mut s) = pack();
    s.set_widget(None, "_commun", "#let x = 1\n".into()).unwrap();
    assert_that!(s.widget_settings("_commun").unwrap()).is_none();
}

#[test]
fn widget_settings_of_a_broken_file_keeps_one_note() {
    let (_d, mut s) = pack();
    s.set_widget(None, "tout", "#let widget = (".into()).unwrap();
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

fn pack_with_list() -> (tempfile::TempDir, crate::Session) {
    let data = fresh_dir("oku-slides-params-");
    let mut s = opened_on(data.path());
    let set = crate::BASE_PACK;
    s.edit_pack(set).unwrap();
    crate::test_support::set_frise_widget(&mut s);
    (data, s)
}

fn list_trial(s: &crate::Session) -> Option<Value> {
    s.example_values(None, &parse_name("frise").unwrap()).unwrap().get("etapes").cloned()
}

/// Ids des étapes par défaut de la frise de test.
const DEFAULT_IDS: [&str; 4] = ["d0", "d1", "d2", "d3"];

fn strings(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|i| (*i).to_owned()).collect()
}

/// Les étapes par défaut, suivies de `id`.
fn with_last(id: &str) -> Vec<String> {
    DEFAULT_IDS.iter().copied().chain([id]).map(String::from).collect()
}

fn ids(v: &Value) -> Vec<String> {
    v.as_array().unwrap().iter().map(|i| i["id"].as_str().unwrap().to_owned()).collect()
}

fn insert(id: &str) -> ListOp {
    ListOp::Insert { index: None, id: id.into(), values: Map::default() }
}

#[test]
fn widget_settings_lists_a_list_param_as_a_list_field() {
    let (_d, s) = pack_with_list();
    let p = settings(&s, "frise");
    let f = &p.spec.fields[0];
    assert_that!(p.keys.first().cloned()).is_equal_to(Some("etapes".to_owned()));
    assert_that!(f.kind).is_equal_to(crate::agent::FieldKind::List);
    let columns: Vec<(String, oku_slides_a2ui::ColumnKind)> =
        f.columns.iter().map(|c| (c.key.clone(), c.kind)).collect();
    assert_that!(columns).is_equal_to(vec![
        ("date".to_owned(), oku_slides_a2ui::ColumnKind::Text),
        ("texte".to_owned(), oku_slides_a2ui::ColumnKind::Text),
    ]);
    assert_that!(ids(&f.value)).is_equal_to(strings(&DEFAULT_IDS));
    assert_that!(f.value[0]["texte"].clone()).is_equal_to(json!("Création"));
    assert_that!((f.item_label.clone(), f.min, f.max)).is_equal_to((Some("étape".to_owned()), Some(0.0), Some(8.0)));
}

#[test]
fn widget_settings_list_field_without_max_reports_the_core_limit() {
    let (_d, mut s) = pack_with_list();
    s.set_widget(None, "frise", crate::test_support::FRISE_TEST.replace(" max: 8,", "")).unwrap();
    let f = &settings(&s, "frise").spec.fields[0];
    assert_that!(f.max).is_equal_to(Some(oku_slides_core::num::count_f64(oku_slides_core::LIST_ITEMS_MAX)));
}

#[test]
fn widget_settings_list_value_shows_the_example_list() {
    let (_d, mut s) = pack_with_list();
    s.edit_example_list("frise", "etapes", ListOp::Remove { item: "d0".into() }).unwrap();
    assert_that!(ids(&settings(&s, "frise").spec.fields[0].value)).is_equal_to(strings(&DEFAULT_IDS[1..]));
}

#[test]
fn edit_example_list_inserts_into_the_default_items() {
    let (_d, mut s) = pack_with_list();
    s.edit_example_list("frise", "etapes", insert("i-1")).unwrap();
    let all = ids(&list_trial(&s).unwrap());
    assert_that!(all).is_equal_to(with_last("i-1"));
    // Une colonne absente reçoit son défaut.
    assert_that!(list_trial(&s).unwrap()[4]["texte"].clone()).is_equal_to(json!("Nouvelle étape"));
}

#[test]
fn edit_example_list_continues_from_the_previous_trial() {
    let (_d, mut s) = pack_with_list();
    s.edit_example_list("frise", "etapes", ListOp::Remove { item: "d0".into() }).unwrap();
    s.edit_example_list(
        "frise",
        "etapes",
        ListOp::SetField { item: "d1".into(), field: "texte".into(), value: json!("Changé") },
    )
    .unwrap();
    let t = list_trial(&s).unwrap();
    assert_that!(ids(&t)).is_equal_to(strings(&DEFAULT_IDS[1..]));
    assert_that!(t[0]["texte"].clone()).is_equal_to(json!("Changé"));
}

#[test]
fn edit_example_list_returns_the_widget_values() {
    let (_d, mut s) = pack_with_list();
    let trials = s.edit_example_list("frise", "etapes", insert("i-1")).unwrap();
    assert_that!(trials.contains_key("etapes")).is_true();
}

#[test]
fn edit_example_list_over_the_max_is_refused_and_keeps_the_trial() {
    let (_d, mut s) = pack_with_list();
    for i in 0..4 {
        s.edit_example_list("frise", "etapes", insert(&format!("i-{i}"))).unwrap();
    }
    let err = s.edit_example_list("frise", "etapes", insert("i-4")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::TrialValue);
    assert_that!(ids(&list_trial(&s).unwrap()).len()).is_equal_to(8);
}

#[rstest]
#[case::not_a_list("tout", "title", SessionErrorKind::TrialUndeclared)]
#[case::undeclared("frise", "absent", SessionErrorKind::TrialUndeclared)]
fn edit_example_list_on_a_non_list_is_refused(
    #[case] widget: &str,
    #[case] param: &str,
    #[case] kind: SessionErrorKind,
) {
    let (_d, mut s) = pack_with_list();
    s.set_widget(None, "tout", ALL.into()).unwrap();
    let err = s.edit_example_list(widget, param, insert("i-1")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(kind);
}

#[test]
fn edit_example_list_outside_pack_mode_is_refused() {
    let data = fresh_dir("oku-slides-params-");
    let mut s = opened_on(data.path());
    let err = s.edit_example_list("frise", "etapes", insert("i-1")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NotEditingPack);
}

#[test]
fn edit_example_list_through_a_request_is_accepted() {
    let (_d, mut s) = pack_with_list();
    let req: crate::Request = serde_json::from_value(json!({
        "cmd": "edit_example_list", "name": "frise", "param": "etapes",
        "change": { "op": "insert", "id": "i-1" }
    }))
    .unwrap();
    s.handle(req).unwrap();
    assert_that!(ids(&list_trial(&s).unwrap()).last().cloned()).is_equal_to(Some("i-1".to_owned()));
}

#[test]
fn set_example_values_accepts_a_whole_list() {
    let (_d, mut s) = pack_with_list();
    let whole = json!([{ "id": "a", "date": "1999", "texte": "Un" }, { "id": "b", "date": "2000", "texte": "Deux" }]);
    s.set_example_values("frise", values(json!({ "etapes": whole.clone() }))).unwrap();
    assert_that!(list_trial(&s)).is_equal_to(Some(whole));
}

#[test]
fn set_example_values_refuses_a_list_with_a_duplicate_id() {
    let (_d, mut s) = pack_with_list();
    let bad = json!([{ "id": "a", "texte": "Un" }, { "id": "a", "texte": "Deux" }]);
    let err = s.set_example_values("frise", values(json!({ "etapes": bad }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::TrialValue);
}

#[test]
fn set_example_values_pasting_the_default_list_erases_the_trial() {
    let (_d, mut s) = pack_with_list();
    s.edit_example_list("frise", "etapes", insert("i-1")).unwrap();
    let default = json!([
        { "id": "d0", "date": "2019", "texte": "Création" },
        { "id": "d1", "date": "2021", "texte": "Premier client" },
        { "id": "d2", "date": "2023", "texte": "International" },
        { "id": "d3", "date": "2025", "texte": "Série B" }
    ]);
    s.set_example_values("frise", values(json!({ "etapes": default }))).unwrap();
    assert_that!(list_trial(&s)).is_none();
}

#[test]
fn widget_preview_params_of_a_list_are_its_default_items() {
    let (_d, s) = pack_with_list();
    let schema = s.valid_schema(&parse_name("frise").unwrap()).unwrap().unwrap();
    let params =
        crate::widget_preview::preview_params(&schema, &Map::new(), &oku_slides_i18n::Localizer::new(Some("fr")));
    let items = params.list("etapes").unwrap();
    assert_that!(items.iter().map(|i| i.id.as_str()).collect::<Vec<_>>()).is_equal_to(DEFAULT_IDS.to_vec());
    assert_that!(items[1].values.text("texte")).is_equal_to(Some("Premier client"));
}
