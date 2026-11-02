use oku_slides_core::Op;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{Trace, TraceName};
use crate::Session;
use crate::test_support::session;

fn op(v: Value) -> Op {
    let shown = v.to_string();
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op {shown} : {e}"))
}

/// The body of `e-texte` (`s-libre`) set to `text`.
fn set_body(text: &str) -> Op {
    op(json!({ "op": "set_param", "target": { "slide": "s-libre", "element": "e-texte" }, "param": "body",
        "value": [{ "runs": [{ "t": text }] }] }))
}

/// The traces of turn 1, in which the agent applied `ops`, one batch each.
fn traced(s: &mut Session, ops: Vec<Op>) -> Vec<Trace> {
    s.begin_turn(1, "consigne");
    for o in ops {
        s.apply_by_agent("agent", vec![o], None).unwrap_or_else(|e| panic!("apply : {}", e.one_line()));
    }
    s.traces(1)
}

fn names(traces: &[Trace]) -> Vec<TraceName> {
    traces.iter().map(|t| t.name.clone()).collect()
}

#[test]
fn traces_title_from_first_text() {
    let mut s = session();
    let traces = traced(&mut s, vec![set_body("  Ventes\n du  trimestre ")]);
    assert_that!(names(&traces)).is_equal_to(vec![TraceName::Text("Ventes du trimestre".into())]);
}

#[test]
fn traces_long_title_is_cut() {
    let mut s = session();
    let traces = traced(&mut s, vec![set_body(&"a".repeat(60))]);
    assert_that!(names(&traces)).is_equal_to(vec![TraceName::Text(format!("{}…", "a".repeat(39)))]);
}

#[test]
fn traces_without_text_name_the_kind() {
    let mut s = session();
    let style = op(json!({ "op": "set_style", "slide": "s-libre", "element": "e-fond", "style": {} }));
    let traces = traced(&mut s, vec![style]);
    let kind = traces.first().map(|t| match &t.name {
        TraceName::Kind { kind, .. } => kind.clone(),
        TraceName::Text(text) => format!("text {text}"),
    });
    assert_that!(kind).is_equal_to(Some(oku_slides_core::kinds::base::SHAPE.to_owned()));
}

#[test]
fn traces_removed_element_is_marked() {
    let mut s = session();
    let style = op(json!({ "op": "set_style", "slide": "s-libre", "element": "e-fond", "style": {} }));
    let remove = op(json!({ "op": "remove_element", "slide": "s-libre", "element": "e-fond" }));
    let traces = traced(&mut s, vec![style, remove]);
    assert_that!(traces.iter().map(|t| (t.removed, t.changes)).collect::<Vec<_>>()).is_equal_to(vec![(true, 2)]);
}

#[test]
fn traces_removed_named_element_keeps_its_text() {
    let mut s = session();
    let remove = op(json!({ "op": "remove_element", "slide": "s-libre", "element": "e-texte" }));
    let traces = traced(&mut s, vec![set_body("Ventes"), remove]);
    assert_that!(traces.iter().map(|t| (t.name.clone(), t.removed)).collect::<Vec<_>>())
        .is_equal_to(vec![(TraceName::Text("Ventes".into()), true)]);
}
