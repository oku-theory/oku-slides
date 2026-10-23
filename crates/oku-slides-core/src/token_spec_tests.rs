use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::*;
use crate::error::TokenEntryErrorKind;

#[rstest]
#[case(json!({"path": ["card", "bg"], "default": "bg", "control": "color"}), true)]
#[case(json!({"path": ["chapter", "number-color"], "control": "color", "optional": true}), true)]
#[case(json!({"path": ["title", "rule"], "default": null, "control": "rule", "nullable": true}), true)]
#[case(json!({"path": ["card", "bg"], "control": "color"}), false)]
#[case(json!({"path": [], "default": 1, "control": "bool"}), false)]
fn component_token_check_requires_default_unless_optional(#[case] entry: Value, #[case] ok: bool) {
    let token: ComponentToken = serde_json::from_value(entry).unwrap();
    assert_that!(token.check().is_ok()).is_equal_to(ok);
}

#[rstest]
#[case::empty_path(json!({"path": [], "default": 1, "control": "bool"}), TokenEntryErrorKind::EmptyPath)]
#[case::missing_default(json!({"path": ["card", "bg"], "control": "color"}), TokenEntryErrorKind::MissingDefault)]
fn component_token_check_refusal_says_why(#[case] entry: Value, #[case] kind: TokenEntryErrorKind) {
    let token: ComponentToken = serde_json::from_value(entry).unwrap();
    assert_that!(token.check().map_err(|e| e.kind())).is_err_containing(kind);
}

#[test]
fn component_token_literal_null_default_is_kept() {
    let token: ComponentToken =
        serde_json::from_value(json!({"path": ["t", "r"], "default": null, "control": "rule", "nullable": true}))
            .unwrap();
    assert_that!(token.default).is_equal_to(Some(Value::Null));
}

#[test]
fn component_token_absent_default_is_none() {
    let token: ComponentToken =
        serde_json::from_value(json!({"path": ["t", "r"], "control": "color", "optional": true})).unwrap();
    assert_that!(token.default).is_none();
}

/// Une entrée se réécrit telle qu'écrite : pas de clé ajoutée, entiers gardés entiers
/// (le manifeste réécrit garde son empreinte).
#[rstest]
#[case(json!({"path": ["card", "bg"], "default": "bg", "control": "color"}))]
#[case(json!({"path": ["title", "rule"], "default": null, "control": "rule", "nullable": true}))]
#[case(json!({"path": ["card", "pad"], "default": 24, "control": "pt", "min": 0, "max": 200}))]
#[case(json!({"path": ["card", "pad"], "default": "md", "control": "pt", "min": 0.5, "max": 200, "refs": "space"}))]
#[case(json!({"path": ["toc", "cols"], "default": 1, "control": "number", "min": 1, "max": 4, "step": 1, "label": "Columns"}))]
#[case(json!({"path": ["card", "align"], "default": "left", "control": "choice", "options": ["left", "center"]}))]
#[case(json!({"path": ["chapter", "number-color"], "control": "color", "optional": true}))]
fn component_token_round_trips_verbatim(#[case] entry: Value) {
    let token: ComponentToken = serde_json::from_value(entry.clone()).unwrap();
    assert_that!(serde_json::to_value(&token).unwrap()).is_equal_to(entry);
}

#[test]
fn control_reads_its_tag_and_bounds() {
    let control: Control = serde_json::from_value(json!({"control": "em", "min": 0, "max": 2})).unwrap();
    assert_that!(control).is_equal_to(Control::Em { min: 0.0, max: 2.0, refs: None });
}

#[rstest]
#[case::siblings(&[&["card", "bg"][..], &["card", "bg-dark"]], None)]
#[case::prefix(&[&["card"][..], &["card", "bg"]], Some(TokenEntryErrorKind::PathConflict))]
#[case::duplicate(&[&["card", "bg"][..], &["card", "bg"]], Some(TokenEntryErrorKind::PathConflict))]
#[case::empty(&[&[][..]], Some(TokenEntryErrorKind::EmptyPath))]
fn check_entries_refuses_overlapping_paths(#[case] paths: &[&[&str]], #[case] refused: Option<TokenEntryErrorKind>) {
    let tokens: Vec<ComponentToken> = paths
        .iter()
        .map(|p| serde_json::from_value(json!({"path": p, "default": "x", "control": "text"})).unwrap())
        .collect();
    assert_that!(check_entries(&tokens).err().map(|e| e.kind())).is_equal_to(refused);
}
