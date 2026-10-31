//! Data components of a chart or a table (`ChartData`, `TableData`), host only.
//!
//! The session builds them from the deck; the UI writes a change at the model's `edit` path,
//! then sends the action, which the session applies as one undoable op.

use serde::{Deserialize, Serialize};

/// A series that can be colored: its name (its column header) and its own color.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeriesSwatch {
    /// The series name, read in its column header.
    pub name: String,
    /// Its own `#rrggbb` color; absent: the theme palette.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod tests;
