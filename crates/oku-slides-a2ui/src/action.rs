//! Data-model bound values and the actions components send.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A value written as is, or read from the data model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Dynamic<T> {
    /// JSON Pointer path (RFC 6901) into the surface's model.
    Path {
        /// The path read, relative to the surface's model.
        path: String,
    },
    /// The value itself.
    Literal(T),
}

impl<T> Dynamic<T> {
    /// A value read at `path` in the model.
    pub fn path(path: impl Into<String>) -> Self {
        Self::Path { path: path.into() }
    }
}

/// What a component sends when the user acts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    /// The event sent to the host.
    pub event: Event,
}

/// A named event; the UI reads the context paths when sending it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    /// Event name, routed by the host.
    pub name: String,
    /// Attached values, by key; paths are read at send time.
    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub context: BTreeMap<String, Dynamic<Value>>,
}

impl Action {
    /// The `name` event, with its context.
    pub fn event<'a>(name: impl Into<String>, context: impl IntoIterator<Item = (&'a str, Dynamic<Value>)>) -> Self {
        Self {
            event: Event { name: name.into(), context: context.into_iter().map(|(k, v)| (k.to_owned(), v)).collect() },
        }
    }

    /// The `name` event, without context.
    pub fn named(name: impl Into<String>) -> Self {
        Self { event: Event { name: name.into(), context: BTreeMap::new() } }
    }
}
