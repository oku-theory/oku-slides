//! Columns of a `ListTable` and its nested list, checked when read.

use serde::{Deserialize, Serialize};

use crate::props::ChoiceOption;

/// A column of a [`ComponentKind::ListTable`](crate::ComponentKind::ListTable): the item key, its
/// label and its input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListColumn {
    /// Key of the item field shown in the column.
    pub key: String,
    /// Column header.
    pub label: String,
    /// Column input.
    pub kind: ColumnKind,
    /// Options of a `select` column.
    #[serde(default)]
    pub options: Vec<ChoiceOption>,
    /// Columns, bounds and label of a `list` column (nested list, one level).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<Box<SubList>>,
}

/// The nested list of a `list` column.
///
/// It holds its columns (never `list`: a single level), its bounds
/// (`min` ≤ `max`) and the name of one of its items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, try_from = "SubListRaw")]
pub struct SubList {
    /// The columns of the detail table, with no `list` column.
    pub columns: Vec<SubColumn>,
    /// Minimum number of items.
    pub min: u32,
    /// Maximum number of items, at least `min`; absent: no limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
    /// Name of one item, for the add and remove buttons.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_label: Option<String>,
}

/// A column of a nested list: like [`ListColumn`], without a nested list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubColumn {
    /// Key of the item field shown in the column.
    pub key: String,
    /// Column header.
    pub label: String,
    /// Column input, never `list`.
    pub kind: ColumnKind,
    /// Options of a `select` column.
    #[serde(default)]
    pub options: Vec<ChoiceOption>,
}

/// [`SubList`] as read, before its checks.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SubListRaw {
    columns: Vec<SubColumn>,
    min: u32,
    #[serde(default)]
    max: Option<u32>,
    #[serde(default)]
    item_label: Option<String>,
}

/// Rejection of a [`SubList`]: a `list` column (two levels), or `max` below `min`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubListFault {
    /// A column of the nested list is itself a list.
    NestedList,
    /// `max` is below `min`.
    MaxBelowMin,
}

impl std::fmt::Display for SubListFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SubListFault::NestedList => "a nested list column cannot be a list",
            SubListFault::MaxBelowMin => "nested list max below min",
        })
    }
}

impl TryFrom<SubListRaw> for SubList {
    type Error = SubListFault;

    fn try_from(r: SubListRaw) -> Result<Self, Self::Error> {
        SubList::new(r.columns, r.min, r.max, r.item_label)
    }
}

impl SubList {
    /// The nested list, checked: no `list` column, `max` at least `min`.
    ///
    /// # Errors
    ///
    /// [`SubListFault::NestedList`] if a column is a `list`, [`SubListFault::MaxBelowMin`]
    /// if `max` is below `min`.
    pub fn new(
        columns: Vec<SubColumn>,
        min: u32,
        max: Option<u32>,
        item_label: Option<String>,
    ) -> Result<Self, SubListFault> {
        if columns.iter().any(|c| c.kind == ColumnKind::List) {
            return Err(SubListFault::NestedList);
        }
        if max.is_some_and(|m| m < min) {
            return Err(SubListFault::MaxBelowMin);
        }
        Ok(Self { columns, min, max, item_label })
    }
}

/// Input of a list column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnKind {
    /// Single-line text.
    Text,
    /// Paragraphs, edited in a popover.
    Rich,
    /// Number.
    Number,
    /// Checkbox.
    Flag,
    /// Dropdown list of `options`.
    Select,
    /// Asset identifier, read-only.
    Asset,
    /// Nested list: its items are edited in a detail table (see [`SubList`]).
    List,
}

#[cfg(test)]
#[path = "list_tests.rs"]
mod tests;
