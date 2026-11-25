//! Data components, host only: a chart's or a table's (`ChartData`, `TableData`), the deck's
//! data list (`DataList`) and one data's rows (`DataSheet`).
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

/// The data a view shows: what the pane names, and how many views share it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataRef {
    /// The data's id, which a `bind` edit names.
    pub id: String,
    /// Its name, unique in the deck.
    pub name: String,
    /// Its display label, shown instead of the name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The views that show it, this one included.
    pub users: usize,
}

/// A data the view can switch to: it holds every column the view reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataChoice {
    /// The data's id, which a `bind` edit names.
    pub id: String,
    /// Its name, unique in the deck.
    pub name: String,
    /// Its display label, shown instead of the name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A data of the deck, as the data panel lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataRow {
    /// The data's id, which the panel's actions name; unique in the list.
    pub id: String,
    /// Its name, unique in the deck.
    pub name: String,
    /// Its display label, shown instead of the name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// What makes its rows: `manual` (typed in or imported).
    pub kind: String,
    /// Its number of columns.
    pub columns: u32,
    /// Its number of rows.
    pub rows: u32,
    /// The views that show it, in document order.
    pub users: Vec<DataUser>,
    /// The refusal of the panel's last action on it, localized.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// A view showing a data: its element and how the panel names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataUser {
    /// The slide that holds the view.
    pub slide: String,
    /// The element; the slide id for a slide param.
    pub element: String,
    /// Its localized label.
    pub label: String,
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod tests;
