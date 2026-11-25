//! The data panel: the deck's data listed, its actions, and one data's rows paged.

use std::path::Path;
use std::time::Instant;

use oku_slides_a2ui::{ComponentKind, DataRow, DataUser, ServerBody};
use oku_slides_core::Op;
use oku_slides_data::{ColumnType, DataId};
use oku_slides_i18n::{FluentValue, Localize, Localizer};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{DATA_SHEET, DECK_DATA, DataAction, data_list};
use crate::agent::Surface;
use crate::test_support::{fresh_dir, session, session_on, undo_label, workspace_on};
use crate::{Request, Session, Workspace};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");

fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("{e}"))
}

fn id(s: &str) -> DataId {
    DataId::try_from(s.to_owned()).unwrap_or_else(|e| panic!("{e}"))
}

/// `d9` (columns `c1` text, `c2` number; rows `r1`, `r2`) and the chart `ch1` on `s-libre` reading it.
fn data_and_chart() -> Vec<Op> {
    let table = json!({ "generation": "g1",
        "columns": [{ "id": "c1", "name": "Mois", "type": "text" }, { "id": "c2", "name": "ventes", "type": "number" }],
        "rows": [{ "id": "r1", "cells": ["Jan", 7] }, { "id": "r2", "cells": ["Fév", 41] }] });
    vec![
        op(json!({ "op": "add_data", "id": "d9", "name": "ventes", "label": "Ventes", "table": table })),
        op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-libre" }, "param": "elements" },
            "element": { "id": "ch1", "frame": { "x": 0, "y": 0, "w": 400, "h": 300 }, "kind": "base/chart",
                "params": { "option": { "data": "d9", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } } })),
    ]
}

/// The unused data `d5`: one column, three rows.
fn unused() -> Op {
    let table = json!({ "generation": "g1", "columns": [{ "id": "c1", "name": "x" }],
        "rows": [{ "id": "r1", "cells": [1] }, { "id": "r2", "cells": [2] }, { "id": "r3", "cells": [3] }] });
    op(json!({ "op": "add_data", "id": "d5", "name": "spare", "table": table }))
}

fn with_data(ops: Vec<Op>) -> Session {
    let mut s = session();
    s.apply("setup", ops, None).unwrap_or_else(|e| panic!("apply: {}", e.one_line()));
    s
}

fn act(s: &mut Session, action: DataAction) -> crate::Result<String> {
    s.handle(Request::DataAction { action })
}

fn components(s: &Surface) -> Vec<ComponentKind> {
    s.messages
        .iter()
        .filter_map(|m| match &m.body {
            ServerBody::UpdateComponents(u) => Some(u.components.clone()),
            _ => None,
        })
        .flatten()
        .map(|c| c.kind)
        .collect()
}

/// The rows of the `deck-data` surface of `s`.
fn listed(s: &Session) -> Vec<DataRow> {
    let surface = s.data_list_surface().unwrap_or_else(|| panic!("no data list"));
    assert_that!(surface.id.as_str()).is_equal_to(DECK_DATA);
    let found = components(&surface).into_iter().find_map(|c| match c {
        ComponentKind::DataList { rows, .. } => Some(rows),
        _ => None,
    });
    found.unwrap_or_else(|| panic!("DataList expected"))
}

/// The `DataSheet` of a `data-sheet` surface: source, row ids, types, offset, total.
fn sheet(surface: &Surface) -> (Vec<Vec<Value>>, Vec<String>, Vec<String>, u32, u32) {
    assert_that!(surface.id.as_str()).is_equal_to(DATA_SHEET);
    let found = components(surface).into_iter().find_map(|c| match c {
        ComponentKind::DataSheet { source, row_ids, types, offset, total, .. } => {
            Some((source, row_ids, types, offset, total))
        }
        _ => None,
    });
    found.unwrap_or_else(|| panic!("DataSheet expected"))
}

#[test]
fn data_list_reports_kind_size_and_users() {
    let s = with_data(data_and_chart());
    let fr = Localizer::new(Some("fr"));
    let rows = data_list(s.deck(), &fr);
    let label = fr.tr_with("session-data-user", &[("slide", 7.into()), ("element", "ch1".into())]);
    let user = DataUser { slide: "s-libre".into(), element: "ch1".into(), label };
    assert_that!(rows).is_equal_to(vec![DataRow {
        id: "d9".into(),
        name: "ventes".into(),
        label: Some("Ventes".into()),
        kind: "manual".into(),
        columns: 2,
        rows: 2,
        users: vec![user],
        error: None,
    }]);
}

#[test]
fn data_list_shows_unused_data() {
    let mut ops = data_and_chart();
    ops.push(unused());
    let s = with_data(ops);
    let rows = listed(&s);
    let spare = rows.iter().find(|r| r.id == "d5").unwrap_or_else(|| panic!("d5 not listed"));
    assert_that!((spare.columns, spare.rows, spare.users.len())).is_equal_to((1, 3, 0));
    assert_that!(rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["d5", "d9"]);
}

#[test]
fn add_action_creates_named_data_one_undo_step() {
    let mut s = session();
    let before = s.deck().clone();
    act(&mut s, DataAction::Add).unwrap_or_else(|e| panic!("add: {}", e.one_line()));
    let deck = s.deck();
    assert_that!(deck.data.len()).is_equal_to(1);
    let data = deck.data.values().next().unwrap_or_else(|| panic!("no data"));
    let columns: Vec<_> = data.table.columns.iter().map(|c| (c.name.clone(), c.ty)).collect();
    assert_that!((data.name.as_str(), columns, data.table.rows.len())).is_equal_to((
        "data",
        vec![("a".to_owned(), ColumnType::Text)],
        1,
    ));
    assert_that!(undo_label(&s)).is_equal_to(Some(s.localizer.tr("session-undo-data-add")));
    s.undo().unwrap_or_else(|e| panic!("undo: {}", e.one_line()));
    assert_that!(*s.deck()).is_equal_to(&before);
}

#[test]
fn add_action_twice_gives_data_and_data_2() {
    let mut s = session();
    act(&mut s, DataAction::Add).unwrap_or_else(|e| panic!("add: {}", e.one_line()));
    act(&mut s, DataAction::Add).unwrap_or_else(|e| panic!("add: {}", e.one_line()));
    let mut names: Vec<_> = s.deck().data.values().map(|d| d.name.clone()).collect();
    names.sort();
    assert_that!(names).is_equal_to(vec!["data".to_owned(), "data_2".to_owned()]);
}

#[test]
fn remove_in_use_shows_views_error() {
    let mut s = with_data(data_and_chart());
    let Err(e) = act(&mut s, DataAction::Remove { data: "d9".into() }) else { panic!("removed a used data") };
    let shown = e.localize(&s.localizer);
    let in_use = s.localizer.tr_with("op-data-in-use", &[("views", "ch1".into()), ("more", 0.into())]);
    assert_that!(shown.as_str()).contains(in_use.as_str());
    let d9 = listed(&s).into_iter().find(|r| r.id == "d9").unwrap_or_else(|| panic!("d9 gone"));
    assert_that!(d9.error).is_equal_to(Some(shown));
    // The next edit clears it.
    act(&mut s, DataAction::Add).unwrap_or_else(|e| panic!("add: {}", e.one_line()));
    assert_that!(listed(&s).iter().all(|r| r.error.is_none())).is_true();
}

#[test]
fn set_type_action_refusal_names_rows() {
    let mut s = with_data(data_and_chart());
    let retype = DataAction::SetType { data: "d9".into(), column: "c1".into(), ty: ColumnType::Number };
    let Err(e) = act(&mut s, retype) else { panic!("text retyped to number") };
    let shown = e.localize(&Localizer::new(Some("fr")));
    assert_that!(shown.as_str()).contains("r1");
    let d9 = listed(&s).into_iter().find(|r| r.id == "d9").unwrap_or_else(|| panic!("d9 gone"));
    assert_that!(d9.error.unwrap_or_default().as_str()).contains("r1");
    assert_that!(s.deck().data[&id("d9")].table.columns[0].ty).is_equal_to(ColumnType::Text);
}

#[test]
fn open_data_action_opens_pane_on_data() {
    let s = with_data(data_and_chart());
    let read = s.data_sheet("d9", 0).unwrap_or_else(|| panic!("no sheet"));
    let surface = read.surface().unwrap_or_else(|| panic!("no surface"));
    let (source, row_ids, types, offset, total) = sheet(&surface);
    assert_that!(source).is_equal_to(vec![
        vec![json!("Mois"), json!("ventes")],
        vec![json!("Jan"), json!(7.0)],
        vec![json!("Fév"), json!(41.0)],
    ]);
    assert_that!((row_ids, types, offset, total)).is_equal_to((
        vec!["r1".into(), "r2".into()],
        vec!["text".into(), "number".into()],
        0,
        2,
    ));
}

#[test]
fn sheet_of_a_missing_data_is_none() {
    let s = with_data(data_and_chart());
    assert_that!(s.data_sheet("d1", 0).is_none()).is_true();
}

/// Opens the fixture with `d9` and its chart, then folds `d9` onto a base.
fn based(s: &mut Session) {
    s.open(Path::new(FIXTURE)).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    s.apply("données", data_and_chart(), None).unwrap_or_else(|e| panic!("apply: {}", e.one_line()));
    let now = Instant::now();
    let job = s.compaction_job(&id("d9"), now, true).unwrap_or_else(|| panic!("no job"));
    let reply = job.run();
    assert_that!(s.finish_compaction(job, reply, now)).is_true();
    assert_that!(s.deck().data[&id("d9")].table.base.is_some()).is_true();
}

#[test]
fn page_action_reads_rows_from_base() {
    let dir = fresh_dir("oku-slides-data-sheet-");
    let ws = workspace_on(dir.path());
    let tab = Workspace::lock(&ws).view().windows[0].tabs[0].id.clone();
    Workspace::with_session(&ws, &tab, based).0.unwrap_or_else(|e| panic!("{}", e.one_line()));
    let req = Request::DataSheet { data: "d9".to_owned(), offset: 1 };
    let (reply, _) = Workspace::request(&ws, &tab, req);
    let reply = reply.unwrap_or_else(|e| panic!("sheet: {}", e.one_line()));
    let surface: Surface = serde_json::from_str(&reply).unwrap_or_else(|e| panic!("{e}"));
    let (source, row_ids, _, offset, total) = sheet(&surface);
    assert_that!((source.len(), row_ids, offset, total)).is_equal_to((2, vec!["r2".to_owned()], 1, 2));
    assert_that!(source[1][0].clone()).is_equal_to(json!("Fév"));
}

#[test]
fn rename_action_renames_and_refused_name_shows_inline() {
    let mut s = with_data(data_and_chart());
    act(&mut s, DataAction::Rename { data: "d9".into(), name: "sales".into() })
        .unwrap_or_else(|e| panic!("rename: {}", e.one_line()));
    assert_that!(s.deck().data[&id("d9")].name.as_str()).is_equal_to("sales");
    assert_that!(act(&mut s, DataAction::Rename { data: "d9".into(), name: "Not Valid".into() }).is_err()).is_true();
    let d9 = listed(&s).into_iter().find(|r| r.id == "d9").unwrap_or_else(|| panic!("d9 gone"));
    assert_that!(d9.error.is_some()).is_true();
}

/// Each key Task 7 adds, with sample arguments.
fn task7_samples() -> Vec<(&'static str, Vec<(&'static str, FluentValue<'static>)>)> {
    let views = |count: usize| vec![("count", count.into())];
    vec![
        ("session-data-user", vec![("slide", 3.into()), ("element", "ch1".into())]),
        ("ui-data-tab", vec![]),
        ("ui-data-add", vec![]),
        ("ui-data-rename", vec![]),
        ("ui-data-remove", vec![]),
        ("ui-data-column-type", vec![]),
        ("ui-data-type-text", vec![]),
        ("ui-data-type-number", vec![]),
        ("ui-data-type-integer", vec![]),
        ("ui-data-type-bool", vec![]),
        ("ui-data-type-date", vec![]),
        ("ui-data-views", views(1)),
        ("ui-data-views", views(4)),
        ("ui-data-unused", vec![]),
        ("ui-data-size", vec![("columns", 3.into()), ("rows", 12.into())]),
        ("ui-data-page", vec![("first", 101.into()), ("last", 200.into()), ("total", 250.into())]),
        ("ui-data-previous", vec![]),
        ("ui-data-next", vec![]),
        ("ui-data-open", vec![]),
        ("session-data-view-unfollowed", vec![("view", "ch1".into())]),
    ]
}

#[test]
fn every_task7_key_formats_in_all_locales() {
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        for (key, args) in task7_samples() {
            assert_that!(l.has(key)).named(&format!("{locale} {key}")).is_true();
            let text = l.tr_with(key, &args);
            assert_that!(text.contains('{')).named(&format!("{locale} {key}: {text}")).is_false();
            assert_that!(text.trim().is_empty()).named(&format!("{locale} {key}")).is_false();
        }
    }
    let (en, fr) = (Localizer::new(Some("en")), Localizer::new(Some("fr")));
    for (key, args) in task7_samples() {
        assert_that!(fr.tr_with(key, &args)).named(key).is_not_equal_to(en.tr_with(key, &args));
    }
}

/// The data list names how many views show a data, in the plural form `count` asks for.
#[rstest]
#[case::ru_1("ru", 1, "1 представление")]
#[case::ru_2("ru", 2, "2 представления")]
#[case::ru_5("ru", 5, "5 представлений")]
#[case::ru_21("ru", 21, "21 представление")]
#[case::ru_22("ru", 22, "22 представления")]
#[case::pl_1("pl", 1, "1 widok")]
#[case::pl_2("pl", 2, "2 widoki")]
#[case::pl_5("pl", 5, "5 widoków")]
fn ui_data_views_selects_its_plural_form(#[case] locale: &str, #[case] count: usize, #[case] form: &str) {
    let text = Localizer::new(Some(locale)).tr_with("ui-data-views", &[("count", count.into())]);
    let plain: String = text.chars().filter(|c| !matches!(c, '\u{2068}' | '\u{2069}')).collect();
    assert_that!(plain.as_str()).named(&plain).is_equal_to(form);
}

/// `d9` with three columns (`c1` text, `c2` and `c3` numbers) and the chart `ch1` on `s-libre`
/// reading it without a projection: one bar series per number column.
fn wide_chart() -> Vec<Op> {
    let table = json!({ "generation": "g1",
        "columns": [{ "id": "c1", "name": "Mois", "type": "text" }, { "id": "c2", "name": "ventes", "type": "number" },
            { "id": "c3", "name": "marge", "type": "number" }],
        "rows": [{ "id": "r1", "cells": ["Jan", 7, 2] }, { "id": "r2", "cells": ["Fév", 41, 5] }] });
    vec![
        op(json!({ "op": "add_data", "id": "d9", "name": "ventes", "table": table })),
        op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-libre" }, "param": "elements" },
            "element": { "id": "ch1", "frame": { "x": 0, "y": 0, "w": 400, "h": 300 }, "kind": "base/chart",
                "params": { "option": { "data": "d9", "xAxis": { "type": "category" },
                    "series": [{ "type": "bar" }, { "type": "line" }] } } } })),
    ]
}

/// The option of chart `ch1` on `s-libre`.
fn chart_option(s: &Session) -> Value {
    let deck = s.deck();
    let slide = deck.slide("s-libre").unwrap_or_else(|| panic!("no s-libre"));
    let located = slide.locate("ch1").unwrap_or_else(|| panic!("no ch1"));
    match located.element.params.get("option") {
        Some(oku_slides_core::ParamValue::Value(v)) => v.clone(),
        other => panic!("option expected, got {other:?}"),
    }
}

/// The series types of chart `ch1`.
fn series_types(s: &Session) -> Vec<Value> {
    let option = chart_option(s);
    option["series"].as_array().unwrap_or_else(|| panic!("no series")).iter().map(|x| x["type"].clone()).collect()
}

/// The source of the first sheet page of `data`.
fn sheet_source(s: &Session, data: &str) -> Vec<Vec<Value>> {
    let read = s.data_sheet(data, 0).unwrap_or_else(|| panic!("no sheet"));
    sheet(&read.surface().unwrap_or_else(|| panic!("no surface"))).0
}

fn cells(data: &str, change: Value) -> DataAction {
    DataAction::Cells { data: data.into(), change: serde_json::from_value(change).unwrap_or_else(|e| panic!("{e}")) }
}

#[test]
fn cells_action_remove_column_keeps_chart_without_projection_in_step() {
    let mut s = with_data(wide_chart());
    act(&mut s, cells("d9", json!({ "op": "remove_column", "column": "c2" })))
        .unwrap_or_else(|e| panic!("remove: {}", e.one_line()));
    // The line series read `c3`: it stays, the bar of the removed `c2` goes.
    assert_that!(series_types(&s)).is_equal_to(vec![json!("line")]);
}

#[test]
fn cells_action_insert_column_keeps_chart_without_projection_in_step() {
    let mut s = with_data(wide_chart());
    act(&mut s, cells("d9", json!({ "op": "insert_column", "after": "c1", "name": "coût", "cells": {} })))
        .unwrap_or_else(|e| panic!("insert: {}", e.one_line()));
    // The new column comes before `c2`: its series too, typed like its neighbour.
    assert_that!(series_types(&s)).is_equal_to(vec![json!("bar"), json!("bar"), json!("line")]);
}

#[test]
fn cells_action_set_cell_twice_is_one_undo_step() {
    let mut s = with_data(data_and_chart());
    let before = sheet_source(&s, "d9");
    for value in [8, 9] {
        act(&mut s, cells("d9", json!({ "op": "set_cell", "row": "r1", "column": "c2", "value": value })))
            .unwrap_or_else(|e| panic!("set: {}", e.one_line()));
    }
    assert_that!(sheet_source(&s, "d9")[1][1].clone()).is_equal_to(json!(9.0));
    assert_that!(undo_label(&s)).is_equal_to(Some(s.localizer.tr("session-undo-data-edit")));
    s.undo().unwrap_or_else(|e| panic!("undo: {}", e.one_line()));
    assert_that!(sheet_source(&s, "d9")).is_equal_to(before);
}

#[test]
fn add_row_action_appends_an_empty_row_one_undo_step() {
    let mut s = with_data(data_and_chart());
    act(&mut s, DataAction::AddRow { data: "d9".into() }).unwrap_or_else(|e| panic!("add row: {}", e.one_line()));
    let source = sheet_source(&s, "d9");
    assert_that!((source.len(), source[3].clone())).is_equal_to((4, vec![Value::Null, Value::Null]));
    s.undo().unwrap_or_else(|e| panic!("undo: {}", e.one_line()));
    assert_that!(sheet_source(&s, "d9").len()).is_equal_to(3);
}

#[test]
fn cells_action_refused_cell_shows_on_its_row() {
    let mut s = with_data(data_and_chart());
    let Err(e) = act(&mut s, cells("d9", json!({ "op": "set_cell", "row": "r9", "column": "c2", "value": 1 }))) else {
        panic!("a missing row was set")
    };
    let shown = e.localize(&s.localizer);
    let d9 = listed(&s).into_iter().find(|r| r.id == "d9").unwrap_or_else(|| panic!("d9 gone"));
    assert_that!(d9.error).is_equal_to(Some(shown));
}

/// A date column: `c1` holds `2026-01-05` on `r1`.
fn dated() -> Op {
    let table = json!({ "generation": "g1", "columns": [{ "id": "c1", "name": "jour", "type": "date" }],
        "rows": [{ "id": "r1", "cells": ["2026-01-05"] }] });
    op(json!({ "op": "add_data", "id": "d4", "name": "jours", "table": table }))
}

#[rstest]
#[case::no_such_day("2026-02-30")]
#[case::not_a_date("lundi")]
fn cells_action_malformed_date_is_refused_not_stored_as_text(#[case] typed: &str) {
    let mut s = with_data(vec![dated()]);
    let set = cells("d4", json!({ "op": "set_cell", "row": "r1", "column": "c1", "value": typed }));
    let Err(e) = act(&mut s, set) else { panic!("{typed} stored in a date column") };
    let (en, fr) = (e.localize(&Localizer::new(Some("en"))), e.localize(&Localizer::new(Some("fr"))));
    assert_that!(fr.as_str()).is_not_equal_to(en.as_str());
    assert_that!(fr.contains('{')).is_false();
    assert_that!(sheet_source(&s, "d4")[1][0].clone()).is_equal_to(json!("2026-01-05"));
}

#[test]
fn paste_action_spreads_a_block_across_cells_and_adds_rows_and_columns() {
    let mut s = with_data(wide_chart());
    let paste = DataAction::Paste {
        data: "d9".into(),
        row: "r2".into(),
        column: "c3".into(),
        cells: serde_json::from_value(json!([[6, 1], [8, 2]])).unwrap_or_else(|e| panic!("{e}")),
        names: vec!["taux".into()],
    };
    act(&mut s, paste).unwrap_or_else(|e| panic!("paste: {}", e.one_line()));
    assert_that!(sheet_source(&s, "d9")).is_equal_to(vec![
        vec![json!("Mois"), json!("ventes"), json!("marge"), json!("taux")],
        vec![json!("Jan"), json!(7.0), json!(2.0), Value::Null],
        vec![json!("Fév"), json!(41.0), json!(6.0), json!(1.0)],
        vec![Value::Null, Value::Null, json!(8.0), json!(2.0)],
    ]);
    // The chart reads every column: the added one gets its series.
    assert_that!(series_types(&s)).is_equal_to(vec![json!("bar"), json!("line"), json!("line")]);
    assert_that!(undo_label(&s)).is_equal_to(Some(s.localizer.tr("session-undo-data-edit")));
}

/// A column edit of a based data, from the data sheet or from the chart's pane.
fn column_edit(s: &mut Session, from_pane: bool, change: Value) -> crate::Result<()> {
    if from_pane {
        let change = serde_json::from_value(change).unwrap_or_else(|e| panic!("{e}"));
        return s.data_edit("s-libre".into(), "ch1", crate::data_edit::DataEdit::Cells { change });
    }
    act(s, cells("d9", change)).map(drop)
}

#[rstest]
#[case::sheet_insert(false, json!({ "op": "insert_column", "after": "c1", "name": "coût", "cells": {} }))]
#[case::sheet_remove(false, json!({ "op": "remove_column", "column": "c3" }))]
#[case::pane_insert(true, json!({ "op": "insert_column", "after": "c1", "name": "coût", "cells": {} }))]
#[case::pane_remove(true, json!({ "op": "remove_column", "column": "c3" }))]
fn column_edit_of_a_based_data_reads_no_base_under_the_lock(#[case] from_pane: bool, #[case] change: Value) {
    let dir = fresh_dir("oku-slides-follow-no-read-");
    let mut s = session_on(dir.path());
    based(&mut s);
    if change["op"] == json!("remove_column") {
        // A second series, as a chart keeps its last one.
        let third = json!({ "op": "insert_column", "id": "c3", "after": "c2", "name": "marge", "cells": {} });
        act(&mut s, cells("d9", third)).unwrap_or_else(|e| panic!("insert: {}", e.one_line()));
    }
    let series_before = series_types(&s).len();
    let handed = s.data_link.handed();
    column_edit(&mut s, from_pane, change.clone()).unwrap_or_else(|e| panic!("edit: {}", e.one_line()));
    // The views follow from the deck's columns: the data worker is never asked.
    assert_that!(s.data_link.handed()).is_equal_to(handed);
    let series_after = series_types(&s).len();
    let grew = change["op"] == json!("insert_column");
    assert_that!(series_after).is_equal_to(if grew { series_before + 1 } else { series_before - 1 });
}

/// `s` with `d9` and its chart, the chart's widget then unknown to the registry, as after a
/// pack change: the chart is a view of `d9` no edit can read.
fn with_unreadable_view() -> Session {
    let mut s = with_data(data_and_chart());
    s.registry = std::sync::Arc::new(oku_slides_core::WidgetRegistry::default());
    s
}

#[test]
fn cell_edit_succeeds_with_a_view_it_cannot_read() {
    let mut s = with_unreadable_view();
    act(&mut s, cells("d9", json!({ "op": "set_cell", "row": "r1", "column": "c2", "value": 8 })))
        .unwrap_or_else(|e| panic!("set: {}", e.one_line()));
    act(&mut s, DataAction::AddRow { data: "d9".into() }).unwrap_or_else(|e| panic!("add row: {}", e.one_line()));
    assert_that!(s.deck().data[&id("d9")].table.rows.len()).is_equal_to(3);
}

#[rstest]
#[case::insert(json!({ "op": "insert_column", "after": "c1", "name": "coût", "cells": {} }))]
#[case::remove(json!({ "op": "remove_column", "column": "c2" }))]
fn column_edit_with_a_view_it_cannot_follow_is_refused(#[case] change: Value) {
    let mut s = with_unreadable_view();
    let Err(e) = act(&mut s, cells("d9", change)) else { panic!("a column changed under a view it cannot follow") };
    for locale in ["en", "fr"] {
        let l = Localizer::new(Some(locale));
        let expected = l.tr_with("session-data-view-unfollowed", &[("view", "ch1".into())]);
        assert_that!(e.localize(&l)).named(locale).is_equal_to(expected);
    }
    assert_that!(s.deck().data[&id("d9")].table.columns.len()).is_equal_to(2);
}
