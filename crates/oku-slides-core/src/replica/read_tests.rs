use loro::{LoroDoc, LoroMap, LoroMovableList, LoroTree, TreeParentId};
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use super::*;
use crate::error::ReplicaError;
use crate::replica::schema::SLOT;
use crate::replica::tree::tree_at;
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

/// Un tableau non vide d'objets portant tous `kind` : les éléments d'un slot.
fn as_elements(v: &Value) -> Option<&[Value]> {
    let a = v.as_array()?;
    (!a.is_empty() && a.iter().all(|e| e.get("kind").is_some())).then_some(a.as_slice())
}

/// Écrit les `params` de `obj` sous `map` ; leurs slots deviennent des
/// nœuds de `tree` sous `parent`, d'id de propriétaire `owner`.
fn put_params(map: &LoroMap, obj: &Map<String, Value>, tree: &LoroTree, parent: TreeParentId, owner: &str) {
    for (k, v) in obj {
        match (k.as_str(), v) {
            ("params", Value::Object(params)) => {
                let m = map.insert_container("params", LoroMap::new()).unwrap();
                for (name, value) in params {
                    match as_elements(value) {
                        Some(elements) => put_nodes(tree, parent, owner, name, elements),
                        // Texte riche : sa carte `{rich: LoroText}`, comme l'écrit la réconciliation.
                        None if value.as_array().is_some_and(|a| crate::params::is_paragraphs(a)) => {
                            let rich = m.insert_container(name, LoroMap::new()).unwrap();
                            let text = rich.insert_container(RICH, loro::LoroText::new()).unwrap();
                            crate::replica::rich::sync(&text, value).unwrap();
                        }
                        None => put(&m, name, value),
                    }
                }
            }
            _ => put(map, k, v),
        }
    }
}

/// Les `elements` du slot `slot` de `owner`, nœuds de `tree` sous `parent`.
fn put_nodes(tree: &LoroTree, parent: TreeParentId, owner: &str, slot: &str, elements: &[Value]) {
    for e in elements {
        let node = tree.create(parent).unwrap();
        let meta = tree.get_meta(node).unwrap();
        let id = e.get("id").and_then(Value::as_str).unwrap_or_default();
        put_params(&meta, e.as_object().unwrap(), tree, TreeParentId::Node(node), id);
        meta.insert_container(SLOT, LoroMap::new()).unwrap().insert(owner, slot).unwrap();
    }
}

/// La slide `slide` en `i` de `list` ; ses éléments dans son arbre.
fn put_slide(list: &LoroMovableList, i: usize, slide: &Map<String, Value>) {
    let m = list.insert_container(i, LoroMap::new()).unwrap();
    let tree = tree_at(&m, crate::replica::schema::ELEMENTS).unwrap();
    put_params(&m, slide, &tree, TreeParentId::Root, crate::replica::schema::ROOT_OWNER);
}

fn read(root: Value) -> Result<crate::DeckRead, ReplicaError> {
    let doc = LoroDoc::new();
    doc.config_text_style(crate::replica::rich::styles());
    let map = doc.get_map("deck");
    for (k, v) in root.as_object().unwrap() {
        match (k.as_str(), v) {
            ("slides", Value::Array(slides)) => {
                let list = map.insert_container("slides", LoroMovableList::new()).unwrap();
                for (i, s) in slides.iter().enumerate() {
                    put_slide(&list, i, s.as_object().unwrap());
                }
            }
            _ => put(&map, k, v),
        }
    }
    doc.commit();
    Ok(deck(&map)?)
}

#[test]
fn put_slide_matches_the_writer() {
    let roadmap = crate::Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap();
    let (forged, issues) = read(serde_json::to_value(&roadmap).unwrap()).unwrap();
    let written = crate::DeckReplica::new(&roadmap, crate::test_registry::shared()).unwrap().deck().0;
    assert_that!(forged).is_equal_to(written);
    assert_that!(issues).is_empty();
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
    let err = read(json!({ "version": 6, "slides": [] })).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::ReplicaErrorKind::UnsupportedVersion);
}

#[test]
fn read_deck_without_version_reads_as_current() {
    let (deck, issues) = read(json!({ "slides": [] })).unwrap();
    assert_that!(deck.version).is_equal_to(crate::DECK_VERSION);
    assert_that!(issues).is_empty();
}

fn slide_ids(deck: &crate::Deck) -> Vec<&str> {
    deck.slides.iter().map(|s| s.id.as_str()).collect()
}

#[test]
fn read_slide_copied_twice_keeps_first_occurrence() {
    let (deck, issues) = read(json!({ "slides": [
        { "id": "s1", "kind": "content", "notes": "a" },
        { "id": "s2", "kind": "content" },
        { "id": "s1", "kind": "content", "notes": "a" }
    ]}))
    .unwrap();
    assert_that!(slide_ids(&deck)).is_equal_to(vec!["s1", "s2"]);
    assert_that!(issues).is_empty();
}

#[test]
fn read_diverged_slides_sharing_an_id_renames_the_later_one() {
    let (deck, issues) = read(json!({ "slides": [
        { "id": "s1", "kind": "content", "notes": "a" },
        { "id": "s1-2", "kind": "content" },
        { "id": "s1", "kind": "content", "notes": "b" }
    ]}))
    .unwrap();
    assert_that!(slide_ids(&deck)).is_equal_to(vec!["s1", "s1-2", "s1-3"]);
    assert_that!(deck.slides[2].notes.as_str()).is_equal_to("b");
    assert_that!(issues).is_empty();
}

#[test]
fn read_diverged_elements_sharing_an_id_rename_the_copy_and_its_children() {
    let canvas = |title: &str| json!({ "id": "c", "kind": "canvas", "params": { "title": title, "children": [{ "id": "x", "kind": "shape" }] } });
    let (deck, issues) = read(json!({ "slides": [{ "id": "s1", "kind": "blank", "params": { "elements": [
        canvas("a"), canvas("a"), canvas("b")
    ]}}]}))
    .unwrap();
    let ids: Vec<(&str, Vec<&str>)> = slot(&deck.slides[0], "elements")
        .iter()
        .map(|e| (e.id.as_str(), e.params.slot("children").unwrap_or_default().iter().map(|c| c.id.as_str()).collect()))
        .collect();
    assert_that!(ids).is_equal_to(vec![("c", vec!["x"]), ("c-2", vec!["x-2"])]);
    assert_that!(issues).is_empty();
}

#[test]
fn read_copy_of_a_renamed_slide_is_dropped() {
    let (deck, _) = read(json!({ "slides": [
        { "id": "x", "kind": "content", "notes": "A" },
        { "id": "x", "kind": "content", "notes": "B" },
        { "id": "x", "kind": "content", "notes": "B" }
    ]}))
    .unwrap();
    assert_that!(slide_ids(&deck)).is_equal_to(vec!["x", "x-2"]);
}
