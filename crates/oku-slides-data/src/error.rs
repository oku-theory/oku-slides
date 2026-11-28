//! Errors of the data model: refused edits and invalid input.
//!
//! Messages name data, columns and rows, never quote a cell: cells are untrusted text.

use std::fmt;

use oku_slides_i18n::{Localize, Localizer, Message};
use snafu::{Backtrace, Snafu};

use crate::text::cut_chars;
use crate::{ColumnType, DataKind};

/// Characters of a name or id quoted in a message.
const QUOTED_CHARS: usize = 80;

/// A data edit or input refused.
#[derive(Debug, Snafu)]
pub struct DataError(pub(crate) DataInner);

/// Nature of a [`DataError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataErrorKind {
    /// A cell does not fit its column's type.
    CellType,
    /// A column cannot change type without loss.
    ColumnType,
    /// No row with this id.
    NoRow,
    /// No column with this id.
    NoColumn,
    /// The id is already used.
    IdTaken,
    /// The id is not valid.
    IdInvalid,
    /// An input id was left out where one is needed.
    IdMissing,
    /// The edit would pass a bound of the data.
    TooLarge,
    /// The data name is not valid.
    InvalidName,
    /// A number is NaN or infinite.
    NonFinite,
    /// A column list is empty or repeats a column.
    ColumnsInvalid,
    /// A row has more cells than the table has columns.
    RowWidth,
    /// The data has a base, which only the render worker reads.
    NeedsBase,
    /// A table with a base gives cells in its rows (its rows carry ids only).
    BaseRowCells,
    /// A base file is malformed, past a bound, or does not match its recorded columns.
    BaseInvalid,
    /// A data's source does not match its kind: a table without `kind: manual`, or `sql`
    /// without `kind: query`.
    SourceMismatch,
    /// A query data marked inline: inline data is only made from a view's inline table.
    InlineQuery,
    /// A query's SQL does not parse; the message names the line and column, never the text.
    QueryParse,
    /// A query uses a construct data queries do not support; the message names it.
    QueryConstruct,
    /// A query's SQL is longer than [`crate::limits::MAX_SQL_BYTES`].
    QueryTooLong,
    /// A query nests past the parser's recursion limit or [`crate::limits::MAX_QUERY_DEPTH`].
    QueryTooDeep,
    /// A query calls a function data queries do not support; the message names it.
    QueryUnknownFunction,
    /// The rewrites made SQL the whitelist refuses or that reads other data: a bug, never a
    /// user error.
    QueryRewrite,
    /// The engine refused or failed the query; the message carries its reason, never a cell.
    QueryRun,
    /// A query result passes a bound of a data: it is refused, never cut.
    QueryTooLarge,
    /// A query result column has a type no data column holds; the message names it.
    QueryColumnType,
    /// Two query result columns have the same name; the message names it.
    QueryDuplicateColumn,
    /// `%`, `mod`, `//` or `round(x, n)` meets decimal numbers, where the engine would differ
    /// from `DuckDB` silently; the message names the operand.
    QueryFloatOperand,
    /// The query ran past the link's time bound and was stopped.
    QueryTimeout,
    /// A query data the query reads failed; the message names it.
    QueryInputFailed,
}

/// The bound a growing edit would pass (`$what` of `data-too-large`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Bound {
    Rows,
    Columns,
    Text,
    Overlay,
}

impl Bound {
    fn as_str(self) -> &'static str {
        match self {
            Bound::Rows => "rows",
            Bound::Columns => "columns",
            Bound::Text => "text",
            Bound::Overlay => "overlay",
        }
    }
}

impl fmt::Display for Bound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum DataInner {
    #[snafu(display("a value does not fit column {column} ({ty})"))]
    CellType { column: String, ty: ColumnType, backtrace: Backtrace },
    #[snafu(display("column {column} cannot become {ty} without loss (rows {rows:?}, {more} more)"))]
    ColumnType { column: String, ty: ColumnType, rows: Vec<String>, more: usize, backtrace: Backtrace },
    #[snafu(display("row {row} does not exist"))]
    NoRow { row: String, backtrace: Backtrace },
    #[snafu(display("column {column} does not exist"))]
    NoColumn { column: String, backtrace: Backtrace },
    #[snafu(display("id {id} is already used"))]
    IdTaken { id: String, backtrace: Backtrace },
    #[snafu(display("id {id:?} is not valid"))]
    IdInvalid { id: String, backtrace: Backtrace },
    #[snafu(display("a row, a column or the generation has no id"))]
    IdMissing { backtrace: Backtrace },
    #[snafu(display("too many {what}: {max} at most"))]
    TooLarge { what: Bound, max: usize, backtrace: Backtrace },
    #[snafu(display("data name {name:?} is not valid"))]
    InvalidName { name: String, backtrace: Backtrace },
    #[snafu(display("{count} values are not finite numbers"))]
    NonFinite { count: usize, backtrace: Backtrace },
    #[snafu(display("a column list is empty or repeats a column"))]
    ColumnsInvalid { backtrace: Backtrace },
    #[snafu(display("a row has more cells than the {max} columns"))]
    RowWidth { max: usize, backtrace: Backtrace },
    #[snafu(display("the data has a base, read only by the render worker"))]
    NeedsBase { backtrace: Backtrace },
    #[snafu(display("a table with a base gives cells in its rows"))]
    BaseRowCells { backtrace: Backtrace },
    #[snafu(display("the base file is not valid: {reason}"))]
    BaseInvalid { reason: String, backtrace: Backtrace },
    #[snafu(display("a {} data gives the wrong source: a manual data its table, a query data its sql", kind.as_str()))]
    SourceMismatch { kind: DataKind, backtrace: Backtrace },
    #[snafu(display("a query data cannot be inline"))]
    InlineQuery { backtrace: Backtrace },
    #[snafu(display("the query does not parse at line {line}, column {column}"))]
    QueryParse { line: u64, column: u64, backtrace: Backtrace },
    #[snafu(display("the query uses {construct}, which data queries do not support"))]
    QueryConstruct { construct: String, backtrace: Backtrace },
    #[snafu(display("the query is longer than {max} bytes"))]
    QueryTooLong { max: usize, backtrace: Backtrace },
    #[snafu(display("the query is nested too deeply"))]
    QueryTooDeep { backtrace: Backtrace },
    #[snafu(display("the query calls {function}, which data queries do not support"))]
    QueryUnknownFunction { function: String, backtrace: Backtrace },
    #[snafu(display("the query could not be prepared for the engine"))]
    QueryRewrite { backtrace: Backtrace },
    #[snafu(display("the query failed: {reason}"))]
    QueryRun { reason: String, backtrace: Backtrace },
    #[snafu(display("the query result has more {what} than {max}"))]
    QueryTooLarge { what: Bound, max: usize, backtrace: Backtrace },
    #[snafu(display("result column {column} has a type data cannot hold"))]
    QueryColumnType { column: String, backtrace: Backtrace },
    #[snafu(display("two result columns are named {column}"))]
    QueryDuplicateColumn { column: String, backtrace: Backtrace },
    #[snafu(display("{function} is applied to {operand}, which holds decimal numbers"))]
    QueryFloatOperand { function: String, operand: String, backtrace: Backtrace },
    #[snafu(display("the query ran longer than {ms} ms"))]
    QueryTimeout { ms: u64, backtrace: Backtrace },
    #[snafu(display("its input {input} failed"))]
    QueryInputFailed { input: String, backtrace: Backtrace },
}

impl DataError {
    /// Kind of the error, to handle it without reading the message.
    #[must_use]
    pub fn kind(&self) -> DataErrorKind {
        match self.0 {
            DataInner::CellType { .. } => DataErrorKind::CellType,
            DataInner::ColumnType { .. } => DataErrorKind::ColumnType,
            DataInner::NoRow { .. } => DataErrorKind::NoRow,
            DataInner::NoColumn { .. } => DataErrorKind::NoColumn,
            DataInner::IdTaken { .. } => DataErrorKind::IdTaken,
            DataInner::IdInvalid { .. } => DataErrorKind::IdInvalid,
            DataInner::IdMissing { .. } => DataErrorKind::IdMissing,
            DataInner::TooLarge { .. } => DataErrorKind::TooLarge,
            DataInner::InvalidName { .. } => DataErrorKind::InvalidName,
            DataInner::NonFinite { .. } => DataErrorKind::NonFinite,
            DataInner::ColumnsInvalid { .. } => DataErrorKind::ColumnsInvalid,
            DataInner::RowWidth { .. } => DataErrorKind::RowWidth,
            DataInner::NeedsBase { .. } => DataErrorKind::NeedsBase,
            DataInner::BaseRowCells { .. } => DataErrorKind::BaseRowCells,
            DataInner::BaseInvalid { .. } => DataErrorKind::BaseInvalid,
            DataInner::SourceMismatch { .. } => DataErrorKind::SourceMismatch,
            DataInner::InlineQuery { .. } => DataErrorKind::InlineQuery,
            DataInner::QueryParse { .. } => DataErrorKind::QueryParse,
            DataInner::QueryConstruct { .. } => DataErrorKind::QueryConstruct,
            DataInner::QueryTooLong { .. } => DataErrorKind::QueryTooLong,
            DataInner::QueryTooDeep { .. } => DataErrorKind::QueryTooDeep,
            DataInner::QueryUnknownFunction { .. } => DataErrorKind::QueryUnknownFunction,
            DataInner::QueryRewrite { .. } => DataErrorKind::QueryRewrite,
            DataInner::QueryRun { .. } => DataErrorKind::QueryRun,
            DataInner::QueryTooLarge { .. } => DataErrorKind::QueryTooLarge,
            DataInner::QueryColumnType { .. } => DataErrorKind::QueryColumnType,
            DataInner::QueryDuplicateColumn { .. } => DataErrorKind::QueryDuplicateColumn,
            DataInner::QueryFloatOperand { .. } => DataErrorKind::QueryFloatOperand,
            DataInner::QueryTimeout { .. } => DataErrorKind::QueryTimeout,
            DataInner::QueryInputFailed { .. } => DataErrorKind::QueryInputFailed,
        }
    }

    /// Whether a growing edit would pass the data's overlay cap (`data-too-large { overlay }`).
    #[must_use]
    pub fn is_overlay_full(&self) -> bool {
        matches!(self.0, DataInner::TooLarge { what: Bound::Overlay, .. })
    }

    /// A data name refused by [`crate::valid_name`] (`data-invalid-name`).
    #[must_use]
    pub fn invalid_name(name: &str) -> DataError {
        InvalidNameSnafu { name }.build().into()
    }

    /// An op input without the id the host fills in (`data-id-missing`).
    #[must_use]
    pub fn id_missing() -> DataError {
        IdMissingSnafu.build().into()
    }

    /// An id already used by another data (`data-id-taken`).
    #[must_use]
    pub fn id_taken(id: &str) -> DataError {
        IdTakenSnafu { id }.build().into()
    }

    /// A column id the data does not have (`data-no-column`).
    #[must_use]
    pub fn no_column(column: &str) -> DataError {
        NoColumnSnafu { column }.build().into()
    }

    /// A row id the data does not have (`data-no-row`).
    #[must_use]
    pub fn no_row(row: &str) -> DataError {
        NoRowSnafu { row }.build().into()
    }

    /// A base file refused for `reason` (`data-base-invalid`).
    #[must_use]
    pub fn base_invalid(reason: impl Into<String>) -> DataError {
        BaseInvalidSnafu { reason: reason.into() }.build().into()
    }

    /// A query stopped by the link after `ms` milliseconds (`data-query-timeout`).
    #[must_use]
    pub fn query_timeout(ms: u64) -> DataError {
        QueryTimeoutSnafu { ms }.build().into()
    }

    /// A query the engine or its worker failed for `reason` (`data-query-run`): never a cell.
    #[must_use]
    pub fn query_run(reason: impl Into<String>) -> DataError {
        QueryRunSnafu { reason: reason.into() }.build().into()
    }

    /// A query whose input, the query data named `input`, failed (`data-query-input-failed`).
    #[must_use]
    pub fn query_input_failed(input: impl Into<String>) -> DataError {
        QueryInputFailedSnafu { input: input.into() }.build().into()
    }

    /// A column projection empty, repeated or malformed (`data-columns-invalid`).
    #[must_use]
    pub fn columns_invalid() -> DataError {
        ColumnsInvalidSnafu.build().into()
    }
}

/// `text` as quoted in a message: cut to 80 characters.
#[must_use]
pub fn quoted(text: &str) -> String {
    cut_chars(text, QUOTED_CHARS)
}

impl DataError {
    /// The error as a message said later: what to fix, naming the data, column or row; never
    /// a cell's value.
    #[must_use]
    pub fn message(&self) -> Message {
        let m = Message::new;
        match &self.0 {
            DataInner::CellType { column, ty, .. } => {
                m("data-cell-type").arg("column", quoted(column)).arg("type", ty.as_str())
            }
            DataInner::ColumnType { column, ty, rows, more, .. } => m("data-column-type")
                .arg("column", quoted(column))
                .arg("type", ty.as_str())
                .arg("rows", rows.iter().map(|r| quoted(r)).collect::<Vec<_>>().join(", "))
                .arg("more", *more),
            DataInner::NoRow { row, .. } => m("data-no-row").arg("row", quoted(row)),
            DataInner::NoColumn { column, .. } => m("data-no-column").arg("column", quoted(column)),
            DataInner::IdTaken { id, .. } => m("data-id-taken").arg("id", quoted(id)),
            DataInner::IdInvalid { id, .. } => m("data-id-invalid").arg("id", quoted(id)),
            DataInner::IdMissing { .. } => m("data-id-missing"),
            DataInner::TooLarge { what, max, .. } => m("data-too-large").arg("what", what.as_str()).arg("max", *max),
            DataInner::InvalidName { name, .. } => m("data-invalid-name").arg("name", quoted(name)),
            DataInner::NonFinite { count, .. } => m("data-non-finite").arg("count", *count),
            DataInner::ColumnsInvalid { .. } => m("data-columns-invalid"),
            DataInner::RowWidth { max, .. } => m("data-row-width").arg("max", *max),
            DataInner::NeedsBase { .. } => m("data-needs-base"),
            DataInner::BaseRowCells { .. } => m("data-base-row-cells"),
            // The reason names the file's flaw for the logs; the user only learns the base is unusable.
            DataInner::BaseInvalid { .. } => m("data-base-invalid"),
            DataInner::SourceMismatch { .. } => m("data-source-mismatch"),
            DataInner::InlineQuery { .. } => m("data-inline-query"),
            // Line and column as text: a number argument would be grouped (`1,024`) in some locales.
            DataInner::QueryParse { line, column, .. } => {
                m("data-query-parse").arg("line", line.to_string()).arg("column", column.to_string())
            }
            DataInner::QueryConstruct { construct, .. } => {
                m("data-query-construct").arg("construct", quoted(construct))
            }
            DataInner::QueryTooLong { max, .. } => m("data-query-too-long").arg("max", *max),
            DataInner::QueryTooDeep { .. } => m("data-query-too-deep"),
            DataInner::QueryRewrite { .. } => m("data-query-rewrite"),
            DataInner::QueryUnknownFunction { function, .. } => {
                m("data-query-unknown-function").arg("function", quoted(function))
            }
            // The reason is the engine's English text, scrubbed of cell values (`engine::query`).
            DataInner::QueryRun { reason, .. } => m("data-query-run").arg("reason", reason.clone()),
            DataInner::QueryTooLarge { what, max, .. } => {
                m("data-query-too-large").arg("what", what.as_str()).arg("max", *max)
            }
            DataInner::QueryColumnType { column, .. } => m("data-query-column-type").arg("column", quoted(column)),
            DataInner::QueryDuplicateColumn { column, .. } => {
                m("data-query-duplicate-column").arg("column", quoted(column))
            }
            DataInner::QueryFloatOperand { function, operand, .. } => {
                m("data-query-float-operand").arg("function", quoted(function)).arg("operand", quoted(operand))
            }
            DataInner::QueryTimeout { ms, .. } => m("data-query-timeout").arg("seconds", ms.div_ceil(1000)),
            DataInner::QueryInputFailed { input, .. } => m("data-query-input-failed").arg("input", quoted(input)),
        }
    }
}

impl Localize for DataError {
    /// What to fix, naming the data, column or row; never a cell's value.
    fn localize(&self, l: &Localizer) -> String {
        self.message().localize(l)
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;
