use oku_slides_chart::ColumnAlign;
use oku_slides_core::{Deck, Op, Target, WidgetRegistry, apply_all};
use oku_slides_data::DataId;
use oku_slides_i18n::{Localize, Localizer};
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{ViewFollow, align_patch, color_patch, data_views, stack_patch};

#[test]
fn align_patch_keeps_other_columns_and_trims_trailing_auto() {
    let option = json!({ "align": ["end", "center"] });
    assert_that!(align_patch(&option, 3, 1, ColumnAlign::Auto)).is_equal_to(json!({ "align": ["end"] }));
}

#[test]
fn align_patch_all_auto_is_null() {
    let option = json!({ "align": ["end"] });
    assert_that!(align_patch(&option, 1, 0, ColumnAlign::Auto)).is_equal_to(json!({ "align": null }));
}

#[test]
fn stack_patch_unstacked_drops_every_stack() {
    let option = json!({ "series": [{ "type": "bar", "stack": "a" }, { "type": "bar", "smooth": true }] });
    assert_that!(stack_patch(&option, false))
        .is_equal_to(json!({ "series": [{ "type": "bar" }, { "type": "bar", "smooth": true }] }));
}

#[test]
fn color_patch_touches_only_its_series() {
    let option = json!({ "series": [{ "type": "bar" }, { "type": "bar", "itemStyle": { "color": "#000000" } }] });
    assert_that!(color_patch(&option, 0, Some("#112233"))).is_equal_to(
        json!({ "series": [{ "type": "bar", "itemStyle": { "color": "#112233" } }, { "type": "bar", "itemStyle": { "color": "#000000" } }] }),
    );
}

/// A registry of layout `l/page`, a slide with a chart param `option`, and of element `t/two`,
/// with two chart params `a` and `b`.
fn two_views_registry() -> WidgetRegistry {
    let chart = |name: &str| json!({ "name": name, "type": "host", "host": "chart" });
    let slot = json!({ "name": "blocks", "type": "slot", "mode": "flow" });
    let page = json!({ "name": "page", "label": null, "category": "slide", "params": [chart("option"), slot] });
    let two = json!({ "name": "two", "label": null, "category": "element", "params": [chart("a"), chart("b")] });
    let probe = json!({ "packs": [
        { "namespace": "l", "kind": "layouts", "widgets": [page] },
        { "namespace": "t", "kind": "widgets", "widgets": [two] },
    ] });
    WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("{e}"))
}

fn chart_on(data: &str) -> Value {
    json!({ "data": data, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] })
}

/// A deck with data `d` (`c1`, `c2`), viewed by slide `s`'s own chart and by both charts of
/// its element `e`.
fn two_views_deck() -> Deck {
    let slide = json!({ "slides": [{ "id": "s", "kind": "l/page", "params": { "option": chart_on("d"),
        "blocks": [{ "id": "e", "kind": "t/two", "params": { "a": chart_on("d"), "b": chart_on("d") } }] } }] });
    let mut deck = Deck::from_json(&slide.to_string()).unwrap_or_else(|e| panic!("{e}"));
    let table = json!({ "generation": "g", "columns": [{ "id": "c1", "name": "x" }, { "id": "c2", "name": "y" }],
        "rows": [{ "id": "r1", "cells": ["a", 1] }] });
    let add = json!({ "op": "add_data", "id": "d", "name": "d", "table": table });
    let add: Op = serde_json::from_value(add).unwrap_or_else(|e| panic!("{e}"));
    apply_all(&mut deck, vec![add], &two_views_registry()).unwrap_or_else(|e| panic!("{e:?}"));
    deck
}

fn d() -> DataId {
    DataId::try_from("d".to_owned()).unwrap_or_else(|e| panic!("{e}"))
}

fn followed(views: &[ViewFollow]) -> Vec<(Target, String, Vec<String>)> {
    views.iter().map(|v| (v.owner.clone(), v.target.param.clone(), v.read.clone())).collect()
}

#[test]
fn data_views_follows_each_param_once_slide_params_included() {
    let views =
        data_views(&two_views_deck(), &two_views_registry(), &d()).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let read = vec!["c1".to_owned(), "c2".to_owned()];
    let on = |element: Option<&str>| Target { slide: "s".into(), element: element.map(str::to_owned) };
    assert_that!(followed(&views)).is_equal_to(vec![
        (on(None), "option".to_owned(), read.clone()),
        (on(Some("e")), "a".to_owned(), read.clone()),
        (on(Some("e")), "b".to_owned(), read),
    ]);
}

#[test]
fn data_views_refuses_a_view_its_registry_cannot_read() {
    let Err(e) = data_views(&two_views_deck(), &WidgetRegistry::default(), &d()) else {
        panic!("a view no widget reads was followed")
    };
    let (en, fr) = (Localizer::new(Some("en")), Localizer::new(Some("fr")));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr_with("session-data-view-unfollowed", &[("view", "s".into())]));
    assert_that!(e.localize(&fr)).is_not_equal_to(e.localize(&en));
}
