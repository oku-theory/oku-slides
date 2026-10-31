//! Cells of a usage report row, shared by the usage box and the CLI.
//!
//! Keys come from an untrusted ledger: every cell drops what [`shown_text`] refuses, so a terminal
//! shows it as text. Cells are plain text; the box escapes them for Markdown itself.

use oku_slides_core::{By, CostTotal, UsageRow, shown_text};
use oku_slides_i18n::Localizer;

/// Columns of a usage table: key, seven counts, then the cost.
pub const USAGE_COLUMNS: usize = 9;

/// A cell with nothing to show; keeps the columns aligned.
const EMPTY_CELL: &str = "–";

/// The localized headers of a report grouped by `by`.
#[must_use]
pub fn usage_headers(by: By, l: &Localizer) -> [String; USAGE_COLUMNS] {
    let key = match by {
        By::Day => "usage-col-day",
        By::Month => "usage-col-month",
        By::Model => "usage-col-model",
        By::Agent => "usage-col-agent",
        By::Tab => "usage-col-tab",
        By::Deck => "usage-col-deck",
    };
    [
        key,
        "usage-col-turns",
        "usage-col-input",
        "usage-col-output",
        "usage-col-thought",
        "usage-col-cache-read",
        "usage-col-cache-write",
        "usage-col-total",
        "usage-col-cost",
    ]
    .map(|id| l.tr(id))
}

/// One report row as plain text; a row without key shows `usage-unknown`.
#[must_use]
pub fn usage_cells(row: &UsageRow, l: &Localizer) -> [String; USAGE_COLUMNS] {
    let key = row.key.as_deref().map_or_else(|| l.tr("usage-unknown"), shown_text);
    cells(key, row, l)
}

/// The totals row as plain text, its key cell `usage-row-total`.
#[must_use]
pub fn usage_total_cells(totals: &UsageRow, l: &Localizer) -> [String; USAGE_COLUMNS] {
    cells(l.tr("usage-row-total"), totals, l)
}

/// One currency's cost, marked partial when some turns sent none.
pub(crate) fn usage_cost(c: &CostTotal, l: &Localizer) -> String {
    let id = if c.partial { "usage-badge-cost-partial" } else { "usage-badge-cost" };
    let amount = format!("{:.4}", c.amount);
    l.tr_with(id, &[("amount", amount.into()), ("currency", shown_text(&c.currency).into())])
}

fn cells(key: String, r: &UsageRow, l: &Localizer) -> [String; USAGE_COLUMNS] {
    let costs: Vec<String> = r.costs.iter().map(|c| usage_cost(c, l)).collect();
    let cost = if costs.is_empty() { EMPTY_CELL.to_owned() } else { costs.join(", ") };
    let key = if key.is_empty() { EMPTY_CELL.to_owned() } else { key };
    [
        key,
        r.turns.to_string(),
        r.input.to_string(),
        r.output.to_string(),
        r.thought.to_string(),
        r.cache_read.to_string(),
        r.cache_write.to_string(),
        r.total.to_string(),
        cost,
    ]
}

#[cfg(test)]
#[path = "usage_table_tests.rs"]
pub(crate) mod usage_table_tests;
