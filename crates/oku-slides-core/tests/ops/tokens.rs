//! Ops sur les jetons du deck (t-554).

use oku_slides_core::*;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::registry;

// ---- Jetons (t-554) -----------------------------------------------------------------

fn path(keys: &[&str]) -> Vec<String> {
    keys.iter().map(|k| (*k).to_owned()).collect()
}

/// `ops` appliquées à un deck vide.
fn tokens_after(ops: Vec<Op>) -> Value {
    let mut d = Deck::empty();
    apply_all(&mut d, ops, &registry()).unwrap_or_else(|e| panic!("ops refusées : {e}"));
    d.tokens
}

#[test]
fn set_token_null_keeps_explicit_null() {
    let tokens = tokens_after(vec![Op::SetToken { path: path(&["component", "header", "left"]), value: Value::Null }]);
    assert_that!(tokens.pointer("/component/header/left")).is_equal_to(Some(&Value::Null));
}

#[test]
fn unset_token_removes_key_and_empty_parents() {
    let tokens = tokens_after(vec![
        Op::SetToken { path: path(&["color", "accent"]), value: json!("#ff0000") },
        Op::UnsetToken { path: path(&["color", "accent"]) },
    ]);
    assert_that!(tokens).is_equal_to(Value::Null);
}

#[test]
fn unset_token_keeps_sibling_keys() {
    let tokens = tokens_after(vec![
        Op::SetToken { path: path(&["color", "accent"]), value: json!("#ff0000") },
        Op::SetToken { path: path(&["color", "text"]), value: json!("#000000") },
        Op::UnsetToken { path: path(&["color", "accent"]) },
    ]);
    assert_that!(tokens).is_equal_to(json!({ "color": { "text": "#000000" } }));
}

#[test]
fn unset_token_of_an_absent_key_changes_nothing() {
    let tokens = tokens_after(vec![Op::UnsetToken { path: path(&["color", "accent"]) }]);
    assert_that!(tokens).is_equal_to(Value::Null);
}

#[test]
fn set_tokens_replaces_all_tokens() {
    let tokens = tokens_after(vec![
        Op::SetToken { path: path(&["color", "accent"]), value: json!("#ff0000") },
        Op::SetTokens { tokens: json!({ "size": { "body": 30 } }) },
    ]);
    assert_that!(tokens).is_equal_to(json!({ "size": { "body": 30 } }));
}

#[test]
fn set_tokens_null_clears_all_tokens() {
    let tokens = tokens_after(vec![
        Op::SetToken { path: path(&["color", "accent"]), value: json!("#ff0000") },
        Op::SetTokens { tokens: Value::Null },
    ]);
    assert_that!(tokens).is_equal_to(Value::Null);
}
