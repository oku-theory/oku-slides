//! The slot a widget preview fills: the whole grid, half or a third of its columns.

use serde::{Deserialize, Serialize};

/// The share of the grid's columns an element widget's preview spans.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewSpan {
    /// Every column of the grid.
    #[default]
    Full,
    /// Half the columns, rounded up.
    Half,
    /// A third of the columns, rounded up.
    Third,
}

impl PreviewSpan {
    /// The columns spanned on a grid of `grid` columns; at least one.
    #[must_use]
    pub fn columns(self, grid: u32) -> u32 {
        let part = match self {
            Self::Full => 1,
            Self::Half => 2,
            Self::Third => 3,
        };
        grid.div_ceil(part).max(1)
    }
}

#[cfg(test)]
#[path = "preview_span_tests.rs"]
mod preview_span_tests;
