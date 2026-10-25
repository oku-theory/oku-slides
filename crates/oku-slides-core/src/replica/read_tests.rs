use loro::{Container, LoroDoc, LoroMap, LoroMovableList, LoroTree, TreeID, TreeParentId, ValueOrContainer};
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use super::*;
use crate::error::ReplicaError;
use crate::replica::schema::{SLIDE, SLOT};
use crate::replica::tree::deck_tree_at;
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
/// nœuds de `tree` sous `parent`, de `TreeID` de propriétaire `owner`.
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

/// Les `elements` du slot `slot` de `owner` (`TreeID` en chaîne), nœuds de `tree` sous `parent`.
fn put_nodes(tree: &LoroTree, parent: TreeParentId, owner: &str, slot: &str, elements: &[Value]) {
    for e in elements {
        let node = tree.create(parent).unwrap();
        let meta = tree.get_meta(node).unwrap();
        put_params(&meta, e.as_object().unwrap(), tree, TreeParentId::Node(node), &node.to_string());
        meta.ensure_mergeable_map(SLOT).unwrap().insert(owner, slot).unwrap();
    }
}

/// La slide `slide`, racine de l'arbre du deck ; ses éléments en descendants.
fn put_slide(tree: &LoroTree, slide: &Map<String, Value>) {
    let n = tree.create(TreeParentId::Root).unwrap();
    let meta = tree.get_meta(n).unwrap();
    meta.insert(SLIDE, true).unwrap();
    put_params(&meta, slide, tree, TreeParentId::Node(n), &n.to_string());
}

/// Le doc forgé de `root`, sans passer par la réconciliation.
fn forged_doc(root: &Value) -> LoroDoc {
    let doc = LoroDoc::new();
    doc.config_text_style(crate::replica::rich::styles());
    let map = doc.get_map("deck");
    for (k, v) in root.as_object().unwrap() {
        match (k.as_str(), v) {
            ("slides", Value::Array(slides)) => {
                let tree = deck_tree_at(&map).unwrap();
                for s in slides {
                    put_slide(&tree, s.as_object().unwrap());
                }
            }
            _ => put(&map, k, v),
        }
    }
    doc.commit();
    doc
}

fn read(root: &Value) -> Result<crate::DeckRead, ReplicaError> {
    Ok(deck(&forged_doc(root).get_map("deck"))?)
}

/// L'arbre du deck de `doc`.
fn deck_tree_of(doc: &LoroDoc) -> LoroTree {
    match doc.get_map("deck").get("slides") {
        Some(ValueOrContainer::Container(Container::Tree(t))) => t,
        other => panic!("arbre du deck attendu : {other:?}"),
    }
}

/// La forme brute d'un arbre du deck, en préordre : par nœud, son id, ses clés
/// réservées (`@…`) triées, et `@slot` où chaque `TreeID` devient l'id de son nœud.
fn raw_form(tree: &LoroTree) -> Vec<(Value, Vec<String>, Value)> {
    let id_of = |n: TreeID| tree.get_meta(n).unwrap().get_value().into_map().unwrap().get("id").map(loro_to_json);
    let mut out = Vec::new();
    let mut stack: Vec<TreeID> = tree.children(TreeParentId::Root).unwrap_or_default().into_iter().rev().collect();
    while let Some(n) = stack.pop() {
        let meta = tree.get_meta(n).unwrap();
        let mut keys: Vec<String> = meta.keys().map(|k| k.to_string()).filter(|k| k.starts_with('@')).collect();
        keys.sort();
        let slot = match meta.get(SLOT) {
            Some(ValueOrContainer::Container(Container::Map(m))) => Value::Object(
                m.keys()
                    .map(|k| {
                        let owner =
                            TreeID::try_from(&*k).ok().and_then(id_of).and_then(|v| v.as_str().map(str::to_owned));
                        (
                            owner.unwrap_or_else(|| k.to_string()),
                            loro_to_json(&m.get_value()).get(&*k).cloned().unwrap_or_default(),
                        )
                    })
                    .collect(),
            ),
            _ => Value::Null,
        };
        out.push((id_of(n).unwrap_or_default(), keys, slot));
        stack.extend(tree.children(TreeParentId::Node(n)).unwrap_or_default().into_iter().rev());
    }
    out
}

#[test]
fn put_slide_matches_the_writer() {
    let roadmap = crate::Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap();
    let json = serde_json::to_value(&roadmap).unwrap();
    let (forged, issues) = read(&json).unwrap();
    let replica = crate::DeckReplica::new(&roadmap, crate::test_registry::shared()).unwrap();
    assert_that!(forged).is_equal_to(replica.deck().0);
    assert_that!(issues).is_empty();
    assert_that!(raw_form(&deck_tree_of(&forged_doc(&json)))).is_equal_to(raw_form(&deck_tree_of(&replica.doc)));
}

fn slot<'a>(s: &'a crate::Slide, name: &str) -> &'a [crate::Element] {
    s.params.slot(name).unwrap_or_default()
}

#[test]
fn read_unreadable_element_drops_it_only_and_names_its_slide() {
    let (deck, issues) = read(&json!({ "slides": [{ "id": "s1", "kind": "essentiel/content", "params": { "elements": [
        { "id": "e-ok", "kind": "base/shape" },
        { "id": "e-ko", "kind": 42 }
    ]}}]}))
    .unwrap();
    assert_that!(slot(&deck.slides[0], "elements").len()).is_equal_to(1);
    assert_that!(issues_ids(&issues)).is_equal_to(vec!["issue-unreadable-element"]);
    assert_that!((issues[0].slide.as_deref(), issues[0].element.as_deref())).is_equal_to((Some("s1"), Some("e-ko")));
}

#[test]
fn read_unreadable_nested_child_drops_the_child_only() {
    let (deck, issues) = read(&json!({ "slides": [{ "id": "s1", "kind": "essentiel/blank", "params": { "elements": [
        { "id": "c", "kind": "test/frame-box", "params": { "children": [
            { "id": "ok", "kind": "base/shape" },
            { "id": "ko", "kind": "base/shape", "frame": "pas un cadre" }
        ]}}
    ]}}]}))
    .unwrap();
    let frame_box = &slot(&deck.slides[0], "elements")[0];
    assert_that!(frame_box.params.slot("children").map(<[crate::Element]>::len)).is_equal_to(Some(1));
    assert_that!(issues[0].element.as_deref()).is_equal_to(Some("ko"));
}

#[test]
fn read_bad_slide_drops_slide_only() {
    let (deck, issues) =
        read(&json!({ "slides": [{ "id": "s1", "kind": "essentiel/content" }, { "id": "s2", "kind": 42 }]})).unwrap();
    assert_that!(deck.slides.len()).is_equal_to(1);
    assert_that!(issues_ids(&issues)).is_equal_to(vec!["issue-unreadable-slide"]);
}

#[test]
fn read_bad_top_level_field_resets_it() {
    let (deck, issues) = read(&json!({ "theme": 7, "slides": [] })).unwrap();
    assert_that!(deck.theme).is_none();
    assert_that!(issues_ids(&issues)).is_equal_to(vec!["issue-unreadable-field"]);
}

#[test]
fn read_value_param_holding_an_array_keeps_it_whole() {
    let (deck, issues) =
        read(&json!({ "slides": [{ "id": "s1", "kind": "essentiel/content", "params": { "series": [1, 2] } }]}))
            .unwrap();
    assert_that!(deck.slides[0].params.value("series")).is_equal_to(Some(&json!([1, 2])));
    assert_that!(issues).is_empty();
}

#[test]
fn read_deck_of_version_1_returns_unsupported_version() {
    let err = read(&json!({ "version": 1, "slides": [] })).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::ReplicaErrorKind::UnsupportedVersion);
}

#[test]
fn read_deck_of_a_newer_version_returns_unsupported_version() {
    let err = read(&json!({ "version": crate::DECK_VERSION + 1, "slides": [] })).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::ReplicaErrorKind::UnsupportedVersion);
}

#[test]
fn read_deck_of_version_6_returns_unsupported_version() {
    let err = read(&json!({ "version": 6, "slides": [] })).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::ReplicaErrorKind::UnsupportedVersion);
}

#[test]
fn read_deck_of_version_7_returns_unsupported_version() {
    let err = read(&json!({ "version": 7, "slides": [] })).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::ReplicaErrorKind::UnsupportedVersion);
}

#[test]
fn read_deck_without_version_reads_as_current() {
    let (deck, issues) = read(&json!({ "slides": [] })).unwrap();
    assert_that!(deck.version).is_equal_to(crate::DECK_VERSION);
    assert_that!(issues).is_empty();
}

fn slide_ids(deck: &crate::Deck) -> Vec<&str> {
    deck.slides.iter().map(|s| s.id.as_str()).collect()
}

#[test]
fn read_slide_copied_twice_keeps_first_occurrence() {
    let (deck, issues) = read(&json!({ "slides": [
        { "id": "s1", "kind": "essentiel/content", "notes": "a" },
        { "id": "s2", "kind": "essentiel/content" },
        { "id": "s1", "kind": "essentiel/content", "notes": "a" }
    ]}))
    .unwrap();
    assert_that!(slide_ids(&deck)).is_equal_to(vec!["s1", "s2"]);
    assert_that!(issues).is_empty();
}

#[test]
fn read_diverged_slides_sharing_an_id_renames_the_later_one() {
    let (deck, issues) = read(&json!({ "slides": [
        { "id": "s1", "kind": "essentiel/content", "notes": "a" },
        { "id": "s1-2", "kind": "essentiel/content" },
        { "id": "s1", "kind": "essentiel/content", "notes": "b" }
    ]}))
    .unwrap();
    assert_that!(slide_ids(&deck)).is_equal_to(vec!["s1", "s1-2", "s1-3"]);
    assert_that!(deck.slides[2].notes.as_str()).is_equal_to("b");
    assert_that!(issues).is_empty();
}

#[test]
fn read_diverged_elements_sharing_an_id_rename_the_copy_and_its_children() {
    let stack = |title: &str| json!({ "id": "c", "kind": "base/stack", "params": { "title": title, "children": [{ "id": "x", "kind": "base/shape" }] } });
    let (deck, issues) = read(&json!({ "slides": [{ "id": "s1", "kind": "essentiel/blank", "params": { "elements": [
        stack("a"), stack("a"), stack("b")
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
    let (deck, _) = read(&json!({ "slides": [
        { "id": "x", "kind": "essentiel/content", "notes": "A" },
        { "id": "x", "kind": "essentiel/content", "notes": "B" },
        { "id": "x", "kind": "essentiel/content", "notes": "B" }
    ]}))
    .unwrap();
    assert_that!(slide_ids(&deck)).is_equal_to(vec!["x", "x-2"]);
}
