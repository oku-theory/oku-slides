//! The `host-data` pane of a chart over a query data: its settings, and its query's result
//! read-only with its SQL, state and inputs, the columns it reads; no edit of its rows from
//! the pane (t-1793).

use oku_slides_a2ui::{ComponentKind, DataChoice, DataState, QueryView, ServerBody};
use oku_slides_i18n::{FluentValue, Localize};
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::HOST_DATA;
use crate::agent::Surface;
use crate::data_edit::DataEdit;
use crate::derived::derived_tests::{TOP, apply, with_query};

/// A query of three columns: `a`, `b` and `c`, twice `b`.
const WIDE: &str = "SELECT a, b, b * 2 AS c FROM ventes WHERE b > 0";

/// Adds table `tb` on `s-libre` over `q`.
fn add_table(s: &mut crate::Session) {
    apply(
        s,
        vec![json!({
            "op": "add_element",
            "slot": { "owner": { "slide": "s-libre" }, "param": "elements" },
            "element": { "id": "tb", "kind": "base/table", "frame": { "x": 0, "y": 300, "w": 400, "h": 200 },
                "params": { "data": { "data": "q" } } }
        })],
    );
}

/// Applies `edit` to `element` of `s-libre`.
fn edit(s: &mut crate::Session, element: &str, edit: &Value) -> crate::Result<()> {
    let edit: DataEdit = serde_json::from_value(edit.clone()).unwrap_or_else(|e| panic!("{e}"));
    s.data_edit("s-libre".into(), element, edit)
}

/// Binds `element`'s `param` to `q`, reading `columns`.
fn bind_q(s: &mut crate::Session, element: &str, param: &str, columns: &[&str]) {
    apply(
        s,
        vec![json!({ "op": "bind_data", "target": { "slide": "s-libre", "element": element }, "param": param,
            "data": "q", "columns": columns })],
    );
}

/// The components of `surface`.
fn components(surface: &Surface) -> Vec<ComponentKind> {
    surface
        .messages
        .iter()
        .filter_map(|m| match &m.body {
            ServerBody::UpdateComponents(u) => Some(u.components.iter().map(|c| c.kind.clone()).collect::<Vec<_>>()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// `op-data-derived` for `q`, as the French session says it.
fn derived(s: &crate::Session) -> String {
    s.localizer.tr_with("op-data-derived", &[("data", FluentValue::from("q"))])
}

/// The `ChartData` of the pane of chart `ch`: its source, kind, choices and query.
fn chart_pane(s: &crate::Session) -> (Vec<Vec<Value>>, Option<String>, Vec<String>, Option<QueryView>) {
    let surface = s.data_panel("s-libre", "ch").unwrap_or_else(|| panic!("no pane for the chart over q"));
    assert_that!(surface.id.as_str()).is_equal_to(HOST_DATA);
    let found = components(&surface).into_iter().find_map(|c| match c {
        ComponentKind::ChartData { source, kind, choices, query, .. } => {
            Some((source, kind, choices.into_iter().map(|c| c.id).collect(), query))
        }
        _ => None,
    });
    found.unwrap_or_else(|| panic!("ChartData expected"))
}

#[test]
fn element_panel_of_a_chart_over_a_query_shows_its_result() {
    let (mut s, _dir) = with_query(TOP);
    s.settle_queries();
    let inputs = vec![DataChoice { id: "m".into(), name: "ventes".into(), label: None }];
    let ready = QueryView { sql: TOP.into(), state: DataState::Ready, error: None, inputs };
    let rows: Vec<Vec<Value>> = vec![vec![json!("a"), json!("b")], vec![json!("x"), json!(1.0)]];
    let choices = vec!["m".to_owned(), "q".to_owned()];
    assert_that!(chart_pane(&s)).is_equal_to((rows, Some("bar".to_owned()), choices, Some(ready)));
}

#[test]
fn element_panel_of_a_chart_over_a_pending_query_shows_its_state() {
    let (s, _dir) = with_query(TOP);
    let (source, _, _, query) = chart_pane(&s);
    assert_that!((source, query.map(|q| q.state))).is_equal_to((vec![], Some(DataState::Pending)));
}

#[test]
fn element_panel_view_edit_on_a_query_view_applies() {
    let (mut s, _dir) = with_query(TOP);
    add_table(&mut s);
    s.settle_queries();
    let edits = [
        ("ch", json!({ "edit": "color", "index": 0, "color": "#112233" })),
        ("tb", json!({ "edit": "flag", "flag": "striped", "on": true })),
        ("tb", json!({ "edit": "align", "column": 1, "align": "end" })),
        ("ch", json!({ "edit": "bind", "data": "m" })),
    ];
    for (element, change) in edits {
        let before = s.deck().clone();
        let done = edit(&mut s, element, &change).map_err(|e| e.localize(&s.localizer));
        assert_that!(done).named(&change.to_string()).is_ok();
        assert_that!(*s.deck()).named(&change.to_string()).is_not_equal_to(&before);
    }
}

#[test]
fn element_panel_stack_on_a_query_view_of_two_series_applies() {
    let (mut s, _dir) = with_query(WIDE);
    s.settle_queries();
    let two = json!({ "series": [{ "type": "bar" }, { "type": "bar" }] });
    apply(
        &mut s,
        vec![json!({ "op": "set_chart_option", "target": { "slide": "s-libre", "element": "ch" }, "param": "option",
            "patch": two })],
    );
    let before = s.deck().clone();
    let done = edit(&mut s, "ch", &json!({ "edit": "stack", "stacked": true })).map_err(|e| e.localize(&s.localizer));
    assert_that!(done).is_ok();
    assert_that!(*s.deck()).is_not_equal_to(&before);
}

#[test]
fn element_panel_align_on_a_pending_query_view_is_refused() {
    let (mut s, _dir) = with_query(TOP);
    add_table(&mut s);
    let before = s.deck().clone();
    let err = edit(&mut s, "tb", &json!({ "edit": "align", "column": 0, "align": "end" })).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::QueryNoResult);
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    let refusal = fr.tr_with("session-data-query-no-result", &[("data", FluentValue::from("top"))]);
    assert_that!(err.localize(&fr)).is_equal_to(refusal);
    assert_that!(err.localize(&fr)).contains("top");
    assert_that!(*s.deck()).is_equal_to(&before);
}

#[test]
fn element_panel_of_a_view_reading_some_query_columns_shows_only_those() {
    let (mut s, _dir) = with_query(WIDE);
    add_table(&mut s);
    s.settle_queries();
    bind_q(&mut s, "ch", "option", &["a", "c"]);
    bind_q(&mut s, "tb", "data", &["a", "c"]);
    let rows: Vec<Vec<Value>> = vec![vec![json!("a"), json!("c")], vec![json!("x"), json!(2.0)]];
    assert_that!(chart_pane(&s).0).is_equal_to(&rows);
    let surface = s.data_panel("s-libre", "tb").unwrap_or_else(|| panic!("no pane for the table over q"));
    let table = components(&surface).into_iter().find_map(|c| match c {
        ComponentKind::TableData { source, .. } => Some(source),
        _ => None,
    });
    assert_that!(table).is_equal_to(Some(rows));
}

#[test]
fn element_panel_edit_on_a_query_view_is_refused() {
    let (mut s, _dir) = with_query(TOP);
    add_table(&mut s);
    s.settle_queries();
    let before = s.deck().clone();
    let edits = [
        json!({ "edit": "cells", "change": { "op": "set_cell", "row": "r0", "column": "a", "value": "y" } }),
        json!({ "edit": "paste", "row": "r0", "column": "a", "cells": [["y"]] }),
        json!({ "edit": "add_row" }),
        // Converting the chart reads its rows too.
        json!({ "edit": "kind", "kind": "line" }),
        json!({ "edit": "show_as", "view": "table" }),
    ];
    let edits = edits.into_iter().map(|e| ("ch", e)).chain([("tb", json!({ "edit": "show_as", "view": "chart" }))]);
    for (element, change) in edits {
        let err = edit(&mut s, element, &change).unwrap_err();
        assert_that!(err.localize(&s.localizer)).named(&change.to_string()).ends_with(derived(&s).as_str());
    }
    assert_that!(*s.deck()).is_equal_to(&before);
}
