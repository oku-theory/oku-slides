use oku_slides_core::WidgetErrorKind;
use serde_json::json;
use speculoos::prelude::*;

use std::sync::Arc;

use super::{PackWidgets, Registries, probe_json, probe_registry, registry_from_probe};
use crate::error::RegistryErrorKind;
use crate::fonts::Fonts;
use crate::load_registry;

#[test]
fn probe_json_no_metadata_returns_missing() {
    let err = probe_json(None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RegistryErrorKind::Missing);
}

#[test]
fn probe_json_valid_json_returns_it() {
    let value = probe_json(Some(Ok(json!({ "base": [] })))).unwrap();
    assert_that!(value).is_equal_to(json!({ "base": [] }));
}

#[test]
fn probe_json_conversion_failure_returns_schema_unreadable() {
    let source = serde_json::from_str::<i32>("nope").unwrap_err();
    let err = probe_json(Some(Err(source))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RegistryErrorKind::Schema);
    assert_that!(err.widget_error().map(|e| e.kind())).is_equal_to(Some(WidgetErrorKind::Unreadable));
}

#[test]
fn registry_from_probe_of_base_probe_equals_load_registry() {
    let json = probe_registry(&Fonts::default(), None).unwrap();
    let from = registry_from_probe(json).unwrap();
    assert_that!(from).is_equal_to(load_registry(&Fonts::default(), None).unwrap());
}

#[test]
fn pack_fingerprint_changes_with_source() {
    let a = PackWidgets::new("p", "#let registry = ()").unwrap();
    let b = PackWidgets::new("p", "#let registry = () ").unwrap();
    assert_that!(a.fingerprint()).is_not_equal_to(b.fingerprint());
}

#[test]
fn registries_probe_is_memoized_by_fingerprint() {
    let mut r = Registries::new(Fonts::default());
    let pack = PackWidgets::new("p", "#let registry = ()").unwrap();
    let first = r.probe(&pack).unwrap();
    let second = r.probe(&pack).unwrap();
    assert_that!(Arc::ptr_eq(&first, &second)).is_true();
}
