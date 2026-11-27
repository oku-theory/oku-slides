//! The spreadsheet model both readers fill (`xlsx`, `ods`): rows and columns bounded at parse
//! time, one text budget for the whole document, the rest counted, and the Markdown of the
//! sheets.

use oku_slides_core::knowledge::{Cut, CutKind, ExtractLimits};
use oku_slides_core::num::count_u64;

use crate::markdown::{Bound, cut_line, escape_cell, table};

/// Budget bytes charged per stored cell, about what its table line spends around the text
/// (`| ` and ` `), so sparse rows cannot store far more than the budget says.
const SLOT: usize = 4;

/// One row being read: the cells within the column cap, and how far its content reaches.
#[derive(Debug, Default)]
pub(crate) struct Row {
    cells: Vec<String>,
    /// One past the last non-empty column, columns past the cap included.
    width: u64,
    /// Bytes of the stored cells.
    bytes: usize,
    /// Bytes of cells left out because the row was past the text budget.
    dropped: u64,
}

impl Row {
    /// Budget bytes the row costs once stored: its text plus [`SLOT`] per cell, empty ones
    /// included.
    fn cost(&self) -> usize {
        self.bytes + SLOT * self.cells.len()
    }

    /// Sets `text` in `repeat` columns from `col`: only the columns below the column cap are
    /// stored, and only while the row stays within the room `grid` has left; only what is
    /// stored is copied.
    pub(crate) fn set(&mut self, col: u64, repeat: u64, text: &str, grid: &Grid<'_>) {
        if text.is_empty() {
            return;
        }
        let end = col.saturating_add(repeat);
        self.width = self.width.max(end);
        let cap = count_u64(grid.limits.sheet_columns);
        let room = grid.room();
        for c in col..end.min(cap) {
            let Ok(c) = usize::try_from(c) else { break };
            if self.bytes + text.len() + SLOT * self.cells.len().max(c + 1) > room {
                self.dropped = self.dropped.saturating_add(count_u64(text.len()));
                continue;
            }
            if self.cells.len() <= c {
                self.cells.resize(c + 1, String::new());
            }
            self.bytes += text.len();
            text.clone_into(&mut self.cells[c]);
        }
    }
}

/// One sheet being read, bounded by [`ExtractLimits`] and the text its document has left.
#[derive(Debug)]
pub(crate) struct Grid<'l> {
    limits: &'l ExtractLimits,
    rows: Vec<Vec<String>>,
    /// Text bytes this sheet may still store.
    budget: usize,
    /// Budget bytes spent by the stored rows ([`Row::cost`]).
    bytes: usize,
    extra_rows: u64,
    extra_cols: u64,
    /// Text bytes left out for the budget.
    dropped: u64,
}

impl Grid<'_> {
    /// Text bytes the sheet may still store.
    pub(crate) fn room(&self) -> usize {
        self.budget - self.bytes
    }

    /// Adds `row` `times` times: a row with nothing to store is counted (columns past the cap,
    /// text past the budget) but not stored; rows past `sheet_rows` are counted as rows, rows
    /// past the text budget as text.
    pub(crate) fn push(&mut self, row: &Row, times: u64) {
        if row.width == 0 {
            return;
        }
        self.extra_cols = self.extra_cols.max(row.width.saturating_sub(count_u64(self.limits.sheet_columns)));
        if row.cells.is_empty() {
            self.dropped = self.dropped.saturating_add(row.dropped.saturating_mul(times));
            return;
        }
        let mut stored = 0;
        while stored < times && self.rows.len() < self.limits.sheet_rows && row.cost() <= self.room() {
            self.rows.push(row.cells.clone());
            self.bytes += row.cost();
            stored += 1;
        }
        self.dropped = self.dropped.saturating_add(row.dropped.saturating_mul(stored));
        let rest = times - stored;
        if self.rows.len() < self.limits.sheet_rows {
            let lost = count_u64(row.bytes).saturating_add(row.dropped);
            self.dropped = self.dropped.saturating_add(lost.saturating_mul(rest));
        } else {
            self.extra_rows = self.extra_rows.saturating_add(rest);
        }
    }
}

/// A sheet left out, and why (`hidden`…).
#[derive(Debug)]
pub(crate) struct Skipped {
    pub(crate) name: String,
    pub(crate) why: &'static str,
}

/// The sheets of one document, sharing one text budget of `max_text_bytes`.
#[derive(Debug)]
pub(crate) struct Book<'l> {
    limits: &'l ExtractLimits,
    sheets: Vec<(String, Grid<'l>)>,
    skipped: Vec<Skipped>,
    spent: usize,
    /// Set once a sheet left text out for the budget: the next sheets are not read.
    exhausted: bool,
    /// Sheets not read because the budget was spent.
    unread: u64,
}

impl<'l> Book<'l> {
    /// A document with no sheet yet.
    pub(crate) fn new(limits: &'l ExtractLimits) -> Self {
        Self { limits, sheets: Vec::new(), skipped: Vec::new(), spent: 0, exhausted: false, unread: 0 }
    }

    /// States a sheet left out for `why`.
    pub(crate) fn skip(&mut self, name: String, why: &'static str) {
        self.skipped.push(Skipped { name, why });
    }

    /// An empty grid for the next sheet, or `None` (the sheet counted as cut) once the text
    /// budget is spent.
    pub(crate) fn grid(&mut self) -> Option<Grid<'l>> {
        let budget = self.limits.max_text_bytes.saturating_sub(self.spent);
        if self.exhausted || budget == 0 {
            self.unread = self.unread.saturating_add(1);
            return None;
        }
        Some(Grid { limits: self.limits, rows: Vec::new(), budget, bytes: 0, extra_rows: 0, extra_cols: 0, dropped: 0 })
    }

    /// Adds a sheet read into a grid from [`Book::grid`].
    pub(crate) fn add(&mut self, name: String, grid: Grid<'l>) {
        self.spent += grid.bytes;
        self.exhausted |= grid.dropped > 0;
        self.sheets.push((name, grid));
    }

    /// The Markdown of the sheets, one `## <name>` section each with its table and its cuts,
    /// then the sheets left out; and the cuts summed over the document.
    pub(crate) fn render(&self) -> (String, Vec<Cut>) {
        let mut out = String::new();
        let mut cuts = Vec::new();
        for (name, grid) in &self.sheets {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str("## ");
            out.push_str(&heading(name));
            out.push_str("\n\n");
            if grid.rows.is_empty() {
                out.push_str("*(empty sheet)*\n");
            } else {
                out.push_str(&table(&grid.rows));
            }
            let sheet_cuts = [
                (CutKind::Rows, grid.extra_rows, self.limits.sheet_rows),
                (CutKind::Columns, grid.extra_cols, self.limits.sheet_columns),
                (CutKind::Text, grid.dropped, self.limits.max_text_bytes),
            ];
            let mut first = true;
            for (what, count, limit) in sheet_cuts {
                if count == 0 {
                    continue;
                }
                if std::mem::take(&mut first) {
                    out.push('\n');
                }
                let cut = Cut { what, count };
                out.push_str(&cut_line(cut, Bound::Limit(limit)));
                add_cut(&mut cuts, cut);
            }
        }
        if self.unread > 0 || !self.skipped.is_empty() {
            out.push('\n');
        }
        if self.unread > 0 {
            let cut = Cut { what: CutKind::Sheets, count: self.unread };
            out.push_str(&cut_line(cut, Bound::Budget));
            add_cut(&mut cuts, cut);
        }
        for s in &self.skipped {
            out.push_str("… sheet \"");
            out.push_str(&escape_cell(&s.name));
            out.push_str("\" not imported (");
            out.push_str(s.why);
            out.push_str(")\n");
        }
        (out, cuts)
    }
}

/// A sheet name as heading text: escaped on one line, every `#` escaped so that none closes
/// the heading.
pub(crate) fn heading(name: &str) -> String {
    escape_cell(name).replace('#', "\\#")
}

/// Adds `cut` to `cuts`, summed with a cut of the same kind.
pub(crate) fn add_cut(cuts: &mut Vec<Cut>, cut: Cut) {
    match cuts.iter_mut().find(|c| c.what == cut.what) {
        Some(c) => c.count = c.count.saturating_add(cut.count),
        None => cuts.push(cut),
    }
}

#[cfg(test)]
#[path = "sheet_tests.rs"]
mod sheet_tests;
