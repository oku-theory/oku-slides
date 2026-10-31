use std::collections::{BTreeSet, HashSet};

use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{Example, Group, examples};
use crate::catalog_schema_tests::catalog;
use crate::{Component, ServerMessage};

#[test]
fn playbook_examples_cover_every_component_in_catalog_order() {
    let mut seen: Vec<String> = Vec::new();
    for e in examples() {
        if seen.last().map(String::as_str) != Some(e.component) {
            seen.push(e.component.to_owned());
        }
    }
    assert_that!(seen).is_equal_to(catalog());
}

#[test]
fn playbook_variants_are_unique_per_component() {
    let all = examples();
    let keys: HashSet<(&str, &str)> = all.iter().map(|e| (e.component, e.variant)).collect();
    assert_that!(keys.len()).is_equal_to(all.len());
}

#[test]
fn playbook_component_name_matches_the_root_serde_tag() {
    for e in examples() {
        let root = serde_json::to_value(&e.components[0]).unwrap();
        asserting(&format!("{}/{}", e.component, e.variant)).that(&root["component"]).is_equal_to(json!(e.component));
    }
}

#[test]
fn playbook_group_is_host_only_exactly_when_not_agent_allowed() {
    for e in examples() {
        let host_only = e.group == Group::HostOnly;
        asserting(&format!("{}/{}", e.component, e.variant))
            .that(&host_only)
            .is_equal_to(!e.components[0].agent_allowed());
    }
}

#[test]
fn playbook_group_is_field_exactly_when_the_root_is_a_field() {
    for e in examples() {
        asserting(&format!("{}/{}", e.component, e.variant))
            .that(&(e.group == Group::Field))
            .is_equal_to(e.components[0].kind.is_field());
    }
}

#[test]
fn playbook_every_layout_and_atom_component_has_an_agent_example() {
    let all = examples();
    let lacking: BTreeSet<&str> = all
        .iter()
        .filter(|e| matches!(e.group, Group::Layout | Group::Atom))
        .map(|e| e.component)
        .filter(|c| !all.iter().any(|e| e.component == *c && e.agent))
        .collect();
    assert_that!(lacking.into_iter().collect::<Vec<_>>()).is_empty();
}

/// Root first under `root`, ids unique, every child present, every component reachable.
fn tree_fault(e: &Example) -> Option<String> {
    if e.components.first().map(|c| c.id.as_str()) != Some("root") {
        return Some("root is not first".into());
    }
    let ids: BTreeSet<&str> = e.components.iter().map(|c| c.id.as_str()).collect();
    if ids.len() != e.components.len() {
        return Some("duplicate id".into());
    }
    let mut reached = BTreeSet::from(["root"]);
    let mut todo = vec!["root"];
    while let Some(id) = todo.pop() {
        let Some(c) = e.components.iter().find(|c| c.id == id) else { return Some(format!("missing {id}")) };
        for child in c.kind.children() {
            if !ids.contains(child) {
                return Some(format!("missing child {child}"));
            }
            if reached.insert(child) {
                todo.push(child);
            }
        }
    }
    (reached != ids).then(|| "unreachable component".into())
}

#[test]
fn playbook_trees_are_valid() {
    let faults: Vec<String> = examples()
        .iter()
        .filter_map(|e| tree_fault(e).map(|f| format!("{}/{}: {f}", e.component, e.variant)))
        .collect();
    assert_that!(faults).is_empty();
}

#[test]
fn playbook_components_survive_a_serde_round_trip() {
    for e in examples() {
        for c in &e.components {
            let back: Component = serde_json::from_value(serde_json::to_value(c).unwrap()).unwrap();
            assert_that!(back).is_equal_to(c.clone());
        }
    }
}

/// An example as the gallery reads it: its surface as v0.9 messages.
fn ui_entry(e: &Example) -> Value {
    let surface = format!("playbook-{}-{}", e.component, e.variant);
    json!({
        "component": e.component,
        "variant": e.variant,
        "note": e.note,
        "group": e.group,
        "agent": e.agent,
        "surface": surface,
        "messages": [
            ServerMessage::create_surface(&surface),
            ServerMessage::update_components(&surface, e.components.clone()),
            ServerMessage::set_data_model(&surface, e.data.clone()),
        ],
    })
}

/// Contract with the gallery: `ui/src/a2ui/fixtures/playbook.json`. `OKU_UPDATE_FIXTURES=1` rewrites it.
#[test]
fn playbook_ui_fixture_is_current() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/a2ui/fixtures/playbook.json");
    let entries: Vec<Value> = examples().iter().map(ui_entry).collect();
    let json = serde_json::to_string_pretty(&entries).unwrap() + "\n";
    if std::env::var_os("OKU_UPDATE_FIXTURES").is_some() {
        std::fs::write(path, &json).unwrap();
    }
    let fixture = std::fs::read_to_string(path).unwrap_or_default();
    assert_that!(fixture).named("OKU_UPDATE_FIXTURES=1 to regenerate").is_equal_to(json);
}
