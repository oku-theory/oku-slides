//! Paramètres d'un widget : valeurs nommées d'une slide ou d'un élément.
//!
//! Le type Rust d'une valeur se déduit de la forme de son JSON, sans le
//! registre : les ops arrivent désérialisées par serde aux frontières Tauri
//! et MCP. Le registre s'applique ensuite (ops, validation, réconciliation).

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::model::{Element, Paragraph};

/// Un item de liste (`list`) : son id, unique dans la liste, et ses
/// valeurs par colonne.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
    /// Stable id of the item, unique in the list and without `/`.
    pub id: String,
    /// Values by column name; an absent column is empty.
    #[serde(flatten)]
    pub values: Params,
}

/// Une valeur de paramètre.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ParamValue {
    /// Texte simple (`plain`), ou valeur de `choice` et d'`asset`.
    Text(String),
    /// Paragraphes à runs (`rich`) : objets aux seules clés d'un paragraphe
    /// et d'un run. Un tableau vide se lit ici.
    Rich(Vec<Paragraph>),
    /// Éléments enfants (`slot`) : tableau non vide d'objets à `id` et `kind`.
    Slot(Vec<Element>),
    /// Items d'une liste (`list`) : tableau non vide d'objets à `id` chaîne,
    /// sans `kind`.
    List(Vec<ListItem>),
    /// Toute autre valeur : booléen, nombre, grille, options de graphique…
    /// et tout tableau qui n'est ni l'un ni l'autre, gardé tel quel.
    Value(Value),
}

impl<'de> Deserialize<'de> for ParamValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        Ok(match v {
            Value::String(s) => ParamValue::Text(s),
            Value::Array(a) if is_elements(&a) => {
                ParamValue::Slot(serde_json::from_value(Value::Array(a)).map_err(serde::de::Error::custom)?)
            }
            Value::Array(a) if is_items(&a) => {
                ParamValue::List(serde_json::from_value(Value::Array(a)).map_err(serde::de::Error::custom)?)
            }
            Value::Array(a) if is_paragraphs(&a) => {
                ParamValue::Rich(serde_json::from_value(Value::Array(a)).map_err(serde::de::Error::custom)?)
            }
            other => ParamValue::Value(other),
        })
    }
}

/// Champs d'un [`Paragraph`] et d'un [`Run`](crate::Run) : un objet qui en a
/// d'autres n'est pas un paragraphe (rien ne se perd).
const PARAGRAPH_KEYS: [&str; 2] = ["runs", "list"];
const RUN_KEYS: [&str; 7] = ["t", "bold", "italic", "color", "size", "link", "highlight"];

/// Liste d'éléments : non vide, chacun avec `id` et `kind` en chaîne.
pub(crate) fn is_elements(a: &[Value]) -> bool {
    let is_element =
        |x: &Value| x.get("id").is_some_and(Value::is_string) && x.get("kind").is_some_and(Value::is_string);
    !a.is_empty() && a.iter().all(is_element)
}

/// Items de liste : non vide, chacun avec `id` en chaîne et sans `kind`
/// (un élément en a un).
pub(crate) fn is_items(a: &[Value]) -> bool {
    let is_item = |x: &Value| x.get("id").is_some_and(Value::is_string) && x.get("kind").is_none();
    !a.is_empty() && a.iter().all(is_item)
}

/// Paragraphes : chaque objet n'a que des clés de [`Paragraph`], et ses
/// `runs` que des clés de [`Run`](crate::Run). Un tableau vide en est.
pub(crate) fn is_paragraphs(a: &[Value]) -> bool {
    let only = |x: &Value, keys: &[&str]| x.as_object().is_some_and(|o| o.keys().all(|k| keys.contains(&k.as_str())));
    let runs_ok = |x: &Value| match x.get("runs") {
        None => true,
        Some(Value::Array(runs)) => runs.iter().all(|r| only(r, &RUN_KEYS)),
        Some(_) => false,
    };
    a.iter().all(|x| only(x, &PARAGRAPH_KEYS) && runs_ok(x))
}

/// Les paramètres d'une slide ou d'un élément, par nom.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Params(pub BTreeMap<String, ParamValue>);

impl Params {
    /// The value of the parameter `name`, if it is set.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ParamValue> {
        self.0.get(name)
    }

    /// Is no parameter set?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Pose une valeur ; renvoie l'ancienne.
    pub fn set(&mut self, name: impl Into<String>, value: ParamValue) -> Option<ParamValue> {
        self.0.insert(name.into(), value)
    }

    /// Removes the parameter `name`; returns its former value.
    pub fn remove(&mut self, name: &str) -> Option<ParamValue> {
        self.0.remove(name)
    }

    /// Texte simple.
    #[must_use]
    pub fn text(&self, name: &str) -> Option<&str> {
        match self.get(name)? {
            ParamValue::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Paragraphes ; un slot vide se lit comme un texte vide.
    #[must_use]
    pub fn rich(&self, name: &str) -> Option<&[Paragraph]> {
        match self.get(name)? {
            ParamValue::Rich(p) => Some(p),
            ParamValue::Slot(s) if s.is_empty() => Some(&[]),
            _ => None,
        }
    }

    /// Éléments d'un slot ; un texte riche vide se lit comme un slot vide.
    #[must_use]
    pub fn slot(&self, name: &str) -> Option<&[Element]> {
        match self.get(name)? {
            ParamValue::Slot(s) => Some(s),
            ParamValue::Rich(p) if p.is_empty() => Some(&[]),
            _ => None,
        }
    }

    /// Le slot `name`, créé vide s'il est absent ou s'il porte une autre
    /// valeur. Toujours `Some` ; l'`Option` évite une branche qui panique.
    pub fn slot_mut(&mut self, name: &str) -> Option<&mut Vec<Element>> {
        let value = self.0.entry(name.to_owned()).or_insert_with(|| ParamValue::Slot(Vec::new()));
        if !matches!(value, ParamValue::Slot(_)) {
            *value = ParamValue::Slot(Vec::new());
        }
        match value {
            ParamValue::Slot(list) => Some(list),
            // Impossible : la valeur vient d'être posée en `Slot`. Branche
            // gardée pour ne pas paniquer (pas d'`unreachable!` hors tests).
            _ => None,
        }
    }

    /// Items d'une liste ; un texte riche ou un slot vide se lit comme une liste vide.
    #[must_use]
    pub fn list(&self, name: &str) -> Option<&[ListItem]> {
        match self.get(name)? {
            ParamValue::List(l) => Some(l),
            ParamValue::Rich(p) if p.is_empty() => Some(&[]),
            ParamValue::Slot(s) if s.is_empty() => Some(&[]),
            _ => None,
        }
    }

    /// Toutes les listes, avec leur nom.
    pub fn lists(&self) -> impl Iterator<Item = (&str, &[ListItem])> {
        self.0.iter().filter_map(|(k, v)| match v {
            ParamValue::List(l) => Some((k.as_str(), l.as_slice())),
            _ => None,
        })
    }

    /// Raw JSON value of the parameter `name` (grid, host, unknown type).
    #[must_use]
    pub fn value(&self, name: &str) -> Option<&Value> {
        match self.get(name)? {
            ParamValue::Value(v) => Some(v),
            _ => None,
        }
    }

    /// Tous les slots, avec leur nom.
    pub fn slots(&self) -> impl Iterator<Item = (&str, &[Element])> {
        self.0.iter().filter_map(|(k, v)| match v {
            ParamValue::Slot(s) => Some((k.as_str(), s.as_slice())),
            _ => None,
        })
    }

    /// All slots, with their name, mutable.
    pub fn slots_mut(&mut self) -> impl Iterator<Item = (&str, &mut Vec<Element>)> {
        self.0.iter_mut().filter_map(|(k, v)| match v {
            ParamValue::Slot(s) => Some((k.as_str(), s)),
            _ => None,
        })
    }

    /// Tous les textes riches, avec leur nom.
    pub fn riches(&self) -> impl Iterator<Item = (&str, &[Paragraph])> {
        self.0.iter().filter_map(|(k, v)| match v {
            ParamValue::Rich(p) => Some((k.as_str(), p.as_slice())),
            _ => None,
        })
    }

    /// All parameters, by ascending name.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &ParamValue)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// All parameters, by ascending name, mutable.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&str, &mut ParamValue)> {
        self.0.iter_mut().map(|(k, v)| (k.as_str(), v))
    }
}

/// JSON schema: an object with free values.
///
/// The shape of each value depends on the widget; the registry checks it (see
/// [`validate`](crate::validate)).
impl schemars::JsonSchema for Params {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Params".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "object", "additionalProperties": true })
    }
}

#[cfg(test)]
#[path = "params_tests.rs"]
mod params_tests;
