use serde_json::json;
use speculoos::prelude::*;

use super::*;

fn issues_ids(issues: &[crate::Issue]) -> Vec<&'static str> {
    issues.iter().map(|i| i.message.id()).collect()
}

#[test]
fn materialize_unknown_element_kind_drops_element_only() {
    let root = json!({ "slides": [{ "id": "s1", "kind": "content", "elements": [
        { "id": "e-ok", "kind": "shape" },
        { "id": "e-ko", "kind": "hologramme" }
    ]}]});
    let (deck, issues) = materialize(root.as_object().unwrap().clone());
    assert_that!(deck.slides[0].elements).has_length(1);
    assert_that!(issues_ids(&issues)).is_equal_to(vec!["issue-unreadable-element"]);
}

#[test]
fn materialize_bad_slide_drops_slide_only() {
    let root = json!({ "slides": [
        { "id": "s1", "kind": "content" },
        { "id": "s2", "kind": 42 }
    ]});
    let (deck, issues) = materialize(root.as_object().unwrap().clone());
    assert_that!(deck.slides).has_length(1);
    assert_that!(issues_ids(&issues)).is_equal_to(vec!["issue-unreadable-slide"]);
}

#[test]
fn materialize_bad_top_level_field_resets_it() {
    let root = json!({ "theme": 7, "slides": [] });
    let (deck, issues) = materialize(root.as_object().unwrap().clone());
    assert_that!(deck.theme).is_none();
    assert_that!(issues_ids(&issues)).is_equal_to(vec!["issue-unreadable-field"]);
}

#[test]
fn materialize_incoherent_chart_drops_element() {
    let root = json!({ "slides": [{ "id": "s1", "kind": "content", "elements": [
        { "id": "c", "kind": "chart", "option": { "series": "pas une liste" } }
    ]}]});
    let (deck, issues) = materialize(root.as_object().unwrap().clone());
    // La slide reste, seul l'élément incohérent est écarté.
    assert_that!(deck.slides).has_length(1);
    assert_that!(deck.slides[0].elements).is_empty();
    assert_that!(issues_ids(&issues)).is_equal_to(vec!["issue-unreadable-element"]);
}
