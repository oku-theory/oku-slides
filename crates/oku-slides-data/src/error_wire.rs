//! [`DataError`] across the render worker's pipe: nature and arguments pass, backtraces stay.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{Bound, DataError, DataInner};
use crate::{ColumnType, DataKind};

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
    SourceMismatch { kind: DataKind },
    InlineQuery,
    QueryParse { line: u64, column: u64 },
    QueryConstruct { construct: String },
    QueryTooLong { max: usize },
    QueryTooDeep,
    QueryUnknownFunction { function: String },
    QueryRewrite,
    QueryRun { reason: String },
    QueryTooLarge { what: Bound, max: usize },
    QueryColumnType { column: String },
    QueryDuplicateColumn { column: String },
    QueryFloatOperand { function: String, operand: String },
    QueryTimeout { ms: u64 },
    QueryInputFailed { input: String },
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
            DataInner::SourceMismatch { kind, .. } => Self::SourceMismatch { kind: *kind },
            DataInner::InlineQuery { .. } => Self::InlineQuery,
            DataInner::QueryParse { line, column, .. } => Self::QueryParse { line: *line, column: *column },
            DataInner::QueryConstruct { construct, .. } => Self::QueryConstruct { construct: construct.clone() },
            DataInner::QueryTooLong { max, .. } => Self::QueryTooLong { max: *max },
            DataInner::QueryTooDeep { .. } => Self::QueryTooDeep,
            DataInner::QueryRewrite { .. } => Self::QueryRewrite,
            DataInner::QueryUnknownFunction { function, .. } => {
                Self::QueryUnknownFunction { function: function.clone() }
            }
            DataInner::QueryRun { reason, .. } => Self::QueryRun { reason: reason.clone() },
            DataInner::QueryTooLarge { what, max, .. } => Self::QueryTooLarge { what: *what, max: *max },
            DataInner::QueryColumnType { column, .. } => Self::QueryColumnType { column: column.clone() },
            DataInner::QueryDuplicateColumn { column, .. } => Self::QueryDuplicateColumn { column: column.clone() },
            DataInner::QueryFloatOperand { function, operand, .. } => {
                Self::QueryFloatOperand { function: function.clone(), operand: operand.clone() }
            }
            DataInner::QueryTimeout { ms, .. } => Self::QueryTimeout { ms: *ms },
            DataInner::QueryInputFailed { input, .. } => Self::QueryInputFailed { input: input.clone() },
        }
    }
}

impl From<Wire> for DataError {
    fn from(w: Wire) -> Self {
        use crate::error::{
            BaseInvalidSnafu, BaseRowCellsSnafu, CellTypeSnafu, ColumnTypeSnafu, ColumnsInvalidSnafu, IdInvalidSnafu,
            IdMissingSnafu, IdTakenSnafu, InlineQuerySnafu, InvalidNameSnafu, NeedsBaseSnafu, NoColumnSnafu,
            NoRowSnafu, NonFiniteSnafu, QueryColumnTypeSnafu, QueryConstructSnafu, QueryDuplicateColumnSnafu,
            QueryFloatOperandSnafu, QueryInputFailedSnafu, QueryParseSnafu, QueryRewriteSnafu, QueryRunSnafu,
            QueryTimeoutSnafu, QueryTooDeepSnafu, QueryTooLargeSnafu, QueryTooLongSnafu, QueryUnknownFunctionSnafu,
            RowWidthSnafu, SourceMismatchSnafu, TooLargeSnafu,
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
            Wire::SourceMismatch { kind } => SourceMismatchSnafu { kind }.build(),
            Wire::InlineQuery => InlineQuerySnafu.build(),
            Wire::QueryParse { line, column } => QueryParseSnafu { line, column }.build(),
            Wire::QueryConstruct { construct } => QueryConstructSnafu { construct }.build(),
            Wire::QueryTooLong { max } => QueryTooLongSnafu { max }.build(),
            Wire::QueryTooDeep => QueryTooDeepSnafu.build(),
            Wire::QueryRewrite => QueryRewriteSnafu.build(),
            Wire::QueryUnknownFunction { function } => QueryUnknownFunctionSnafu { function }.build(),
            Wire::QueryRun { reason } => QueryRunSnafu { reason }.build(),
            Wire::QueryTooLarge { what, max } => QueryTooLargeSnafu { what, max }.build(),
            Wire::QueryColumnType { column } => QueryColumnTypeSnafu { column }.build(),
            Wire::QueryDuplicateColumn { column } => QueryDuplicateColumnSnafu { column }.build(),
            Wire::QueryFloatOperand { function, operand } => QueryFloatOperandSnafu { function, operand }.build(),
            Wire::QueryTimeout { ms } => QueryTimeoutSnafu { ms }.build(),
            Wire::QueryInputFailed { input } => QueryInputFailedSnafu { input }.build(),
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
