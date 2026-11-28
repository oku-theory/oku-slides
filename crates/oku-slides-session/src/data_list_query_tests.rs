//! A query data in the data panel: listed with its state, never edited by hand or compacted,
//! read once computed (t-1771); added and its SQL set from the panel, its sheet showing the SQL,
//! the result, the error and the inputs (t-1779).

use oku_slides_a2ui::{DataChoice, DataState, QueryView};

use super::*;
use crate::QueryStatus;
use crate::derived::derived_tests::{TOP, apply as apply_ops, set_b, with_query as with_ready_query};

/// A session on `fixtures/roadmap.json` holding the query data `q1`, and its folder.
fn with_query() -> (Session, tempfile::TempDir) {
    let dir = fresh_dir("oku-slides-session-query-");
    let text = std::fs::read_to_string(FIXTURE).unwrap_or_else(|e| panic!("{e}"));
    let mut deck: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}"));
    deck["data"] = json!({ "q1": { "name": "ventes", "kind": "query", "sql": "SELECT 1" } });
    let path = dir.path().join("query.json");
    std::fs::write(&path, deck.to_string()).unwrap_or_else(|e| panic!("{e}"));
    let mut s = session_on(dir.path());
    s.open(&path).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    (s, dir)
}

/// `op-data-derived` for `q1`, as the French session says it.
fn derived(s: &Session) -> String {
    s.localizer.tr_with("op-data-derived", &[("data", FluentValue::from("q1"))])
}

#[test]
fn data_list_shows_a_query_data_without_columns_or_rows() {
    let (s, _dir) = with_query();
    let rows = listed(&s);
    let q1 = rows.iter().find(|r| r.id == "q1").unwrap_or_else(|| panic!("q1 not listed"));
    assert_that!((q1.kind.as_str(), q1.columns, q1.rows, q1.state)).is_equal_to((
        "query",
        0,
        0,
        Some(DataState::Pending),
    ));
}

#[rstest]
#[case::add_row(DataAction::AddRow { data: "q1".into() })]
#[case::paste(DataAction::Paste {
    data: "q1".into(), row: "r1".into(), column: "c1".into(), cells: vec![vec![]], names: vec![],
})]
fn manual_action_on_a_query_data_is_refused_as_derived(#[case] action: DataAction) {
    let (mut s, _dir) = with_query();
    let before = s.deck().clone();
    let err = act(&mut s, action).map(|_| ()).unwrap_err();
    assert_that!(err.localize(&s.localizer)).is_equal_to(derived(&s));
    assert_that!(*s.deck()).is_equal_to(&before);
}

/// The `DataSheet` of `data` from row 0: its page, its types, its query and its refusal.
fn query_page(s: &Session, data: &str) -> (Vec<Vec<Value>>, Vec<String>, Option<QueryView>, Option<String>) {
    let read = s.data_sheet(data, 0).unwrap_or_else(|| panic!("no sheet for {data}"));
    let surface = read.surface().unwrap_or_else(|| panic!("no sheet surface for {data}"));
    let found = components(&surface).into_iter().find_map(|c| match c {
        ComponentKind::DataSheet { source, types, query, error, .. } => Some((source, types, query, error)),
        _ => None,
    });
    found.unwrap_or_else(|| panic!("DataSheet expected"))
}

/// `ventes` (`m`) as a query's input.
fn ventes_input() -> Vec<DataChoice> {
    vec![DataChoice { id: "m".into(), name: "ventes".into(), label: None }]
}

#[test]
fn data_sheet_of_a_pending_query_shows_its_sql_without_rows() {
    let (s, _dir) = with_query();
    let (source, types, query, error) = query_page(&s, "q1");
    let pending = QueryView { sql: "SELECT 1".into(), state: DataState::Pending, error: None, inputs: vec![] };
    assert_that!((source, types, query, error)).is_equal_to((vec![vec![]], vec![], Some(pending), None));
}

#[test]
fn data_sheet_of_a_ready_query_shows_its_result_and_inputs() {
    let (mut s, _dir) = with_ready_query(TOP);
    s.settle_queries();
    let (source, types, query, _) = query_page(&s, "q");
    assert_that!(source).is_equal_to(vec![vec![json!("a"), json!("b")], vec![json!("x"), json!(1.0)]]);
    assert_that!(types).is_equal_to(vec!["text".to_owned(), "number".to_owned()]);
    let ready = QueryView { sql: TOP.into(), state: DataState::Ready, error: None, inputs: ventes_input() };
    assert_that!(query).is_equal_to(Some(ready));
}

#[test]
fn data_sheet_of_a_failed_query_shows_its_error_fr() {
    let (mut s, _dir) = with_ready_query("SELECT zzz FROM ventes");
    s.settle_queries();
    let (source, _, query, _) = query_page(&s, "q");
    let query = query.unwrap_or_else(|| panic!("no query view"));
    let Some(QueryStatus::Failed(message)) = s.query_statuses().remove(&id("q")) else { panic!("q did not fail") };
    assert_that!(message.id()).is_equal_to("data-query-run");
    let reason = message.localize(&Localizer::new(Some("fr")));
    assert_that!(reason.as_str()).is_not_equal_to(message.localize(&Localizer::new(Some("en"))).as_str());
    assert_that!((source, query.state, query.error)).is_equal_to((vec![vec![]], DataState::Failed, Some(reason)));
}

#[test]
fn data_list_shows_a_query_missing_from_the_statuses_as_failed() {
    let (s, _dir) = with_ready_query(TOP);
    let rows = data_list(s.deck(), &std::collections::BTreeMap::new(), &s.localizer);
    let state = |data: &str| rows.iter().find(|r| r.id == data).map(|r| r.state);
    assert_that!((state("q"), state("m"))).is_equal_to((Some(Some(DataState::Failed)), Some(None)));
}

#[test]
fn data_list_shows_a_ready_query_with_its_result_size() {
    let (mut s, _dir) = with_ready_query(TOP);
    s.settle_queries();
    let q = listed(&s).into_iter().find(|r| r.id == "q").unwrap_or_else(|| panic!("q not listed"));
    assert_that!((q.columns, q.rows, q.state)).is_equal_to((2, 1, Some(DataState::Ready)));
}

#[test]
fn add_query_action_creates_query_data() {
    let (mut s, _dir) = with_query();
    let before = s.deck().clone();
    act(&mut s, DataAction::AddQuery { name: None }).unwrap_or_else(|e| panic!("add: {}", e.one_line()));
    assert_that!(undo_label(&s)).is_equal_to(Some(s.localizer.tr("session-undo-data-add")));
    act(&mut s, DataAction::AddQuery { name: Some("top".into()) }).unwrap_or_else(|e| panic!("add: {}", e.one_line()));
    s.settle_queries();
    let mut added: Vec<(String, String, Option<DataState>)> =
        listed(&s).into_iter().filter(|r| r.id != "q1").map(|r| (r.name, r.kind, r.state)).collect();
    added.sort_by(|x, y| x.0.cmp(&y.0));
    let query = |name: &str| (name.to_owned(), "query".to_owned(), Some(DataState::Ready));
    assert_that!(added).is_equal_to(vec![query("query"), query("top")]);
    s.undo().unwrap_or_else(|e| panic!("undo: {}", e.one_line()));
    s.undo().unwrap_or_else(|e| panic!("undo: {}", e.one_line()));
    assert_that!(*s.deck()).is_equal_to(&before);
}

#[test]
fn set_query_action_sets_sql_one_undo_step_and_recomputes() {
    let (mut s, _dir) = with_ready_query(TOP);
    s.settle_queries();
    let before = s.deck().clone();
    let sql = "SELECT a, b FROM ventes";
    act(&mut s, DataAction::SetQuery { data: "q".into(), sql: sql.into() })
        .unwrap_or_else(|e| panic!("set: {}", e.one_line()));
    assert_that!(undo_label(&s)).is_equal_to(Some(s.localizer.tr("session-undo-data-query")));
    s.settle_queries();
    let q = listed(&s).into_iter().find(|r| r.id == "q").unwrap_or_else(|| panic!("q not listed"));
    assert_that!((s.deck().data[&id("q")].sql(), q.rows)).is_equal_to((Some(sql), 3));
    s.undo().unwrap_or_else(|e| panic!("undo: {}", e.one_line()));
    assert_that!(*s.deck()).is_equal_to(&before);
}

#[test]
fn set_query_action_refusal_shows_on_sheet() {
    let (mut s, _dir) = with_ready_query(TOP);
    s.settle_queries();
    let before = s.deck().clone();
    let set = DataAction::SetQuery { data: "q".into(), sql: "SELECT a FROM absent".into() };
    let err = act(&mut s, set).map(|_| ()).unwrap_err();
    let fr = s.localizer.tr_with("op-data-unknown-input", &[("name", FluentValue::from("absent"))]);
    assert_that!(err.localize(&s.localizer).as_str()).contains(fr.as_str());
    let (_, _, query, error) = query_page(&s, "q");
    assert_that!((query.map(|q| q.sql), error)).is_equal_to((Some(TOP.to_owned()), Some(err.localize(&s.localizer))));
    assert_that!(*s.deck()).is_equal_to(&before);
    // The next edit clears it.
    apply_ops(&mut s, vec![set_b(1, 5.0)]);
    assert_that!(query_page(&s, "q").3).is_none();
}

#[test]
fn query_sheet_sends_no_cell_edit_for_its_result() {
    let (mut s, _dir) = with_ready_query(TOP);
    s.settle_queries();
    let cells = DataAction::Cells {
        data: "q".into(),
        change: serde_json::from_value(json!({ "op": "set_cell", "row": "r0", "column": "a", "value": "y" }))
            .unwrap_or_else(|e| panic!("{e}")),
    };
    let err = act(&mut s, cells).map(|_| ()).unwrap_err();
    let derived = s.localizer.tr_with("op-data-derived", &[("data", FluentValue::from("q"))]);
    assert_that!(err.localize(&s.localizer)).ends_with(derived.as_str());
}

#[test]
fn query_data_is_never_compacted() {
    let (mut s, _dir) = with_query();
    assert_that!(s.compaction_job(&id("q1"), Instant::now(), true).is_none()).is_true();
}

#[test]
fn rows_of_a_query_data_without_result_are_refused() {
    let (s, _dir) = with_query();
    let read = s.data_rows("q1", 10, 0..10, 100).unwrap_or_else(|| panic!("q1 missing"));
    let err = read.map(|_| ()).unwrap_err();
    let expected = s.localizer.tr_with("session-data-query-read-no-result", &[("data", FluentValue::from("ventes"))]);
    assert_that!(err.localize(&s.localizer)).is_equal_to(expected);
}
