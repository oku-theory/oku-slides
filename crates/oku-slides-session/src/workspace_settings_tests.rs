use std::sync::Mutex;

use serde_json::json;
use speculoos::prelude::*;

use super::workspace_fixtures::Data;
use super::*;
use crate::test_support::{apply_ws, workspace_on, workspace_platform};

#[test]
fn messages_say_the_chosen_language() {
    let data = Data::new();
    let json = apply_ws(&workspace_on(&data.0), json!({"cmd": "messages"})).unwrap().into_json();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    // Les réglages de test disent `fr`.
    assert_that!(v["chosen"].as_str()).is_equal_to(Some("fr"));
}

#[test]
fn set_locale_answers_with_the_messages_of_that_language_and_keeps_it() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let json = apply_ws(&ws, json!({"cmd": "set_locale", "locale": "de"})).unwrap().into_json();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_that!(v["locale"].as_str()).is_equal_to(Some("de"));
    assert_that!(v["chosen"].as_str()).is_equal_to(Some("de"));
    let config = std::fs::read_to_string(data.0.join("config.toml")).unwrap();
    assert_that!(config.contains("locale = \"de\"")).is_true();
}

#[test]
fn set_locale_null_follows_the_system_again() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "set_locale", "locale": "de"})).unwrap();
    let json = apply_ws(&ws, json!({"cmd": "set_locale", "locale": null})).unwrap().into_json();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_that!(v["chosen"].is_null()).is_true();
    let config = std::fs::read_to_string(data.0.join("config.toml")).unwrap();
    assert_that!(config.contains("locale")).is_false();
}

#[test]
fn set_locale_unknown_language_follows_the_system() {
    let data = Data::new();
    let json = apply_ws(&workspace_on(&data.0), json!({"cmd": "set_locale", "locale": "xx"})).unwrap().into_json();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_that!(v["chosen"].is_null()).is_true();
}

#[test]
fn messages_give_the_ui_its_language_and_fluent_resources() {
    let data = Data::new();
    let json = apply_ws(&workspace_on(&data.0), json!({"cmd": "messages"})).unwrap().into_json();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_that!(v["locale"].as_str()).is_equal_to(Some("fr"));
    let resources: Vec<String> = serde_json::from_value(v["resources"].clone()).unwrap();
    let expected: Vec<String> = crate::Localizer::new(Some("fr")).ui_messages().into_iter().map(String::from).collect();
    assert_that!(resources).is_equal_to(expected);
}

#[test]
fn scheme_without_setting_follows_the_system() {
    let data = Data::new();
    let json = apply_ws(&workspace_on(&data.0), json!({"cmd": "scheme"})).unwrap().into_json();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_that!(v["scheme"].is_null()).is_true();
}

#[test]
fn set_scheme_dark_answers_with_it_and_keeps_it() {
    let data = Data::new();
    let json = apply_ws(&workspace_on(&data.0), json!({"cmd": "set_scheme", "scheme": "dark"})).unwrap().into_json();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_that!(v["scheme"].as_str()).is_equal_to(Some("dark"));
    let config = std::fs::read_to_string(data.0.join("config.toml")).unwrap();
    assert_that!(config.contains("scheme = \"dark\"")).is_true();
}

#[test]
fn set_scheme_null_follows_the_system_again() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "set_scheme", "scheme": "light"})).unwrap();
    apply_ws(&ws, json!({"cmd": "set_scheme", "scheme": null})).unwrap();
    let config = std::fs::read_to_string(data.0.join("config.toml")).unwrap();
    assert_that!(config.contains("scheme")).is_false();
}

#[test]
fn set_scheme_unknown_value_follows_the_system() {
    let data = Data::new();
    let json = apply_ws(&workspace_on(&data.0), json!({"cmd": "set_scheme", "scheme": "sepia"})).unwrap().into_json();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_that!(v["scheme"].is_null()).is_true();
}

#[test]
fn scheme_reads_the_setting_of_the_platform() {
    let data = Data::new();
    let mut settings = (*workspace_platform(&data.0).settings).clone();
    settings.appearance.scheme = Some("light".into());
    let ws = Mutex::new(Workspace::new(Fonts::default(), &Platform::with_settings(settings).unwrap(), None).unwrap());
    let json = apply_ws(&ws, json!({"cmd": "scheme"})).unwrap().into_json();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_that!(v["scheme"].as_str()).is_equal_to(Some("light"));
}
