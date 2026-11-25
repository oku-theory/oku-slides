//! Errors of the data model: refused edits and invalid input.
//!
//! Messages name data, columns and rows, never quote a cell: cells are untrusted text.

use std::fmt;

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, Snafu};

use crate::ColumnType;
use crate::text::cut_chars;

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

impl Localize for DataError {
    /// What to fix, naming the data, column or row; never a cell's value.
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            DataInner::CellType { column, ty, .. } => {
                l.tr_with("data-cell-type", &[("column", quoted(column).into()), ("type", ty.as_str().into())])
            }
            DataInner::ColumnType { column, ty, rows, more, .. } => l.tr_with(
                "data-column-type",
                &[
                    ("column", quoted(column).into()),
                    ("type", ty.as_str().into()),
                    ("rows", rows.iter().map(|r| quoted(r)).collect::<Vec<_>>().join(", ").into()),
                    ("more", (*more).into()),
                ],
            ),
            DataInner::NoRow { row, .. } => l.tr_with("data-no-row", &[("row", quoted(row).into())]),
            DataInner::NoColumn { column, .. } => l.tr_with("data-no-column", &[("column", quoted(column).into())]),
            DataInner::IdTaken { id, .. } => l.tr_with("data-id-taken", &[("id", quoted(id).into())]),
            DataInner::IdInvalid { id, .. } => l.tr_with("data-id-invalid", &[("id", quoted(id).into())]),
            DataInner::IdMissing { .. } => l.tr("data-id-missing"),
            DataInner::TooLarge { what, max, .. } => {
                l.tr_with("data-too-large", &[("what", what.as_str().into()), ("max", (*max).into())])
            }
            DataInner::InvalidName { name, .. } => l.tr_with("data-invalid-name", &[("name", quoted(name).into())]),
            DataInner::NonFinite { count, .. } => l.tr_with("data-non-finite", &[("count", (*count).into())]),
            DataInner::ColumnsInvalid { .. } => l.tr("data-columns-invalid"),
            DataInner::RowWidth { max, .. } => l.tr_with("data-row-width", &[("max", (*max).into())]),
            DataInner::NeedsBase { .. } => l.tr("data-needs-base"),
            DataInner::BaseRowCells { .. } => l.tr("data-base-row-cells"),
            // The reason names the file's flaw for the logs; the user only learns the base is unusable.
            DataInner::BaseInvalid { .. } => l.tr("data-base-invalid"),
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;
