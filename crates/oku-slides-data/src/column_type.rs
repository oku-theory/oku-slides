//! The declared type of a column, and which cells fit it.

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use time::macros::format_description;

use crate::DataCell;
use crate::limits::MAX_TEXT_BYTES;
use crate::number::parse_number;

/// Largest whole number an `f64` holds exactly: 2^53.
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_992.0;

/// Type of a column: what its cells hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ColumnType {
    /// Text, at most `MAX_TEXT_BYTES` bytes per cell.
    Text,
    /// A finite number.
    Number,
    /// A finite whole number, at most 2^53 in absolute value.
    Integer,
    /// True or false.
    Bool,
    /// A date written `YYYY-MM-DD`, years 0001 to 9999.
    Date,
}

impl ColumnType {
    /// The serialized name (`text`, `number`…), also the `$type` of messages.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ColumnType::Text => "text",
            ColumnType::Number => "number",
            ColumnType::Integer => "integer",
            ColumnType::Bool => "bool",
            ColumnType::Date => "date",
        }
    }

    /// Whether `cell` may be stored in a column of this type; an empty cell fits every type.
    #[must_use]
    pub fn fits(self, cell: &DataCell) -> bool {
        match (self, cell) {
            (_, DataCell::Null) | (ColumnType::Bool, DataCell::Bool(_)) => true,
            (ColumnType::Text, DataCell::Text(s)) => s.len() <= MAX_TEXT_BYTES,
            (ColumnType::Number, DataCell::Number(n)) => n.is_finite(),
            (ColumnType::Integer, DataCell::Number(n)) => is_integer(*n),
            (ColumnType::Date, DataCell::Text(s)) => is_date(s),
            _ => false,
        }
    }

    /// The type of a column holding `cells`, empty cells ignored: bool, number or date when
    /// every other cell is one, else text.
    pub fn infer<'a>(cells: impl IntoIterator<Item = &'a DataCell>) -> ColumnType {
        let mut seen: Option<ColumnType> = None;
        for cell in cells {
            let ty = match cell {
                DataCell::Null => continue,
                DataCell::Bool(_) => ColumnType::Bool,
                DataCell::Number(n) if n.is_finite() => ColumnType::Number,
                DataCell::Text(s) if is_date(s) => ColumnType::Date,
                DataCell::Number(_) | DataCell::Text(_) => return ColumnType::Text,
            };
            match seen {
                Some(previous) if previous != ty => return ColumnType::Text,
                _ => seen = Some(ty),
            }
        }
        seen.unwrap_or(ColumnType::Text)
    }

    /// `cell`, of a column of type `from`, converted to this type without loss; `None` if it
    /// does not convert. An empty cell stays empty.
    pub(crate) fn convert(self, from: ColumnType, cell: &DataCell) -> Option<DataCell> {
        if matches!(cell, DataCell::Null) || self == from {
            return Some(cell.clone());
        }
        let converted = match (from, self, cell) {
            (_, ColumnType::Text, _) => DataCell::Text(cell.as_label()),
            (ColumnType::Text, ColumnType::Number | ColumnType::Integer, DataCell::Text(s)) => {
                DataCell::Number(parse_number(s)?)
            }
            (ColumnType::Text, ColumnType::Bool, DataCell::Text(s)) => match s.as_str() {
                "true" => DataCell::Bool(true),
                "false" => DataCell::Bool(false),
                _ => return None,
            },
            (ColumnType::Text, ColumnType::Date, DataCell::Text(_))
            | (ColumnType::Number, ColumnType::Integer, DataCell::Number(_))
            | (ColumnType::Integer, ColumnType::Number, DataCell::Number(_)) => cell.clone(),
            _ => return None,
        };
        self.fits(&converted).then_some(converted)
    }
}

impl fmt::Display for ColumnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A finite whole number an `f64` holds exactly.
fn is_integer(n: f64) -> bool {
    n.is_finite() && n.fract() == 0.0 && n.abs() <= MAX_SAFE_INTEGER
}

/// A real date written `YYYY-MM-DD`, year 0001 or later.
fn is_date(s: &str) -> bool {
    let format = format_description!("[year]-[month]-[day]");
    s.len() == 10
        && s.as_bytes().first().is_some_and(u8::is_ascii_digit)
        && time::Date::parse(s, &format).is_ok_and(|d| d.year() >= 1)
}

#[cfg(test)]
#[path = "column_type_tests.rs"]
mod column_type_tests;
