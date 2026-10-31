use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use crate::pack::PackManifest;
use crate::test_support::ATELIER;
use crate::{Request, Session, SessionErrorKind};

const KPI: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (value: p.plain()), (args, ctx) => field(ctx, "value", args.value))
"#;

/// A pack tab of the empty widget pack `atelier`, on a fresh folder.
fn editing(data: &std::path::Path) -> Session {
    let mut opener = crate::test_support::session_on(data);
    crate::test_support::pin_atelier(&mut opener);
    crate::test_support_packs::pack_tab_from(&mut opener, data, ATELIER)
}

/// A layout pack tab holding `a`, `b` and `c`.
fn layouts(data: &std::path::Path) -> Session {
    crate::test_support::layout_pack_tab(data, "salle", &["a", "b", "c"]).0
}

/// The edited version's items, in their stored order.
fn items(s: &Session) -> Vec<String> {
    let t = s.widget_target(None).unwrap();
    let manifest = PackManifest::from_files(&t.files).unwrap();
    oku_slides_core::item_order(&manifest.order, &t.files).into_iter().map(|n| n.as_str().to_owned()).collect()
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|n| (*n).to_owned()).collect()
}

#[test]
fn duplicate_widget_without_a_name_takes_the_first_free_copy_name() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = editing(data.path());
    s.set_widget(None, "kpi", KPI.into()).unwrap();
    let copy = s.duplicate_widget(None, "kpi", None).unwrap();
    assert_that!(copy.name.as_str()).is_equal_to("kpi-2");
    assert_that!(s.widget_source(None, "kpi-2").unwrap().as_str()).contains("widget(\"kpi-2\"");
}

#[test]
fn duplicate_widget_when_the_copy_name_is_taken_takes_the_next() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = editing(data.path());
    s.set_widget(None, "kpi", KPI.into()).unwrap();
    s.duplicate_widget(None, "kpi", None).unwrap();
    let copy = s.duplicate_widget(None, "kpi", None).unwrap();
    assert_that!(copy.name.as_str()).is_equal_to("kpi-3");
}

#[test]
fn duplicate_widget_with_a_name_uses_it() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = editing(data.path());
    s.set_widget(None, "kpi", KPI.into()).unwrap();
    let copy = s.duplicate_widget(None, "kpi", Some("total")).unwrap();
    assert_that!(copy.name.as_str()).is_equal_to("total");
    assert_that!(s.widget_source(None, "total").unwrap().as_str()).contains("widget(\"total\"");
}

#[test]
fn duplicate_widget_to_a_taken_name_is_refused() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = editing(data.path());
    s.set_widget(None, "kpi", KPI.into()).unwrap();
    s.set_widget(None, "total", KPI.into()).unwrap();
    let err = s.duplicate_widget(None, "kpi", Some("total")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InvalidWidget);
    let key = "session-widget-exists";
    let en = s.localizer.tr_with(key, &[("name", "total".into())]);
    assert_that!(oku_slides_i18n::Localize::localize(&err, &s.localizer)).is_equal_to(en);
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr))
        .is_equal_to(fr.tr_with(key, &[("name", "total".into())]));
    assert_that!(fr.tr_with(key, &[("name", "x".into())])).contains("existe déjà");
}

#[test]
fn duplicate_widget_of_the_shared_file_is_refused() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = editing(data.path());
    s.set_widget(None, "_commun", "#let cadre(body) = block(inset: 8pt, body)\n".into()).unwrap();
    let err = s.duplicate_widget(None, "_commun", None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InvalidWidget);
}

#[test]
fn duplicate_widget_of_a_64_char_name_is_refused_with_no_copy_name() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = editing(data.path());
    let long = "k".repeat(64);
    s.set_widget(None, &long, KPI.replace("\"kpi\"", &format!("\"{long}\""))).unwrap();
    let err = s.duplicate_widget(None, &long, None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InvalidWidget);
    let key = "session-widget-no-copy-name";
    let en = s.localizer.tr_with(key, &[("from", long.as_str().into())]);
    assert_that!(oku_slides_i18n::Localize::localize(&err, &s.localizer)).is_equal_to(en);
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr))
        .is_equal_to(fr.tr_with(key, &[("from", long.as_str().into())]));
    assert_that!(fr.tr_with(key, &[("from", "x".into())])).contains("Aucun nom libre");
}

fn values(v: Value) -> Map<String, Value> {
    serde_json::from_value(v).unwrap()
}

#[test]
fn duplicate_widget_copies_examples_and_style() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = editing(data.path());
    s.set_widget(None, "kpi", KPI.into()).unwrap();
    s.set_example_values("kpi", values(json!({ "value": "12" }))).unwrap();
    s.duplicate_widget(None, "kpi", None).unwrap();
    let t = s.widget_target(None).unwrap();
    let manifest = PackManifest::from_files(&t.files).unwrap();
    let of = |n: &str| oku_slides_core::WidgetFile::parse(n).unwrap();
    assert_that!(manifest.examples.get(&of("kpi-2")).cloned()).is_equal_to(Some(values(json!({ "value": "12" }))));
    assert_that!(manifest.examples.get(&of("kpi-2"))).is_equal_to(manifest.examples.get(&of("kpi")));
    assert_that!(manifest.styles.get(&of("kpi-2"))).is_equal_to(manifest.styles.get(&of("kpi")));
}

#[test]
fn duplicate_widget_places_the_copy_right_after_its_original() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = layouts(data.path());
    s.duplicate_widget(None, "a", None).unwrap();
    assert_that!(items(&s)).is_equal_to(names(&["a", "a-2", "b", "c"]));
}

#[test]
fn duplicate_widget_then_undo_removes_the_copy() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = editing(data.path());
    s.set_widget(None, "kpi", KPI.into()).unwrap();
    s.duplicate_widget(None, "kpi", None).unwrap();
    s.undo().unwrap();
    let err = s.widget_source(None, "kpi-2").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWidget);
}

#[test]
fn reorder_widgets_stores_the_order() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = layouts(data.path());
    s.reorder_widgets(None, &names(&["c", "a", "b"])).unwrap();
    assert_that!(items(&s)).is_equal_to(names(&["c", "a", "b"]));
}

#[rstest]
#[case::missing(&["a", "b"])]
#[case::extra(&["a", "b", "c", "d"])]
#[case::repeat(&["a", "a", "b", "c"])]
#[case::shared(&["_commun", "a", "b", "c"])]
fn reorder_widgets_with_a_missing_or_extra_name_is_refused(#[case] order: &[&str]) {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = layouts(data.path());
    let err = s.reorder_widgets(None, &names(order)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InvalidWidget);
}

#[test]
fn reorder_widgets_with_a_missing_name_lists_it_in_french() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = layouts(data.path());
    let err = s.reorder_widgets(None, &names(&["a", "b"])).unwrap_err();
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    let shown = oku_slides_i18n::Localize::localize(&err, &fr);
    assert_that!(shown)
        .is_equal_to(fr.tr_with("session-widget-order-mismatch", &[("missing", "c".into()), ("extra", "—".into())]));
    assert_that!(shown).contains("Manquants");
}

#[test]
fn reorder_widgets_to_the_same_order_makes_no_version() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = layouts(data.path());
    s.reorder_widgets(None, &names(&["a", "b", "c"])).unwrap();
    assert_that!(s.snapshot().editing.unwrap().session_version).is_none();
    s.reorder_widgets(None, &names(&["c", "a", "b"])).unwrap();
    let version = s.widget_target(None).unwrap().version;
    s.reorder_widgets(None, &names(&["c", "a", "b"])).unwrap();
    assert_that!(s.widget_target(None).unwrap().version).is_equal_to(version);
}

#[test]
fn reorder_widgets_then_undo_restores_the_order() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = layouts(data.path());
    s.reorder_widgets(None, &names(&["c", "a", "b"])).unwrap();
    s.undo().unwrap();
    assert_that!(items(&s)).is_equal_to(names(&["a", "b", "c"]));
}

#[test]
fn rename_widget_keeps_its_place_in_the_order() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = layouts(data.path());
    s.reorder_widgets(None, &names(&["c", "a", "b"])).unwrap();
    s.rename_widget(None, "a", "z").unwrap();
    assert_that!(items(&s)).is_equal_to(names(&["c", "z", "b"]));
}

#[test]
fn remove_widget_drops_it_from_the_order() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = layouts(data.path());
    s.reorder_widgets(None, &names(&["c", "a", "b"])).unwrap();
    s.remove_widget(None, "a").unwrap();
    assert_that!(items(&s)).is_equal_to(names(&["c", "b"]));
}

#[test]
fn duplicate_and_reorder_requests_answer_json() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = editing(data.path());
    s.set_widget(None, "kpi", KPI.into()).unwrap();
    let json = s.handle(Request::DuplicateWidget { pack: None, from: "kpi".into(), to: None }).unwrap();
    assert_that!(json.as_str()).contains("\"name\":\"kpi-2\"");
    s.handle(Request::ReorderWidgets { pack: None, order: names(&["kpi-2", "kpi"]) }).unwrap();
    assert_that!(items(&s)).is_equal_to(names(&["kpi-2", "kpi"]));
}
