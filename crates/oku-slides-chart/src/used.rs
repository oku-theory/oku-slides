//! The columns a chart reads from its dataset, for views that read a projection.

use crate::model::is_horizontal;
use crate::option::{ChartOption, ColumnRef, Encode, SeriesType};

impl ChartOption {
    /// How many leading columns of a table headed `headers` the chart reads.
    ///
    /// The category column, each series' value column (and a waterfall's totals column), or
    /// every column for a radar. Series read columns by position: a prefix keeps each index.
    /// A series with inline data reads none; a reference to no column is left to the render,
    /// which reports it.
    #[must_use]
    pub fn used_columns(&self, headers: &[&str]) -> usize {
        let width = headers.len();
        let Some(first) = self.series.first() else { return 0 };
        let at = |r: Option<&ColumnRef>, default: usize| match r {
            None => Some(default),
            Some(ColumnRef::Index(i)) => Some(*i),
            Some(ColumnRef::Name(n)) => headers.iter().position(|h| h == n),
        };
        let enc = |s: &crate::option::Series| s.encode.clone().unwrap_or_else(empty_encode);
        let mut used: Vec<usize> = Vec::new();
        match first.kind {
            SeriesType::Radar => return width,
            SeriesType::Pie => {
                for s in self.series.iter().filter(|s| s.data.is_none()) {
                    let e = enc(s);
                    used.extend(at(e.item_name.as_ref(), 0));
                    used.extend(at(e.value.as_ref(), 1));
                }
            }
            SeriesType::Waterfall => {
                let e = enc(first);
                used.extend(at(e.x.as_ref(), 0));
                if first.data.is_none()
                    && let Some(col) = at(e.y.as_ref(), 1)
                {
                    used.push(col);
                    // The totals column follows the values, when the table has it.
                    if col + 1 < width {
                        used.push(col + 1);
                    }
                }
            }
            SeriesType::Scatter => {
                for (i, s) in self.series.iter().enumerate().filter(|(_, s)| s.data.is_none()) {
                    let e = enc(s);
                    used.extend(at(e.x.as_ref(), 0));
                    used.extend(at(e.y.as_ref(), i + 1));
                }
            }
            SeriesType::Bar | SeriesType::Line => {
                let horizontal = is_horizontal(self);
                let pick = |e: &Encode, category: bool| if horizontal == category { e.y.clone() } else { e.x.clone() };
                used.extend(at(pick(&enc(first), true).as_ref(), 0));
                for (i, s) in self.series.iter().enumerate().filter(|(_, s)| s.data.is_none()) {
                    used.extend(at(pick(&enc(s), false).as_ref(), i + 1));
                }
            }
        }
        used.into_iter().map(|c| c + 1).max().unwrap_or(0).min(width)
    }
}

fn empty_encode() -> Encode {
    Encode { x: None, y: None, item_name: None, value: None }
}

#[cfg(test)]
#[path = "used_tests.rs"]
mod used_tests;
