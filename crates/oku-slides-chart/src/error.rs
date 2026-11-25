//! Chart errors: validation, normalization and rendering.
//!
//! Reasons are meant for the user and the LLM: they say what to fix,
//! not only what failed.

use oku_slides_i18n::{Localize, Localizer, Message};
use snafu::{Backtrace, Snafu};

/// A refused chart or table, or a chart that cannot be drawn.
#[derive(Debug, Snafu)]
pub struct ChartError(pub(crate) ChartInner);

/// Kind of a [`ChartError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChartErrorKind {
    /// Options outside the v1 subset (unknown field, invalid type…).
    InvalidOption,
    /// Valid but inconsistent options.
    Inconsistent,
    /// The patch touches `dataset`: edit the data with `edit_data`.
    PatchTouchesDataset,
    /// charts-rs failed.
    Render,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum ChartInner {
    #[snafu(display("invalid options"))]
    InvalidOption { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("invalid options: {reason}"))]
    InvalidPatch { reason: Message, backtrace: Backtrace },
    #[snafu(display("inconsistent data: {reason}"))]
    Inconsistent { reason: Message, backtrace: Backtrace },
    #[snafu(display("an option patch cannot change `dataset`: use edit_data"))]
    PatchTouchesDataset { backtrace: Backtrace },
    #[snafu(display("chart rendering failed"))]
    Render { source: charts_rs::Error, backtrace: Backtrace },
    #[snafu(display("charts-rs drew {found} bars, {expected} expected"))]
    BarLayout { expected: usize, found: usize, backtrace: Backtrace },
}

impl ChartError {
    /// Kind of the error, to handle it without reading the message.
    pub fn kind(&self) -> ChartErrorKind {
        match self.0 {
            ChartInner::InvalidOption { .. } | ChartInner::InvalidPatch { .. } => ChartErrorKind::InvalidOption,
            ChartInner::Inconsistent { .. } => ChartErrorKind::Inconsistent,
            ChartInner::PatchTouchesDataset { .. } => ChartErrorKind::PatchTouchesDataset,
            ChartInner::Render { .. } | ChartInner::BarLayout { .. } => ChartErrorKind::Render,
        }
    }
}

impl Localize for ChartError {
    /// What to fix, for the user or the LLM (faulty field, why it is inconsistent…).
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            // serde names the faulty field: its message is kept as is.
            ChartInner::InvalidOption { source, .. } => {
                l.tr_with("chart-invalid-option", &[("reason", source.to_string().into())])
            }
            ChartInner::InvalidPatch { reason, .. } | ChartInner::Inconsistent { reason, .. } => reason.localize(l),
            ChartInner::PatchTouchesDataset { .. } => l.tr("chart-patch-dataset"),
            ChartInner::Render { .. } | ChartInner::BarLayout { .. } => l.tr("chart-render"),
        }
    }
}

/// Result of operations on charts and tables.
pub type Result<T, E = ChartError> = std::result::Result<T, E>;
