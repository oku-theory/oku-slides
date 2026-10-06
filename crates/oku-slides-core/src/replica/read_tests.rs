use loro::{LoroDoc, LoroMap, LoroMovableList};
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::*;
use crate::error::ReplicaError;
use crate::replica::write::json_to_loro;

fn issues_ids(issues: &[crate::Issue]) -> Vec<&'static str> {
    issues.iter().map(|i| i.message.id()).collect()
}

/// Écrit `v` sous `map[key]` sans passer par la réconciliation : objet →
/// `LoroMap`, tableau d'objets → `LoroMovableList` de cartes, reste → valeur.
/// Permet de forger un replica que la réconciliation n'écrirait pas.
fn put(map: &LoroMap, key: &str, v: &Value) {
    match v {
        Value::Object(o) => {
            let m = map.insert_container(key, LoroMap::new()).unwrap();
            o.iter().for_each(|(k, v)| put(&m, k, v));
        }
        Value::Array(a) if !a.is_empty() && a.iter().all(Value::is_object) => {
            let l = map.insert_container(key, LoroMovableList::new()).unwrap();
            for (i, item) in a.iter().enumerate() {
                let m = l.insert_container(i, LoroMap::new()).unwrap();
                item.as_object().unwrap().iter().for_each(|(k, v)| put(&m, k, v));
            }
        }
        v => {
            map.insert(key, json_to_loro(v)).unwrap();
        }
    }
}

fn read(root: Value) -> Result<crate::DeckRead, ReplicaError> {
    let doc = LoroDoc::new();
    let map = doc.get_map("deck");
    root.as_object().unwrap().iter().for_each(|(k, v)| put(&map, k, v));
    doc.commit();
    Ok(deck(&map)?)
}

fn slot<'a>(s: &'a crate::Slide, name: &str) -> &'a [crate::Element] {
    s.params.slot(name).unwrap_or_default()
}

#[test]
fn read_unreadable_element_drops_it_only_and_names_its_slide() {
    let (deck, issues) = read(json!({ "slides": [{ "id": "s1", "kind": "content", "params": { "elements": [
        { "id": "e-ok", "kind": "shape" },
        { "id": "e-ko", "kind": 42 }
    ]}}]}))
    .unwrap();
    assert_that!(slot(&deck.slides[0], "elements").len()).is_equal_to(1);
    assert_that!(issues_ids(&issues)).is_equal_to(vec!["issue-unreadable-element"]);
    assert_that!((issues[0].slide.as_deref(), issues[0].element.as_deref())).is_equal_to((Some("s1"), Some("e-ko")));
}

#[test]
fn read_unreadable_nested_child_drops_the_child_only() {
    let (deck, issues) = read(json!({ "slides": [{ "id": "s1", "kind": "blank", "params": { "elements": [
        { "id": "c", "kind": "canvas", "params": { "children": [
            { "id": "ok", "kind": "shape" },
            { "id": "ko", "kind": "shape", "frame": "pas un cadre" }
        ]}}
    ]}}]}))
    .unwrap();
    let canvas = &slot(&deck.slides[0], "elements")[0];
    assert_that!(canvas.params.slot("children").map(<[crate::Element]>::len)).is_equal_to(Some(1));
    assert_that!(issues[0].element.as_deref()).is_equal_to(Some("ko"));
}

#[test]
fn read_bad_slide_drops_slide_only() {
    let (deck, issues) =
        read(json!({ "slides": [{ "id": "s1", "kind": "content" }, { "id": "s2", "kind": 42 }]})).unwrap();
    assert_that!(deck.slides.len()).is_equal_to(1);
    assert_that!(issues_ids(&issues)).is_equal_to(vec!["issue-unreadable-slide"]);
}

#[test]
fn read_bad_top_level_field_resets_it() {
    let (deck, issues) = read(json!({ "theme": 7, "slides": [] })).unwrap();
    assert_that!(deck.theme).is_none();
    assert_that!(issues_ids(&issues)).is_equal_to(vec!["issue-unreadable-field"]);
}

#[test]
fn read_value_param_holding_an_array_keeps_it_whole() {
    let (deck, issues) =
        read(json!({ "slides": [{ "id": "s1", "kind": "content", "params": { "series": [1, 2] } }]})).unwrap();
    assert_that!(deck.slides[0].params.value("series")).is_equal_to(Some(&json!([1, 2])));
    assert_that!(issues).is_empty();
}

#[test]
fn read_deck_of_version_1_returns_unsupported_version() {
    let err = read(json!({ "version": 1, "slides": [] })).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::ReplicaErrorKind::UnsupportedVersion);
}

#[test]
fn read_deck_of_a_newer_version_returns_unsupported_version() {
    let err = read(json!({ "version": 5, "slides": [] })).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::ReplicaErrorKind::UnsupportedVersion);
}

#[test]
fn read_deck_without_version_reads_as_current() {
    let (deck, issues) = read(json!({ "slides": [] })).unwrap();
    assert_that!(deck.version).is_equal_to(crate::DECK_VERSION);
    assert_that!(issues).is_empty();
}
