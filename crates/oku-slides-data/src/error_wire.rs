//! [`DataError`] across the render worker's pipe: nature and arguments pass, backtraces stay.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::ColumnType;
use crate::error::{Bound, DataError, DataInner};

/// The serialized form of a [`DataError`], one variant per inner error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "error", rename_all = "kebab-case")]
enum Wire {
    CellType { column: String, ty: ColumnType },
    ColumnType { column: String, ty: ColumnType, rows: Vec<String>, more: usize },
    NoRow { row: String },
    NoColumn { column: String },
    IdTaken { id: String },
    IdInvalid { id: String },
    IdMissing,
    TooLarge { what: Bound, max: usize },
    InvalidName { name: String },
    NonFinite { count: usize },
    ColumnsInvalid,
    RowWidth { max: usize },
    NeedsBase,
    BaseRowCells,
    BaseInvalid { reason: String },
}

impl From<&DataError> for Wire {
    fn from(e: &DataError) -> Self {
        match &e.0 {
            DataInner::CellType { column, ty, .. } => Self::CellType { column: column.clone(), ty: *ty },
            DataInner::ColumnType { column, ty, rows, more, .. } => {
                Self::ColumnType { column: column.clone(), ty: *ty, rows: rows.clone(), more: *more }
            }
            DataInner::NoRow { row, .. } => Self::NoRow { row: row.clone() },
            DataInner::NoColumn { column, .. } => Self::NoColumn { column: column.clone() },
            DataInner::IdTaken { id, .. } => Self::IdTaken { id: id.clone() },
            DataInner::IdInvalid { id, .. } => Self::IdInvalid { id: id.clone() },
            DataInner::IdMissing { .. } => Self::IdMissing,
            DataInner::TooLarge { what, max, .. } => Self::TooLarge { what: *what, max: *max },
            DataInner::InvalidName { name, .. } => Self::InvalidName { name: name.clone() },
            DataInner::NonFinite { count, .. } => Self::NonFinite { count: *count },
            DataInner::ColumnsInvalid { .. } => Self::ColumnsInvalid,
            DataInner::RowWidth { max, .. } => Self::RowWidth { max: *max },
            DataInner::NeedsBase { .. } => Self::NeedsBase,
            DataInner::BaseRowCells { .. } => Self::BaseRowCells,
            DataInner::BaseInvalid { reason, .. } => Self::BaseInvalid { reason: reason.clone() },
        }
    }
}

impl From<Wire> for DataError {
    fn from(w: Wire) -> Self {
        use crate::error::{
            BaseInvalidSnafu, BaseRowCellsSnafu, CellTypeSnafu, ColumnTypeSnafu, ColumnsInvalidSnafu, IdInvalidSnafu,
            IdMissingSnafu, IdTakenSnafu, InvalidNameSnafu, NeedsBaseSnafu, NoColumnSnafu, NoRowSnafu, NonFiniteSnafu,
            RowWidthSnafu, TooLargeSnafu,
        };
        let inner = match w {
            Wire::CellType { column, ty } => CellTypeSnafu { column, ty }.build(),
            Wire::ColumnType { column, ty, rows, more } => ColumnTypeSnafu { column, ty, rows, more }.build(),
            Wire::NoRow { row } => NoRowSnafu { row }.build(),
            Wire::NoColumn { column } => NoColumnSnafu { column }.build(),
            Wire::IdTaken { id } => IdTakenSnafu { id }.build(),
            Wire::IdInvalid { id } => IdInvalidSnafu { id }.build(),
            Wire::IdMissing => IdMissingSnafu.build(),
            Wire::TooLarge { what, max } => TooLargeSnafu { what, max }.build(),
            Wire::InvalidName { name } => InvalidNameSnafu { name }.build(),
            Wire::NonFinite { count } => NonFiniteSnafu { count }.build(),
            Wire::ColumnsInvalid => ColumnsInvalidSnafu.build(),
            Wire::RowWidth { max } => RowWidthSnafu { max }.build(),
            Wire::NeedsBase => NeedsBaseSnafu.build(),
            Wire::BaseRowCells => BaseRowCellsSnafu.build(),
            Wire::BaseInvalid { reason } => BaseInvalidSnafu { reason }.build(),
        };
        DataError(inner)
    }
}

impl Serialize for DataError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        Wire::from(self).serialize(s)
    }
}

impl<'de> Deserialize<'de> for DataError {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Wire::deserialize(d).map(Self::from)
    }
}

#[cfg(test)]
#[path = "error_wire_tests.rs"]
mod error_wire_tests;
