//! Traces of a turn: only the agent's writes (`handle_by_agent`, `apply_by_agent`) are recorded.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

#[allow(dead_code, reason = "aides partagées : chaque binaire de test n'en utilise qu'une partie")]
mod common;

use common::{fixture_path, ok, req, session, with_element};
use oku_slides_core::kinds;
use oku_slides_session::Session;
use serde_json::{Value, json};
use speculoos::prelude::*;

/// An `apply` request setting the style of `e-fond` (`s-libre`).
fn style_fond() -> Value {
    json!({ "cmd": "apply", "label": "style", "ops": [
        { "op": "set_style", "slide": "s-libre", "element": "e-fond", "style": {} } ] })
}

/// The `(slide, element)` of each trace of turn 1.
fn traced(s: &Session) -> Vec<(String, String)> {
    s.traces(1).into_iter().map(|t| (t.slide, t.element)).collect()
}

fn opened(s: &mut Session) {
    ok(s.open(&fixture_path()));
    s.begin_turn(1, "consigne");
}

#[test]
fn handle_by_agent_apply_records_one_trace() {
    let mut s = session();
    opened(&mut s);
    ok(s.handle_by_agent(req(style_fond())));
    assert_that!(traced(&s)).is_equal_to(vec![("s-libre".to_owned(), "e-fond".to_owned())]);
}

#[test]
fn handle_apply_by_user_during_turn_records_nothing() {
    let mut s = session();
    opened(&mut s);
    ok(s.handle(req(style_fond())));
    assert_that!(traced(&s)).is_empty();
}

#[test]
fn handle_by_agent_refused_op_records_nothing() {
    let mut s = session();
    opened(&mut s);
    let refused = json!({ "cmd": "apply", "label": "style", "ops": [
        { "op": "set_style", "slide": "s-libre", "element": "e-fond", "style": {} },
        { "op": "remove_element", "slide": "s-libre", "element": "absent" } ] });
    assert_that!(s.handle_by_agent(req(refused))).is_err();
    // The user's write after the refusal is not recorded either: the agent's call is over.
    ok(s.handle(req(style_fond())));
    assert_that!(traced(&s)).is_empty();
}

#[test]
fn data_edit_by_agent_marks_data() {
    let mut s = session();
    let chart = json!({ "id": "g", "kind": kinds::base::CHART, "params": { "option": {
        "dataset": { "source": [["M", "A"], ["x", 1]] }, "series": [{ "type": "bar" }] } } });
    let slide = with_element(&mut s, chart);
    s.begin_turn(1, "consigne");
    let edit =
        json!({ "cmd": "data_edit", "slide": slide, "element": "g", "edit": { "edit": "kind", "kind": "line" } });
    ok(s.handle_by_agent(req(edit)));
    assert_that!(s.traces(1).iter().map(|t| (t.element.as_str(), t.data)).collect::<Vec<_>>())
        .is_equal_to(vec![("g", true)]);
}

#[test]
fn end_turn_returns_the_turn_traces() {
    let mut s = session();
    opened(&mut s);
    ok(s.handle_by_agent(req(style_fond())));
    let ended: Vec<String> = s.end_turn().into_iter().map(|t| t.element).collect();
    assert_that!(ended).is_equal_to(vec!["e-fond".to_owned()]);
}
