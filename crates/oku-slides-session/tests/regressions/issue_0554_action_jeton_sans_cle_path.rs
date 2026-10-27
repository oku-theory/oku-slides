//! t-554 : les actions de la surface `host-theme` partent bien du navigateur.
//!
//! Le contexte d'une action portait le chemin du jeton sous la clé `path` :
//! @a2ui prend tout objet ayant une clé `path` pour une liaison de données,
//! et sa résolution levait (« startsWith is not a function ») avant l'envoi.
//! Aucun changement de jeton n'arrivait à la session depuis l'interface.

use oku_slides_session::{PAPIER_THEME, Request};
use serde_json::Value;
use speculoos::prelude::*;

use crate::common::session;

/// Les contextes de toutes les actions de `v`, à toute profondeur.
fn contexts<'a>(v: &'a Value, out: &mut Vec<&'a serde_json::Map<String, Value>>) {
    match v {
        Value::Object(map) => {
            if let Some(Value::Object(context)) = map.get("event").and_then(|e| e.get("context")) {
                out.push(context);
            }
            map.values().for_each(|child| contexts(child, out));
        }
        Value::Array(items) => items.iter().for_each(|child| contexts(child, out)),
        _ => {}
    }
}

#[test]
fn host_theme_action_contexts_have_no_path_key() {
    let mut s = session();
    s.open_theme_here(PAPIER_THEME).unwrap_or_else(|e| panic!("{e}"));
    let reply = s.handle(Request::HostTheme { filter: String::new() }).unwrap_or_else(|e| panic!("{e}"));
    let surface: Value = serde_json::from_str(&reply).unwrap_or_else(|e| panic!("{e}"));
    let mut found = Vec::new();
    contexts(&surface, &mut found);
    let tokens: Vec<_> = found.iter().filter(|c| c.contains_key("token")).collect();
    assert_that!(tokens.is_empty()).is_false();
    assert_that!(found.iter().any(|c| c.contains_key("path"))).is_false();
}
