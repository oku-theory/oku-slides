use oku_slides_core::{Element, Op, ParamValue, Slide, SlotRef, Target, kinds};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::Session;
use crate::test_support::session;

/// The data the base pack's chart example names.
const EXAMPLE_CHART: &str = "base-example-chart";

/// The data the base pack's table example names.
const EXAMPLE_TABLE: &str = "base-example-table";

/// A `kind` element whose `param` view names `data`.
fn view(id: &str, kind: &str, param: &str, data: &str) -> Element {
    let mut e = Element::new(id, kind);
    e.params.set(param, ParamValue::Value(json!({"data": data})));
    e
}

/// A bar chart named `id` whose view names `data`.
fn chart(id: &str, data: &str) -> Element {
    let mut e = Element::new(id, kinds::base::CHART);
    let option = json!({"data": data, "xAxis": {"type": "category"}, "series": [{"type": "bar"}]});
    e.params.set("option", ParamValue::Value(option));
    e
}

/// Adds `element` to the blocks of slide `s-constat`.
fn add(element: Element) -> Op {
    let owner = Target { slide: "s-constat".into(), element: None };
    Op::AddElement { slot: SlotRef { owner, param: "blocks".into() }, index: None, defaults: false, element }
}

/// The data the `param` view of element `id` of slide `slide` names.
fn named(s: &Session, slide: &str, id: &str, param: &str) -> String {
    let located = s.deck().slide(slide).unwrap().locate(id).unwrap();
    match located.element.params.get(param) {
        Some(ParamValue::Value(v)) => v.get("data").and_then(Value::as_str).unwrap().to_owned(),
        other => panic!("view expected: {other:?}"),
    }
}

#[test]
fn two_widgets_sharing_example_data_get_one_copy() {
    let mut s = session();
    let before = s.deck().data.len();
    let ops = vec![add(chart("c1", EXAMPLE_CHART)), add(chart("c2", EXAMPLE_CHART))];
    s.apply("Ajout", ops, None).unwrap_or_else(|e| panic!("apply: {}", e.one_line()));
    let (first, second) = (named(&s, "s-constat", "c1", "option"), named(&s, "s-constat", "c2", "option"));
    assert_that!(s.deck().data.len()).is_equal_to(before + 1);
    assert_that!(second.as_str()).is_equal_to(first.as_str());
    assert_that!(first.as_str()).is_not_equal_to(EXAMPLE_CHART);
    let copy = s.deck().data.get(first.as_str()).unwrap();
    assert_that!(copy.table().unwrap().rows.len()).is_equal_to(3);
}

#[test]
fn layout_example_with_data_ref_adds_data_once() {
    let mut s = session();
    let before = s.deck().data.len();
    let mut slide = Slide::new("s-new", kinds::essentiel::CONTENT);
    let blocks = vec![
        view("t1", kinds::base::TABLE, "data", EXAMPLE_TABLE),
        view("t2", kinds::base::TABLE, "data", EXAMPLE_TABLE),
    ];
    slide.params.set("blocks", ParamValue::Slot(blocks));
    let op = Op::AddSlide { index: None, defaults: false, slide };
    s.apply("Ajout", vec![op], None).unwrap_or_else(|e| panic!("apply: {}", e.one_line()));
    let (first, second) = (named(&s, "s-new", "t1", "data"), named(&s, "s-new", "t2", "data"));
    assert_that!(s.deck().data.len()).is_equal_to(before + 1);
    assert_that!(second.as_str()).is_equal_to(first.as_str());
    assert_that!(s.deck().data.contains_key(first.as_str())).is_true();
}

#[test]
fn example_data_copy_is_undone_with_its_element() {
    let mut s = session();
    let before = s.deck().data.clone();
    s.apply("Ajout", vec![add(chart("c1", EXAMPLE_CHART))], None).unwrap();
    s.undo().unwrap();
    assert_that!(s.deck().data.clone()).is_equal_to(before);
}

#[test]
fn ref_to_existing_deck_data_adds_no_copy() {
    let mut s = session();
    s.apply("Ajout", vec![add(chart("c1", EXAMPLE_CHART))], None).unwrap();
    let copy = named(&s, "s-constat", "c1", "option");
    let before = s.deck().data.len();
    s.apply("Ajout", vec![add(chart("c2", &copy))], None).unwrap();
    assert_that!(s.deck().data.len()).is_equal_to(before);
    assert_that!(named(&s, "s-constat", "c2", "option")).is_equal_to(copy);
}

/// A session editing the base pack (pack mode).
fn editing_base() -> (tempfile::TempDir, Session) {
    let data = crate::test_support::fresh_dir("oku-slides-example-data-");
    let s = crate::test_support_packs::pack_tab_of(data.path(), crate::pack::BASE_PACK);
    (data, s)
}

/// `v` as example values.
fn values(v: Value) -> serde_json::Map<String, Value> {
    serde_json::from_value(v).unwrap()
}

/// A chart or table view is a `host` param, which example values never set: inline data or a ref
/// to a data the pack lacks is refused with the localized `TrialFixed`, and nothing is written.
#[rstest::rstest]
#[case::chart_series("chart", "option", json!({"xAxis": {"type": "category"}, "series": [{"type": "bar", "data": [1, 2]}]}))]
#[case::chart_with_dataset("chart", "option", json!({"dataset": {"source": [["a", 1]]}, "series": [{"type": "bar"}]}))]
#[case::table_dataset("table", "data", json!({"dataset": {"source": [["a"], ["b"]]}}))]
#[case::chart_missing_ref("chart", "option", json!({"data": "nowhere", "xAxis": {"type": "category"}, "series": [{"type": "bar"}]}))]
#[case::table_missing_ref("table", "data", json!({"data": "nowhere"}))]
fn set_example_values_of_a_data_view_is_refused(#[case] widget: &str, #[case] param: &str, #[case] view: Value) {
    let (_dir, mut s) = editing_base();
    let file = oku_slides_core::WidgetFile::parse(widget).unwrap();
    let before = s.example_values(None, &file).unwrap();
    let err = s.set_example_values(widget, values(json!({ param: view }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::TrialFixed);
    let [en, fr] =
        ["en", "fr"].map(|l| oku_slides_i18n::Localize::localize(&err, &oku_slides_i18n::Localizer::new(Some(l))));
    assert_that!(fr.contains('{')).is_false();
    assert_that!(fr).is_not_equal_to(en);
    assert_that!(s.example_values(None, &file).unwrap()).is_equal_to(before);
}

/// Each key Task 4c adds, with sample arguments; `n` fills the numeric ones.
fn task4c_samples(n: usize) -> Vec<(&'static str, Vec<(&'static str, oku_slides_i18n::FluentValue<'static>)>)> {
    vec![
        ("pack-manifest-data-count", vec![("count", n.into()), ("max", n.into())]),
        ("pack-manifest-data-base", vec![("data", "d1".into())]),
        ("pack-manifest-data-cells", vec![("count", n.into()), ("max", n.into())]),
        ("session-example-base-chart-category", vec![]),
        ("session-example-base-chart-value", vec![]),
    ]
}

/// Keys whose numeric variables pick a plural form.
const TASK4C_NUMERIC: [&str; 2] = ["pack-manifest-data-count", "pack-manifest-data-cells"];

#[test]
fn every_task4c_key_formats_in_all_locales() {
    use oku_slides_i18n::Localizer;
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        for (key, args) in task4c_samples(17) {
            assert_that!(l.has(key)).named(&format!("{locale} {key}")).is_true();
            let text = l.tr_with(key, &args);
            assert_that!(text.contains('{')).named(&format!("{locale} {key}: {text}")).is_false();
            assert_that!(text.trim().is_empty()).named(&format!("{locale} {key}")).is_false();
        }
    }
    let (en, fr) = (Localizer::new(Some("en")), Localizer::new(Some("fr")));
    for (key, args) in task4c_samples(17) {
        assert_that!(fr.tr_with(key, &args)).named(key).is_not_equal_to(en.tr_with(key, &args));
    }
}

/// The message of `key` in `l` with `n` in every numeric variable, the number replaced by `#`.
fn task4c_form(l: &oku_slides_i18n::Localizer, key: &str, n: usize) -> String {
    let (_, args) = task4c_samples(n).into_iter().find(|(k, _)| *k == key).unwrap();
    l.tr_with(key, &args).replace(&n.to_string(), "#")
}

#[test]
fn every_numeric_task4c_key_has_distinct_plural_forms_in_ru_and_pl() {
    use oku_slides_i18n::Localizer;
    let (ru, pl) = (Localizer::new(Some("ru")), Localizer::new(Some("pl")));
    for key in TASK4C_NUMERIC {
        for l in [&ru, &pl] {
            let [one, two, five] = [1, 2, 5].map(|n| task4c_form(l, key, n));
            assert_that!(one).named(key).is_not_equal_to(two.clone());
            assert_that!(two).named(key).is_not_equal_to(five.clone());
            assert_that!(one).named(key).is_not_equal_to(five);
        }
        assert_that!(task4c_form(&ru, key, 21)).named(key).is_equal_to(task4c_form(&ru, key, 1));
        assert_that!(task4c_form(&ru, key, 22)).named(key).is_equal_to(task4c_form(&ru, key, 2));
    }
}
