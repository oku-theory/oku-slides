use oku_slides_core::Op;
use oku_slides_i18n::{FluentValue, Localizer};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::data_undo_label;
use crate::Session;
use crate::test_support::{session, undo_label};

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap()
}

/// A table of months and sales: columns `c1` and `c2`, rows `r1` and `r2`.
fn sales() -> Value {
    json!({
        "generation": "g1",
        "columns": [{ "id": "c1", "name": "Month" }, { "id": "c2", "name": "sales" }],
        "rows": [{ "id": "r1", "cells": ["Jan", 10] }, { "id": "r2", "cells": ["Feb", 12] }]
    })
}

fn add_data(id: &str, name: &str) -> Op {
    op(json!({ "op": "add_data", "id": id, "name": name, "table": sales() }))
}

fn edit_d2(change: &Value) -> Op {
    op(json!({ "op": "edit_data", "data": "d2", "change": change }))
}

/// The roadmap deck with the data `d1` and `d2` and a chart `ch` showing `d1`.
fn session_with_data() -> Session {
    let mut s = session();
    let chart = json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-libre" }, "param": "elements" },
        "element": {
            "id": "ch",
            "kind": "base/chart",
            "frame": { "x": 0, "y": 0, "w": 400, "h": 240 },
            "params": { "option": { "data": "d1", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } }
        }
    });
    s.apply("setup", vec![add_data("d1", "sales"), add_data("d2", "months"), op(chart)], None).unwrap();
    s
}

#[rstest]
#[case::add_data(add_data("d3", "costs"))]
#[case::set_cell(edit_d2(&json!({ "op": "set_cell", "row": "r1", "column": "c2", "value": 11 })))]
#[case::rename_column(edit_d2(&json!({ "op": "rename_column", "column": "c2", "name": "costs" })))]
#[case::insert_row(edit_d2(&json!({ "op": "insert_row", "after": "r1", "cells": { "c1": "Mar" } })))]
#[case::remove_row(edit_d2(&json!({ "op": "remove_row", "row": "r2" })))]
#[case::insert_column(edit_d2(&json!({ "op": "insert_column", "name": "margin", "cells": { "r1": 3 } })))]
#[case::remove_column(edit_d2(&json!({ "op": "remove_column", "column": "c2" })))]
#[case::replace(edit_d2(&json!({ "op": "replace", "table": { "columns": [{ "name": "Year" }], "rows": [{ "cells": [2026] }] } })))]
#[case::bind_data(op(json!({ "op": "bind_data", "target": { "slide": "s-libre", "element": "ch" }, "param": "option", "data": "d2" })))]
#[case::remove_data(op(json!({ "op": "remove_data", "data": "d2" })))]
#[case::rename_data(op(json!({ "op": "rename_data", "data": "d2", "name": "mois" })))]
#[case::set_column_type(op(json!({ "op": "set_column_type", "data": "d2", "column": "c2", "type": "text" })))]
fn undo_data_op_restores_deck_then_redo(#[case] data_op: Op) {
    let mut s = session_with_data();
    let label = data_undo_label(&Localizer::new(Some("fr")), std::slice::from_ref(&data_op)).unwrap();
    let before = s.deck().clone();
    s.apply(&label, vec![data_op], None).unwrap();
    let after = s.deck().clone();
    assert_that!(after).is_not_equal_to(&before);
    assert_that!(undo_label(&s)).is_equal_to(Some(label));

    s.undo().unwrap();
    assert_that!(*s.deck()).is_equal_to(&before);
    s.redo().unwrap();
    assert_that!(*s.deck()).is_equal_to(&after);
}

#[rstest]
#[case::add(vec![add_data("d1", "a"), add_data("d2", "b")], Some("Add data"))]
#[case::edit(vec![edit_d2(&json!({ "op": "remove_row", "row": "r1" }))], Some("Edit data"))]
#[case::mixed(vec![add_data("d1", "a"), edit_d2(&json!({ "op": "remove_row", "row": "r1" }))], None)]
#[case::other(vec![op(json!({ "op": "remove_slide", "slide": "s-fin" }))], None)]
#[case::empty(vec![], None)]
fn data_undo_label_names_a_batch_of_one_data_kind(#[case] ops: Vec<Op>, #[case] expected: Option<&str>) {
    let l = Localizer::new(Some("en"));
    assert_that!(data_undo_label(&l, &ops).as_deref()).is_equal_to(expected);
}

/// Each key Task 4a adds, with sample arguments.
fn task4a_samples() -> Vec<(&'static str, Vec<(&'static str, FluentValue<'static>)>)> {
    vec![
        ("op-data-missing", vec![("data", "d9".into())]),
        ("op-data-name-taken", vec![("name", "sales".into())]),
        ("op-data-column-in-use", vec![("column", "c2".into()), ("views", "s/ch, s/tb".into())]),
        ("op-data-column-missing", vec![("column", "c9".into()), ("data", "d1".into())]),
        ("op-data-not-view", vec![("param", "items".into())]),
        ("op-data-truncated", vec![("data", "d1".into())]),
        ("op-data-rebind", vec![("subject", "ch".into())]),
        ("op-data-internal", vec![]),
        ("op-data-batch-too-large", vec![("count", 100_001.into()), ("max", 100_000.into())]),
        ("chart-ref-and-inline", vec![]),
        ("chart-ref-null", vec![]),
        ("chart-columns-invalid", vec![]),
        ("session-undo-data-add", vec![]),
        ("session-undo-data-edit", vec![]),
        ("session-undo-data-bind", vec![]),
    ]
}

#[test]
fn every_task4a_key_formats_in_all_locales() {
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        for (key, args) in task4a_samples() {
            assert_that!(l.has(key)).named(&format!("{locale} {key}")).is_true();
            let text = l.tr_with(key, &args);
            assert_that!(text.contains('{')).named(&format!("{locale} {key}: {text}")).is_false();
            assert_that!(text.trim().is_empty()).named(&format!("{locale} {key}")).is_false();
        }
    }
    let (en, fr) = (Localizer::new(Some("en")), Localizer::new(Some("fr")));
    for (key, args) in task4a_samples() {
        assert_that!(fr.tr_with(key, &args)).named(key).is_not_equal_to(en.tr_with(key, &args));
    }
}

/// Each key Task 5 adds, with sample arguments.
fn task5_samples() -> Vec<(&'static str, Vec<(&'static str, FluentValue<'static>)>)> {
    let in_use = |more: usize| vec![("views", "ch, tb".into()), ("more", more.into())];
    vec![
        ("op-data-in-use", in_use(0)),
        ("op-data-in-use", in_use(1)),
        ("op-data-in-use", in_use(5)),
        ("session-undo-data-remove", vec![]),
        ("session-undo-data-rename", vec![]),
        ("session-undo-data-type", vec![]),
        ("session-data-base-stale", vec![("data", "sales".into())]),
    ]
}

#[test]
fn every_task5_key_formats_in_all_locales() {
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        for (key, args) in task5_samples() {
            assert_that!(l.has(key)).named(&format!("{locale} {key}")).is_true();
            let text = l.tr_with(key, &args);
            assert_that!(text.contains('{')).named(&format!("{locale} {key}: {text}")).is_false();
            assert_that!(text.trim().is_empty()).named(&format!("{locale} {key}")).is_false();
        }
    }
    let (en, fr) = (Localizer::new(Some("en")), Localizer::new(Some("fr")));
    for (key, args) in task5_samples() {
        assert_that!(fr.tr_with(key, &args)).named(key).is_not_equal_to(en.tr_with(key, &args));
    }
    // The views listed alone, then with the count of those left out.
    let in_use = |more: usize| fr.tr_with("op-data-in-use", &[("views", "ch".into()), ("more", more.into())]);
    assert_that!(in_use(0)).is_not_equal_to(in_use(1));
    assert_that!(in_use(1).contains('1')).is_true();
}

/// `op-data-in-use` picks the plural arm `$more` asks for in ru and pl.
#[rstest]
#[case::ru_1("ru", 1, "ещё 1 представление;")]
#[case::ru_2("ru", 2, "ещё 2 представления;")]
#[case::ru_5("ru", 5, "ещё 5 представлений;")]
#[case::ru_21("ru", 21, "ещё 21 представление;")]
#[case::ru_22("ru", 22, "ещё 22 представления;")]
#[case::pl_1("pl", 1, "i 1 inny widok;")]
#[case::pl_2("pl", 2, "i 2 inne widoki;")]
#[case::pl_5("pl", 5, "i 5 innych widoków;")]
fn data_in_use_more_selects_its_plural_form(#[case] locale: &str, #[case] more: usize, #[case] form: &str) {
    let l = Localizer::new(Some(locale));
    let text = l.tr_with("op-data-in-use", &[("views", "ch".into()), ("more", more.into())]);
    let plain: String = text.chars().filter(|c| !matches!(c, '\u{2068}' | '\u{2069}')).collect();
    assert_that!(plain.contains(form)).named(&plain).is_true();
}

#[rstest]
#[case::remove(vec![op(json!({ "op": "remove_data", "data": "d2" }))], "session-undo-data-remove")]
#[case::rename(vec![op(json!({ "op": "rename_data", "data": "d2", "name": "mois" }))], "session-undo-data-rename")]
#[case::retype(vec![op(json!({ "op": "set_column_type", "data": "d2", "column": "c2", "type": "text" }))], "session-undo-data-type")]
fn data_undo_label_names_task5_ops(#[case] ops: Vec<Op>, #[case] key: &str) {
    let fr = Localizer::new(Some("fr"));
    assert_that!(data_undo_label(&fr, &ops)).is_equal_to(Some(fr.tr(key)));
}

fn add_query(id: &str, name: &str, sql: &str) -> Op {
    op(json!({ "op": "add_data", "id": id, "name": name, "sql": sql }))
}

fn sql_of<'a>(s: &'a Session, id: &str) -> Option<&'a str> {
    s.deck().data.iter().find(|(k, _)| k.as_str() == id).and_then(|(_, d)| d.sql())
}

#[test]
fn undo_set_query_restores_sql() {
    let mut s = session_with_data();
    s.apply("query", vec![add_query("d3", "top", "SELECT * FROM sales")], None).unwrap();
    let set = op(json!({ "op": "set_query", "data": "d3", "sql": "SELECT * FROM months" }));
    let fr = Localizer::new(Some("fr"));
    let label = data_undo_label(&fr, std::slice::from_ref(&set)).unwrap();
    assert_that!(label.as_str()).is_equal_to(fr.tr("session-undo-data-query").as_str());
    s.apply(&label, vec![set], None).unwrap();
    assert_that!(sql_of(&s, "d3")).is_equal_to(Some("SELECT * FROM months"));
    assert_that!(undo_label(&s)).is_equal_to(Some(label));

    s.undo().unwrap();
    assert_that!(sql_of(&s, "d3")).is_equal_to(Some("SELECT * FROM sales"));
    s.redo().unwrap();
    assert_that!(sql_of(&s, "d3")).is_equal_to(Some("SELECT * FROM months"));
}

#[test]
fn rename_data_rewrites_reader_sql_one_undo_step() {
    let mut s = session_with_data();
    s.apply("query", vec![add_query("d3", "top", "SELECT Month FROM sales WHERE sales > 10")], None).unwrap();
    let before = s.deck().clone();
    s.apply("rename", vec![op(json!({ "op": "rename_data", "data": "d1", "name": "ventes" }))], None).unwrap();
    let rewritten = sql_of(&s, "d3").unwrap().to_owned();
    let inputs = oku_slides_data::sql::parse_query(&rewritten).unwrap().inputs();
    assert_that!(inputs.into_iter().collect::<Vec<_>>()).is_equal_to(vec!["ventes".to_owned()]);

    // One undo puts back both the name and the reader's query.
    s.undo().unwrap();
    assert_that!(*s.deck()).is_equal_to(&before);
    s.redo().unwrap();
    assert_that!(sql_of(&s, "d3")).is_equal_to(Some(rewritten.as_str()));
}
