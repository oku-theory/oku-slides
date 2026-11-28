//! Refusals of the data ops: a data, a column or a view the op cannot use.

use std::fmt;

use oku_slides_data::quoted;
use oku_slides_i18n::{FluentValue, Localizer};

/// Why a data op, or an option edit naming a data, is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DataFault {
    /// No data with this id in the deck.
    Missing { data: String },
    /// Another data has this name, ignoring case.
    NameTaken { name: String },
    /// Views project the column; `views` names their elements.
    ColumnInUse { column: String, views: Vec<String> },
    /// The new data of a view lacks a column the view reads.
    ColumnMissing { column: String, data: String },
    /// The param is a list, not a chart or table view.
    NotView { param: String },
    /// The data was cut when the deck was read: only a `replace` restores it.
    Truncated { data: String },
    /// An option edit would point an existing view at another data.
    Rebind { subject: String },
    /// `base` or `overlay` given by an external caller.
    Internal,
    /// An external batch carries more than `max` cells.
    BatchTooLarge { count: usize, max: usize },
    /// Chart or table options carry their data inline; only `prepare_ops` lifts them.
    Inline { subject: String },
    /// Views show the data or queries read it; `views` names the first views, `queries` the
    /// first reading data, `more` counts the rest of both.
    InUse { views: Vec<String>, queries: Vec<String>, more: usize },
    /// The data is derived from a query: its rows are a result, never edited by hand.
    Derived { data: String },
    /// The query of the data named `data` would read it back, through itself or other queries.
    Cycle { data: String },
    /// A query reads `name`, which no data of the deck has.
    UnknownInput { name: String },
    /// `set_query` on a data entered by hand.
    NotQuery { data: String },
    /// `add_data` without exactly one of `table` and `sql`.
    Source,
    /// The query data `reader` gives its own relation (a `WITH` table, an alias) the new name
    /// `name` of a data it reads: rewritten, it would read that relation.
    RenameCaptured { reader: String, name: String },
    /// Another data has the name `name` too, and queries read it: which one they read is unclear.
    NameAmbiguous { name: String },
}

/// Kind of a [`DataFault`], for [`crate::OpErrorKind::DataRefused`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataFaultKind {
    /// No data with this id.
    Missing,
    /// Data name already taken.
    NameTaken,
    /// Column projected by a view.
    ColumnInUse,
    /// Column a view reads absent from the new data.
    ColumnMissing,
    /// Param that is not a chart or table view.
    NotView,
    /// Data cut on reading.
    Truncated,
    /// View pointed at another data by an option edit.
    Rebind,
    /// Internal fields given by an external caller.
    Internal,
    /// External batch over the cell budget.
    BatchTooLarge,
    /// Inline data given to an apply without `prepare_ops`.
    Inline,
    /// Data still shown by views.
    InUse,
    /// Manual edit of a data derived from a query.
    Derived,
    /// Query reading its own data back.
    Cycle,
    /// Query reading a name no data has.
    UnknownInput,
    /// Query set on a data entered by hand.
    NotQuery,
    /// Data given both or neither of a table and a query.
    Source,
    /// Rename to a name a reading query gives its own relation.
    RenameCaptured,
    /// Rename of a name another data has too, read by queries.
    NameAmbiguous,
}

impl DataFault {
    pub(crate) fn kind(&self) -> DataFaultKind {
        match self {
            DataFault::Missing { .. } => DataFaultKind::Missing,
            DataFault::NameTaken { .. } => DataFaultKind::NameTaken,
            DataFault::ColumnInUse { .. } => DataFaultKind::ColumnInUse,
            DataFault::ColumnMissing { .. } => DataFaultKind::ColumnMissing,
            DataFault::NotView { .. } => DataFaultKind::NotView,
            DataFault::Truncated { .. } => DataFaultKind::Truncated,
            DataFault::Rebind { .. } => DataFaultKind::Rebind,
            DataFault::Internal => DataFaultKind::Internal,
            DataFault::BatchTooLarge { .. } => DataFaultKind::BatchTooLarge,
            DataFault::Inline { .. } => DataFaultKind::Inline,
            DataFault::InUse { .. } => DataFaultKind::InUse,
            DataFault::Derived { .. } => DataFaultKind::Derived,
            DataFault::Cycle { .. } => DataFaultKind::Cycle,
            DataFault::UnknownInput { .. } => DataFaultKind::UnknownInput,
            DataFault::NotQuery { .. } => DataFaultKind::NotQuery,
            DataFault::Source => DataFaultKind::Source,
            DataFault::RenameCaptured { .. } => DataFaultKind::RenameCaptured,
            DataFault::NameAmbiguous { .. } => DataFaultKind::NameAmbiguous,
        }
    }

    /// What the fault names: a data, name, column, param or element.
    pub(crate) fn subject(&self) -> Option<&str> {
        match self {
            DataFault::Missing { data }
            | DataFault::Truncated { data }
            | DataFault::Derived { data }
            | DataFault::Cycle { data }
            | DataFault::NotQuery { data } => Some(data),
            DataFault::NameTaken { name } | DataFault::UnknownInput { name } | DataFault::NameAmbiguous { name } => {
                Some(name)
            }
            DataFault::RenameCaptured { reader, .. } => Some(reader),
            DataFault::ColumnInUse { column, .. } | DataFault::ColumnMissing { column, .. } => Some(column),
            DataFault::NotView { param } => Some(param),
            DataFault::Rebind { subject } | DataFault::Inline { subject } => Some(subject),
            DataFault::InUse { views, queries, .. } => views.first().or(queries.first()).map(String::as_str),
            DataFault::Internal | DataFault::BatchTooLarge { .. } | DataFault::Source => None,
        }
    }

    /// The localized message: names cut to 80 characters, never a cell.
    pub(crate) fn localize(&self, l: &Localizer) -> String {
        let q = |s: &str| FluentValue::from(quoted(s));
        match self {
            DataFault::Missing { data } => l.tr_with("op-data-missing", &[("data", q(data))]),
            DataFault::NameTaken { name } => l.tr_with("op-data-name-taken", &[("name", q(name))]),
            DataFault::ColumnInUse { column, views } => {
                l.tr_with("op-data-column-in-use", &[("column", q(column)), ("views", q(&views.join(", ")))])
            }
            DataFault::ColumnMissing { column, data } => {
                l.tr_with("op-data-column-missing", &[("column", q(column)), ("data", q(data))])
            }
            DataFault::NotView { param } => l.tr_with("op-data-not-view", &[("param", q(param))]),
            DataFault::Truncated { data } => l.tr_with("op-data-truncated", &[("data", q(data))]),
            DataFault::Derived { data } => l.tr_with("op-data-derived", &[("data", q(data))]),
            DataFault::Cycle { data } => l.tr_with("op-data-cycle", &[("data", q(data))]),
            DataFault::UnknownInput { name } => l.tr_with("op-data-unknown-input", &[("name", q(name))]),
            DataFault::NotQuery { data } => l.tr_with("op-data-not-query", &[("data", q(data))]),
            DataFault::Source => l.tr("op-data-source"),
            DataFault::RenameCaptured { reader, name } => {
                l.tr_with("op-data-rename-captured", &[("reader", q(reader)), ("name", q(name))])
            }
            DataFault::NameAmbiguous { name } => l.tr_with("op-data-name-ambiguous", &[("name", q(name))]),
            DataFault::Rebind { subject } => l.tr_with("op-data-rebind", &[("subject", q(subject))]),
            DataFault::Internal => l.tr("op-data-internal"),
            DataFault::BatchTooLarge { count, max } => {
                l.tr_with("op-data-batch-too-large", &[("count", (*count).into()), ("max", (*max).into())])
            }
            DataFault::Inline { subject } => l.tr_with("op-data-inline", &[("subject", q(subject))]),
            DataFault::InUse { views, queries, more } if queries.is_empty() => {
                l.tr_with("op-data-in-use", &[("views", q(&views.join(", "))), ("more", (*more).into())])
            }
            DataFault::InUse { views, queries, more } => l.tr_with(
                "op-data-read-by-queries",
                &[
                    ("queries", q(&queries.join(", "))),
                    ("shown", views.len().into()),
                    ("views", q(&views.join(", "))),
                    ("more", (*more).into()),
                ],
            ),
        }
    }
}

impl fmt::Display for DataFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataFault::Missing { data } => write!(f, "no data {data}"),
            DataFault::NameTaken { name } => write!(f, "a data is already named {name}"),
            DataFault::ColumnInUse { column, views } => write!(f, "column {column} is used by {}", views.join(", ")),
            DataFault::ColumnMissing { column, data } => write!(f, "data {data} has no column {column}"),
            DataFault::NotView { param } => write!(f, "{param} is not a chart or table"),
            DataFault::Truncated { data } => write!(f, "data {data} was truncated: replace it"),
            DataFault::Derived { data } => write!(f, "data {data} is derived from a query: edit its query"),
            DataFault::Cycle { data } => write!(f, "the query of {data} would read it back"),
            DataFault::UnknownInput { name } => write!(f, "the query reads {name}, which is no data"),
            DataFault::NotQuery { data } => write!(f, "data {data} is not a query"),
            DataFault::Source => write!(f, "give exactly one of table and sql"),
            DataFault::RenameCaptured { reader, name } => {
                write!(f, "the query of {reader} already names one of its tables {name}")
            }
            DataFault::NameAmbiguous { name } => write!(f, "several data are named {name} and queries read it"),
            DataFault::Rebind { subject } => write!(f, "{subject} already reads another data: use bind_data"),
            DataFault::Internal => write!(f, "base and overlay are internal"),
            DataFault::BatchTooLarge { count, max } => write!(f, "the batch carries {count} cells, over {max}"),
            DataFault::Inline { subject } => write!(f, "{subject} carries inline data: prepare the batch"),
            DataFault::InUse { views, queries, more } => {
                let shown = (!views.is_empty()).then(|| format!("shown by {}", views.join(", ")));
                let read = (!queries.is_empty()).then(|| format!("read by {}", queries.join(", ")));
                let uses: Vec<String> = shown.into_iter().chain(read).collect();
                write!(f, "data is {}", uses.join(", "))?;
                if *more > 0 { write!(f, " and {more} more") } else { Ok(()) }
            }
        }
    }
}
