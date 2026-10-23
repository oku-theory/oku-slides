use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use super::{effective_tokens, merge, theme_layer};
use crate::Namespace;

fn ns(s: &str) -> Namespace {
    Namespace::parse(s).unwrap_or_else(|| panic!("espace de noms invalide : {s}"))
}

fn map(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        _ => panic!("objet attendu"),
    }
}

#[test]
fn merge_of_nested_objects_keeps_the_keys_it_does_not_override() {
    let mut base = map(json!({"color": {"ink": "#000000", "bg": "#ffffff"}, "page": {"width": 1920}}));
    merge(&mut base, &map(json!({"color": {"ink": "#222222"}})));
    assert_that!(Value::Object(base))
        .is_equal_to(json!({"color": {"ink": "#222222", "bg": "#ffffff"}, "page": {"width": 1920}}));
}

#[test]
fn merge_of_a_non_object_replaces_the_value() {
    let mut base = map(json!({"component": {"title": {"rule": {"height": 5}}}, "size": {"body": 32}}));
    merge(&mut base, &map(json!({"component": {"title": {"rule": null}}, "size": 40})));
    assert_that!(Value::Object(base)).is_equal_to(json!({"component": {"title": {"rule": null}}, "size": 40}));
}

#[test]
fn effective_tokens_layer_the_theme_over_the_defaults_without_touching_them() {
    let defaults = map(json!({"color": {"ink": "#000000", "accent": "#2f5fb3"}}));
    let tokens = effective_tokens(&defaults, &map(json!({"color": {"accent": "#00bef4", "white": "#ffffff"}})));
    assert_that!(Value::Object(tokens))
        .is_equal_to(json!({"color": {"ink": "#000000", "accent": "#00bef4", "white": "#ffffff"}}));
    assert_that!(defaults.get("color").and_then(|c| c.get("accent"))).is_equal_to(Some(&json!("#2f5fb3")));
}

#[test]
fn theme_layer_later_parent_wins_and_list_replaces() {
    let components = [(ns("base"), map(json!({"card": {"bg": "bg", "gap": "sm"}})))];
    let papier = map(json!({"color": {"accent": "#111"}, "font": {"body": ["A", "B"]}}));
    let sombre = map(
        json!({"color": {"accent": "#222"}, "font": {"body": ["C"]}, "component": {"base": {"card": {"bg": "ink"}}}}),
    );
    let layer = theme_layer(components.iter().map(|(n, m)| (n, m.clone())), [&papier, &sombre]);
    assert_that!(Value::Object(layer)).is_equal_to(json!({
        "component": {"base": {"card": {"bg": "ink", "gap": "sm"}}},
        "color": {"accent": "#222"}, "font": {"body": ["C"]},
    }));
}

#[test]
fn theme_layer_without_components_or_chain_is_empty() {
    let layer = theme_layer(std::iter::empty(), std::iter::empty());
    assert_that!(layer.is_empty()).is_true();
}

#[test]
fn theme_layer_skips_a_pack_without_component_defaults() {
    let components = [(ns("base"), Map::new())];
    let layer = theme_layer(components.iter().map(|(n, m)| (n, m.clone())), std::iter::empty());
    assert_that!(Value::Object(layer)).is_equal_to(json!({}));
}
