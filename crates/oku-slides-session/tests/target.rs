//! Requêtes du panneau de la cible : `target_settings`, `target_param`, `target_list`.

use oku_slides_session::Request;
use serde_json::json;
use speculoos::prelude::*;

mod common;
use common::{Isolated, fixture_path, req, session};

/// Le fixture ouvert, avec le widget de pack `frise` des tests et un élément `fr` sur `s-constat`.
fn with_frise() -> Isolated {
    let mut s = session();
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("fixture : {}", e.one_line()));
    s.set_widget("frise", include_str!("../../../fixtures/widgets/frise-test.typ").into())
        .unwrap_or_else(|e| panic!("frise : {}", e.one_line()));
    s.handle(req(json!({ "cmd": "apply", "label": "frise", "ops": [{
        "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "defaults": true, "element": { "id": "fr", "kind": "frise" }
    }] })))
    .unwrap_or_else(|e| panic!("élément frise : {}", e.one_line()));
    s
}

fn target_param_title() -> Request {
    req(json!({ "cmd": "target_param", "slide": "s-ch1", "key": "title", "value": "X" }))
}

fn target_list_insert() -> Request {
    req(json!({
        "cmd": "target_list", "slide": "s-constat", "element": "fr", "key": "etapes",
        "change": { "op": "insert", "id": "n1", "values": { "date": "2030" } }
    }))
}

#[test]
fn target_settings_request_without_element_returns_the_slide_surface() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let reply = s.handle(req(json!({ "cmd": "target_settings", "slide": "s-ch1" }))).unwrap();
    let v: serde_json::Value = serde_json::from_str(&reply).unwrap();
    assert_that!(v["id"].as_str()).is_equal_to(Some("host-target"));
}

#[test]
fn target_param_request_changes_the_title() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    s.handle(target_param_title()).unwrap();
    assert_that!(s.deck().slide("s-ch1").unwrap().params.text("title")).is_equal_to(Some("X"));
}

#[test]
fn target_list_request_inserts_the_item() {
    let mut s = with_frise();
    s.handle(target_list_insert()).unwrap();
    let el = &s.deck().slide("s-constat").unwrap().locate("fr").unwrap().element;
    let items = serde_json::to_value(el.params.get("etapes").unwrap()).unwrap();
    let last = items.as_array().unwrap().last().cloned().unwrap();
    assert_that!(last).is_equal_to(json!({ "id": "n1", "date": "2030", "texte": "Nouvelle étape" }));
}

#[test]
fn target_requests_in_history_are_refused_as_in_history() {
    let mut s = with_frise();
    s.handle(req(json!({ "cmd": "history_open" }))).unwrap();
    for r in [target_param_title(), target_list_insert()] {
        let err = s.handle(r).unwrap_err();
        assert_that!(err.kind()).is_equal_to(oku_slides_session::SessionErrorKind::InHistory);
    }
}

#[test]
fn target_requests_in_help_are_refused_as_read_only() {
    let mut s = session();
    s.open_help("en", None).unwrap();
    for r in [target_param_title(), target_list_insert()] {
        let err = s.handle(r).unwrap_err();
        assert_that!(err.kind()).is_equal_to(oku_slides_session::SessionErrorKind::ReadOnly);
    }
}
