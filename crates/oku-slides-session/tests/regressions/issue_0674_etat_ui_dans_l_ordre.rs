//! t-674 : l'état d'interface gardé ne se perd plus à la fermeture.
//!
//! L'interface gardait sa dernière sauvegarde en attente tant que la précédente n'avait pas répondu
//! (t-668) : ⌘Q pendant cette attente la perdait. Chaque sauvegarde part désormais tout de suite,
//! avec un ordre `{page, seq}` : l'hôte ignore celle qui arrive après une plus récente de la même page.
//! La garantie vaut pour les sauvegardes déjà parties : l'hôte les traite (chacune tient le verrou de
//! la session) avant de s'arrêter ; un changement pas encore envoyé par la page peut toujours manquer.

use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::{req, session, snapshot_json};

fn ui(slide: &str, order: Value) -> Value {
    let mut request = json!({"cmd": "ui", "slide": slide, "grid": true});
    request["order"] = order;
    request
}

#[test]
fn ui_requests_out_of_order_keep_the_latest() {
    let mut s = session();
    s.handle(req(ui("s-2", json!({"page": "p", "seq": 2})))).unwrap_or_else(|e| panic!("{e}"));
    s.handle(req(ui("s-1", json!({"page": "p", "seq": 1})))).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(snapshot_json(&mut s)["ui"]["slide"]).is_equal_to(json!("s-2"));
}

#[test]
fn ui_request_with_same_seq_is_ignored() {
    let mut s = session();
    s.handle(req(ui("s-2", json!({"page": "p", "seq": 2})))).unwrap_or_else(|e| panic!("{e}"));
    s.handle(req(ui("s-1", json!({"page": "p", "seq": 2})))).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(snapshot_json(&mut s)["ui"]["slide"]).is_equal_to(json!("s-2"));
}

/// Une page rechargée repart de `seq` 1 : elle est toujours écoutée.
#[test]
fn ui_request_from_another_page_is_applied() {
    let mut s = session();
    s.handle(req(ui("s-2", json!({"page": "p", "seq": 2})))).unwrap_or_else(|e| panic!("{e}"));
    s.handle(req(ui("s-1", json!({"page": "q", "seq": 1})))).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(snapshot_json(&mut s)["ui"]["slide"]).is_equal_to(json!("s-1"));
}

/// Sans ordre (agent, tests, ancienne interface) : toujours appliquée.
#[test]
fn ui_request_without_order_is_applied() {
    let mut s = session();
    s.handle(req(ui("s-2", json!({"page": "p", "seq": 2})))).unwrap_or_else(|e| panic!("{e}"));
    s.handle(req(json!({"cmd": "ui", "slide": "s-1"}))).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(snapshot_json(&mut s)["ui"]["slide"]).is_equal_to(json!("s-1"));
}

/// L'ordre n'entre pas dans l'état d'interface montré (ni gardé pour la reprise).
#[test]
fn ui_order_is_not_in_the_snapshot() {
    let mut s = session();
    s.handle(req(ui("s-2", json!({"page": "p", "seq": 2})))).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(snapshot_json(&mut s)["ui"].get("order")).is_none();
}
