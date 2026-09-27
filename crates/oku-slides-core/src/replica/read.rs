//! Lecture : conteneurs Loro → JSON du deck, puis JSON → `Deck` en lecture
//! totale (rien ne fait échouer [`materialize`] ; l'illisible est écarté et
//! signalé par une [`Issue`]).

use loro::{Container, LoroMap, LoroMovableList, LoroValue, ValueOrContainer};
use oku_slides_i18n::Message;
use serde_json::{Map, Value};

use super::schema::{Ctx, Kind};
use crate::model::{Deck, Element, Slide};
use crate::validate::Issue;

/// Profondeur maximale explorée dans un replica : au-delà, le sous-arbre est
/// ignoré. Un blob non fiable peut imbriquer arbitrairement des cartes ;
/// sans cette borne, la récursion de [`read_map`] pourrait épuiser la pile
/// (un `catch_unwind` ne rattrape pas un débordement de pile).
pub(super) const MAX_DEPTH: u32 = 64;

/// Point d'entrée : profondeur 0 (utilisé pour une valeur autonome, par
/// exemple une marque de texte riche).
pub(super) fn loro_to_json(v: &LoroValue) -> Value {
    loro_to_json_at(v, 0)
}

/// Une `LoroValue` peut elle-même imbriquer listes et cartes sans passer par
/// un conteneur (`LoroValue::List`/`LoroValue::Map`) : un blob non fiable
/// peut donc faire déborder la pile ici aussi si on ne borne pas cette
/// récursion, séparée de celle de [`read_map_at`] sur les conteneurs.
fn loro_to_json_at(v: &LoroValue, depth: u32) -> Value {
    if depth >= MAX_DEPTH {
        return Value::Null;
    }
    match v {
        LoroValue::Null | LoroValue::Binary(_) | LoroValue::Container(_) => Value::Null,
        LoroValue::Bool(b) => Value::Bool(*b),
        LoroValue::Double(d) => serde_json::json!(d),
        LoroValue::I64(i) => serde_json::json!(i),
        LoroValue::String(s) => Value::String(s.to_string()),
        LoroValue::List(l) => Value::Array(l.iter().map(|v| loro_to_json_at(v, depth + 1)).collect()),
        LoroValue::Map(m) => {
            Value::Object(m.iter().map(|(k, v)| (k.to_string(), loro_to_json_at(v, depth + 1))).collect())
        }
    }
}

/// Point d'entrée : profondeur 0.
pub(super) fn read_map(map: &LoroMap, ctx: Ctx) -> Map<String, Value> {
    read_map_at(map, ctx, 0)
}

fn read_map_at(map: &LoroMap, ctx: Ctx, depth: u32) -> Map<String, Value> {
    let mut out = Map::new();
    // Bornée : un replica non fiable peut imbriquer des cartes sans limite ;
    // au-delà, on ignore le sous-arbre plutôt que de récurser indéfiniment.
    if depth >= MAX_DEPTH {
        return out;
    }
    for key in map.keys() {
        let key = key.to_string();
        let Some(entry) = map.get(&key) else { continue };
        let value = match (ctx.kind(&key), entry) {
            (Kind::Rich, ValueOrContainer::Container(Container::Text(t))) => {
                let paragraphs = super::rich::read(&t);
                // Vide = absent.
                if paragraphs.is_empty() {
                    continue;
                }
                Value::Array(paragraphs)
            }
            (_, ValueOrContainer::Container(Container::Text(t))) => {
                let s = t.to_string();
                // Vide = absent.
                if s.trim().is_empty() {
                    continue;
                }
                Value::String(s)
            }
            (Kind::Collection(child), ValueOrContainer::Container(Container::MovableList(l))) => {
                let items = read_list(&l, child, depth + 1);
                // `slides` est obligatoire ; les autres collections vides sont omises.
                if items.is_empty() && ctx != Ctx::Deck {
                    continue;
                }
                Value::Array(items)
            }
            (_, ValueOrContainer::Container(Container::Map(m))) => {
                let child = match ctx.kind(&key) {
                    Kind::Nested(c) => c,
                    _ => Ctx::Opaque,
                };
                Value::Object(read_map_at(&m, child, depth + 1))
            }
            (_, ValueOrContainer::Value(v)) => loro_to_json_at(&v, depth + 1),
            // Conteneur d'un type inattendu : ignoré.
            _ => continue,
        };
        out.insert(key, value);
    }
    out
}

fn read_list(list: &LoroMovableList, ctx: Ctx, depth: u32) -> Vec<Value> {
    (0..list.len())
        .filter_map(|i| match list.get(i) {
            Some(ValueOrContainer::Container(Container::Map(m))) => Some(Value::Object(read_map_at(&m, ctx, depth))),
            _ => None,
        })
        .collect()
}

fn issue(slide: Option<String>, element: Option<String>, message: Message) -> Issue {
    Issue { slide, element, message }
}

/// Garde les éléments lisibles (récursivement dans les groupes), signale les autres.
fn clean_elements(list: Vec<Value>, slide: &str, issues: &mut Vec<Issue>) -> Vec<Value> {
    list.into_iter()
        .filter_map(|mut e| {
            if let Some(Value::Array(children)) = e.get_mut("children") {
                *children = clean_elements(std::mem::take(children), slide, issues);
            }
            match serde_json::from_value::<Element>(e.clone()) {
                Ok(_) => Some(e),
                Err(_) => {
                    let id = e.get("id").and_then(Value::as_str).map(str::to_owned);
                    issues.push(issue(Some(slide.to_owned()), id, Message::new("issue-unreadable-element")));
                    None
                }
            }
        })
        .collect()
}

fn read_slide(mut s: Value, issues: &mut Vec<Issue>) -> Option<Slide> {
    let id = s.get("id").and_then(Value::as_str).unwrap_or_default().to_owned();
    for key in ["blocks", "elements"] {
        if let Some(Value::Array(list)) = s.get_mut(key) {
            *list = clean_elements(std::mem::take(list), &id, issues);
        }
    }
    match serde_json::from_value::<Slide>(s) {
        Ok(slide) => Some(slide),
        Err(_) => {
            issues.push(issue(Some(id), None, Message::new("issue-unreadable-slide")));
            None
        }
    }
}

/// Lecture totale : tout état du replica donne un `Deck`. Ce qui ne se lit
/// pas est écarté (élément, slide) ou remis par défaut (champ du deck), et signalé.
pub(super) fn materialize(mut root: Map<String, Value>) -> (Deck, Vec<Issue>) {
    let mut issues = Vec::new();
    let slides: Vec<Slide> = match root.remove("slides") {
        Some(Value::Array(a)) => a.into_iter().filter_map(|s| read_slide(s, &mut issues)).collect(),
        _ => Vec::new(),
    };
    let keys: Vec<String> = root.keys().cloned().collect();
    for key in keys {
        let mut probe = Map::new();
        probe.insert("slides".into(), Value::Array(Vec::new()));
        if let Some(v) = root.get(&key) {
            probe.insert(key.clone(), v.clone());
        }
        if serde_json::from_value::<Deck>(Value::Object(probe)).is_err() {
            root.remove(&key);
            issues.push(issue(None, None, Message::new("issue-unreadable-field").arg("field", key)));
        }
    }
    root.insert("slides".into(), Value::Array(Vec::new()));
    let mut deck = serde_json::from_value::<Deck>(Value::Object(root)).unwrap_or_else(|_| Deck {
        version: crate::model::DECK_VERSION,
        theme: None,
        params: Default::default(),
        vars: Default::default(),
        tokens: Value::Null,
        templates: None,
        slides: Vec::new(),
    });
    deck.slides = slides;
    (deck, issues)
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod read_tests;
