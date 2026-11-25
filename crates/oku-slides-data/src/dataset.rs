//! The dataset: a 2D table with stable row and column identifiers.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::number::{format_number, parse_number};

/// Tabular data shared by the series, with stable identifiers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Dataset {
    /// 2D table; row 0 holds the column names.
    pub source: Vec<Vec<DataCell>>,
    /// Stable identifier of each row, header included.
    ///
    /// Do not supply them: the core draws them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<String>,
    /// Stable identifier of each column.
    ///
    /// Do not supply them: the core draws them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<String>,
}

/// Cell of `dataset.source`: number, text, boolean or empty.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "Cell")]
pub enum DataCell {
    /// Number.
    Number(f64),
    /// Text; read as a number if it looks like one (decimal comma accepted).
    Text(String),
    /// Checkbox (the Total column of a waterfall).
    Bool(bool),
    /// Empty cell.
    Null,
}

impl DataCell {
    /// Numeric value: a number, or a text that looks like one.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            DataCell::Number(n) => Some(*n),
            DataCell::Text(s) => parse_number(s),
            DataCell::Bool(_) | DataCell::Null => None,
        }
    }

    /// Case cochée ; toute autre cellule compte comme décochée.
    #[must_use]
    pub fn is_checked(&self) -> bool {
        matches!(self, DataCell::Bool(true))
    }

    /// Displayed text: whole numbers without decimals, empty for an empty cell.
    #[must_use]
    pub fn as_label(&self) -> String {
        match self {
            DataCell::Number(n) => format_number(*n),
            DataCell::Text(s) => s.clone(),
            DataCell::Bool(b) => b.to_string(),
            DataCell::Null => String::new(),
        }
    }
}
