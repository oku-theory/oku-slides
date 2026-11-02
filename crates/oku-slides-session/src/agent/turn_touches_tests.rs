use oku_slides_core::{Deck, Op};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{Touch, TurnTouches, touches_of};
use crate::agent::KEPT_TURNS;

/// One change of `element` on slide `s`, a text.
fn touch(element: &str) -> Touch {
    Touch {
        slide: "s".into(),
        element: element.into(),
        kind: "base/text".into(),
        text: None,
        from: None,
        changes: 1,
        removed: false,
        data: false,
    }
}

/// A recording turn `turn` that took `touches`, one batch each.
fn recorded(turn: u32, touches: Vec<Touch>) -> TurnTouches {
    let mut t = TurnTouches::default();
    t.begin(turn);
    for one in touches {
        t.record_all(vec![one]);
    }
    t
}

fn elements(t: &TurnTouches, turn: u32) -> Vec<String> {
    t.touches(turn).iter().map(|x| x.element.clone()).collect()
}

#[test]
fn record_same_element_merges_changes() {
    let t = recorded(1, vec![touch("a"), touch("a")]);
    assert_that!(t.touches(1).iter().map(|x| x.changes).collect::<Vec<_>>()).is_equal_to(vec![2]);
}

#[test]
fn record_keeps_first_touch_order() {
    let t = recorded(1, vec![touch("b"), touch("a"), touch("b")]);
    assert_that!(elements(&t, 1)).is_equal_to(vec!["b".to_owned(), "a".to_owned()]);
}

#[test]
fn record_removal_after_edit_marks_removed() {
    let t = recorded(1, vec![touch("a"), Touch { removed: true, ..touch("a") }]);
    assert_that!(t.touches(1).first().map(|x| x.removed)).is_equal_to(Some(true));
}

#[test]
fn record_data_after_edit_marks_data() {
    let t = recorded(1, vec![touch("a"), Touch { data: true, ..touch("a") }, touch("a")]);
    assert_that!(t.touches(1).first().map(|x| x.data)).is_equal_to(Some(true));
}

/// `element` moved from slide `from` to slide `to`.
fn moved(element: &str, from: &str, to: &str) -> Touch {
    Touch { slide: to.into(), from: Some(from.into()), ..touch(element) }
}

#[test]
fn record_move_after_edit_keeps_one_trace() {
    let t = recorded(1, vec![touch("a"), touch("b"), moved("a", "s", "t")]);
    let shown = t.touches(1).iter().map(|x| (x.slide.clone(), x.element.clone(), x.changes)).collect::<Vec<_>>();
    assert_that!(shown).is_equal_to(vec![("t".to_owned(), "a".to_owned(), 2), ("s".to_owned(), "b".to_owned(), 1)]);
}

#[test]
fn record_move_onto_a_touch_there_merges_at_the_first_rank() {
    let there = Touch { slide: "t".into(), ..touch("a") };
    let t = recorded(1, vec![touch("a"), touch("b"), there, moved("a", "s", "t")]);
    let shown = t.touches(1).iter().map(|x| (x.slide.clone(), x.element.clone(), x.changes)).collect::<Vec<_>>();
    assert_that!(shown).is_equal_to(vec![("t".to_owned(), "a".to_owned(), 3), ("s".to_owned(), "b".to_owned(), 1)]);
}

#[test]
fn begin_turn_starts_an_empty_list_and_keeps_the_previous() {
    let mut t = recorded(1, vec![touch("a")]);
    t.end();
    t.begin(2);
    assert_that!((elements(&t, 1), elements(&t, 2))).is_equal_to((vec!["a".to_owned()], Vec::new()));
}

#[test]
fn turns_beyond_kept_turns_are_dropped() {
    let mut t = TurnTouches::default();
    let last = u32::try_from(KEPT_TURNS).unwrap_or(u32::MAX) + 1;
    for turn in 1..=last {
        t.begin(turn);
        t.record_all(vec![touch("a")]);
        t.end();
    }
    assert_that!((t.touches(1).len(), t.touches(2).len(), t.touches(last).len())).is_equal_to((0, 1, 1));
}

fn deck() -> Deck {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    Deck::from_json(&text).unwrap_or_else(|e| panic!("{path} : {e}"))
}

fn op(v: Value) -> Op {
    let shown = v.to_string();
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op {shown} : {e}"))
}

const PIN: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// One case per `Op` variant: `(slide, element, data, removed)` of the touch, or none.
#[rstest]
#[case::set_param(json!({ "op": "set_param", "target": { "slide": "s-priorites", "element": "c1" }, "param": "title", "value": "Cap" }), Some(("s-priorites", "c1", false, false)))]
#[case::set_param_on_a_slide(json!({ "op": "set_param", "target": { "slide": "s-priorites" }, "param": "title", "value": "Cap" }), None)]
#[case::set_deck_param(json!({ "op": "set_deck_param", "key": "title", "value": "Cap" }), None)]
#[case::set_kind(json!({ "op": "set_kind", "target": { "slide": "s-priorites", "element": "c1" }, "kind": "base/text" }), Some(("s-priorites", "c1", false, false)))]
#[case::set_kind_on_a_slide(json!({ "op": "set_kind", "target": { "slide": "s-priorites" }, "kind": "essentiel/blank" }), None)]
#[case::set_style(json!({ "op": "set_style", "slide": "s-libre", "element": "e-fond", "style": {} }), Some(("s-libre", "e-fond", false, false)))]
#[case::replace_style(json!({ "op": "replace_style", "slide": "s-libre", "element": "e-fond", "style": {} }), Some(("s-libre", "e-fond", false, false)))]
#[case::set_frame(json!({ "op": "set_frame", "slide": "s-libre", "element": "e-fond", "frame": null }), Some(("s-libre", "e-fond", false, false)))]
#[case::set_cell(json!({ "op": "set_cell", "slide": "s-libre", "element": "e-fond", "cell": null }), Some(("s-libre", "e-fond", false, false)))]
#[case::set_grid(json!({ "op": "set_grid", "slide": "s-libre", "columns": 4 }), None)]
#[case::replace_element_with_a_card(json!({ "op": "replace_element", "slide": "s-priorites", "element": { "id": "c1", "kind": "base/card" } }), Some(("s-priorites", "c1", false, false)))]
#[case::replace_element_with_a_chart(json!({ "op": "replace_element", "slide": "s-priorites", "element": { "id": "c1", "kind": "base/chart" } }), Some(("s-priorites", "c1", true, false)))]
#[case::set_chart_option(json!({ "op": "set_chart_option", "target": { "slide": "s-libre", "element": "e-texte" }, "param": "option", "patch": {} }), Some(("s-libre", "e-texte", true, false)))]
#[case::set_chart_option_on_a_slide(json!({ "op": "set_chart_option", "target": { "slide": "s-libre" }, "param": "option", "patch": {} }), None)]
#[case::set_table_option(json!({ "op": "set_table_option", "target": { "slide": "s-libre", "element": "e-texte" }, "param": "data", "patch": {} }), Some(("s-libre", "e-texte", true, false)))]
#[case::set_data(json!({ "op": "set_data", "target": { "slide": "s-libre", "element": "e-texte" }, "param": "option", "change": { "op": "set_cell", "row": 1, "col": 1, "value": 5 } }), Some(("s-libre", "e-texte", true, false)))]
#[case::edit_list(json!({ "op": "edit_list", "target": { "slide": "s-libre", "element": "e-texte" }, "param": "items", "change": { "op": "remove", "item": "d0" } }), Some(("s-libre", "e-texte", false, false)))]
#[case::edit_list_on_a_slide(json!({ "op": "edit_list", "target": { "slide": "s-libre" }, "param": "items", "change": { "op": "remove", "item": "d0" } }), None)]
#[case::add_element(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-libre" }, "param": "elements" }, "element": { "id": "n1", "kind": "base/shape" } }), Some(("s-libre", "n1", false, false)))]
#[case::remove_element(json!({ "op": "remove_element", "slide": "s-libre", "element": "e-fond" }), Some(("s-libre", "e-fond", false, true)))]
#[case::move_element(json!({ "op": "move_element", "slide": "s-libre", "element": "e-fond", "to": { "owner": { "slide": "s-constat" }, "param": "blocks" } }), Some(("s-constat", "e-fond", false, false)))]
#[case::add_slide(json!({ "op": "add_slide", "slide": { "id": "s-new", "kind": "essentiel/blank" } }), None)]
#[case::remove_slide(json!({ "op": "remove_slide", "slide": "s-libre" }), None)]
#[case::move_slide(json!({ "op": "move_slide", "slide": "s-libre", "index": 0 }), None)]
#[case::set_notes(json!({ "op": "set_notes", "slide": "s-libre", "notes": "" }), None)]
#[case::set_theme(json!({ "op": "set_theme", "theme": null }), None)]
#[case::add_pack(json!({ "op": "add_pack", "kind": "widgets", "namespace": "acme", "pin": { "id": "6f1c2a8e-4c1b-4f7e-9b1a-2d3c4e5f6a7b", "version": PIN } }), None)]
#[case::remove_pack(json!({ "op": "remove_pack", "kind": "widgets", "namespace": "acme" }), None)]
#[case::set_token(json!({ "op": "set_token", "path": ["color", "accent"], "value": "#000000" }), None)]
#[case::unset_token(json!({ "op": "unset_token", "path": ["color", "accent"] }), None)]
#[case::set_tokens(json!({ "op": "set_tokens", "tokens": null }), None)]
#[case::set_var(json!({ "op": "set_var", "key": "client", "value": "Acme" }), None)]
fn touches_of_each_op(#[case] v: Value, #[case] expected: Option<(&str, &str, bool, bool)>) {
    let touches = touches_of(&deck(), &crate::test_support::registry(), &[op(v)]);
    let got: Vec<(String, String, bool, bool)> =
        touches.into_iter().map(|t| (t.slide, t.element, t.data, t.removed)).collect();
    let want: Vec<(String, String, bool, bool)> =
        expected.into_iter().map(|(s, e, d, r)| (s.to_owned(), e.to_owned(), d, r)).collect();
    assert_that!(got).is_equal_to(want);
}

#[test]
fn touches_of_takes_the_kind_from_the_deck_before_the_batch() {
    let removed = op(json!({ "op": "remove_element", "slide": "s-libre", "element": "e-fond" }));
    let touches = touches_of(&deck(), &crate::test_support::registry(), &[removed]);
    assert_that!(touches.first().map(|t| t.kind.clone())).is_equal_to(Some("base/shape".to_owned()));
}

#[test]
fn touches_of_takes_the_kind_of_an_element_added_earlier_in_the_batch() {
    let add = op(json!({ "op": "add_element", "slot": { "owner": { "slide": "s-libre" }, "param": "elements" },
        "element": { "id": "n1", "kind": "base/image" } }));
    let style = op(json!({ "op": "set_style", "slide": "s-libre", "element": "n1", "style": {} }));
    let touches = touches_of(&deck(), &crate::test_support::registry(), &[add, style]);
    assert_that!(touches.last().map(|t| t.kind.clone())).is_equal_to(Some("base/image".to_owned()));
}
