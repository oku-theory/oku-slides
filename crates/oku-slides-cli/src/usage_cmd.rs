//! `oku-slides usage`: the agent usage recorded in decks, summed.
//!
//! Each deck is read without rendering (no pack set, so an empty widget registry and no render
//! worker); only its ledger issues are logged, once. A turn held by several decks (a copy, a merge)
//! counts once, for the first deck given. [`aggregate`] makes the only report; the table's headers
//! and cells are the `show_usage` box's ([`usage_headers`], [`usage_cells`], [`usage_total_cells`]).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::ValueEnum;
use color_eyre::eyre::{Result, eyre};

use oku_slides_core::{
    By, Issue, Range, SourceLimits, UsageLedger, UsageRecord, UsageReport, WidgetRegistry, aggregate, shown_text,
};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::Platform;
use oku_slides_render::Fonts;
use oku_slides_session::{
    EXTENSION, Opened, Storage, USAGE_COLUMNS, read_document, usage_cells, usage_headers, usage_total_cells,
};

use crate::out::Out;

/// What `--by` groups the rows by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum CliBy {
    /// One row per UTC day.
    Day,
    /// One row per UTC month.
    Month,
    /// One row per model.
    Model,
    /// One row per agent.
    Agent,
    /// One row per kind of tab.
    Tab,
    /// One row per deck.
    Deck,
}

impl From<CliBy> for By {
    fn from(by: CliBy) -> Self {
        match by {
            CliBy::Day => By::Day,
            CliBy::Month => By::Month,
            CliBy::Model => By::Model,
            CliBy::Agent => By::Agent,
            CliBy::Tab => By::Tab,
            CliBy::Deck => By::Deck,
        }
    }
}

/// The `usage` command's arguments, as parsed by clap.
#[derive(Debug)]
pub(crate) struct UsageArgs {
    /// The decks to read, in priority order for a turn found in several.
    pub decks: Vec<PathBuf>,
    /// What the rows group by.
    pub by: CliBy,
    /// First UTC day kept, as typed.
    pub since: Option<String>,
    /// Last UTC day kept, as typed.
    pub until: Option<String>,
    /// Print JSON instead of a table.
    pub json: bool,
}

/// Prints the summed ledgers of `args.decks`, as a table or JSON.
pub(crate) fn usage(
    platform: &Platform,
    out: &mut Out<'_>,
    app_fonts: &Fonts,
    limits: SourceLimits,
    args: &UsageArgs,
) -> Result<()> {
    let l = out.localizer();
    let range = Range::parse(args.since.as_deref(), args.until.as_deref()).map_err(|e| eyre!(e.localize(l)))?;
    // No pack set: `registry` returns the empty registry, without probing or the render worker.
    let registry = crate::registry(app_fonts)?;
    let mut decks = Vec::with_capacity(args.decks.len());
    for path in &args.decks {
        let mut opened = read_deck_usage(&platform.storage, l, path, limits, &registry)?;
        crate::log_issues(l, &usage_issues(&opened.issues));
        decks.push((deck_label(path), std::mem::take(&mut opened.sources.usage)));
    }
    let records = once_per_turn(&decks);
    let report = aggregate(records.iter().copied(), args.by.into(), range);
    let text = if args.json { serde_json::to_string_pretty(&report)? } else { format_table(&report, l) };
    out.data(&text)?;
    Ok(())
}

/// Reads the deck at `path`; its error is one filtered line.
///
/// The error's causes embed the path: the whole line goes through [`shown_text`], so a file name
/// with escape or bidi characters cannot reach the terminal raw.
pub(crate) fn read_deck_usage(
    storage: &Storage,
    l: &Localizer,
    path: &Path,
    limits: SourceLimits,
    registry: &Arc<WidgetRegistry>,
) -> Result<Opened> {
    read_document(storage, path, limits, registry).map_err(|e| {
        let context = l.tr_with("cli-reading", &[("path", path.display().to_string().into())]);
        eyre!(shown_text(&format!("{context}: {}", e.one_line())))
    })
}

/// The ledger issues among `issues`: the others (unknown widgets, packs) are noise here.
pub(crate) fn usage_issues(issues: &[Issue]) -> Vec<Issue> {
    issues.iter().filter(|i| i.message.id().starts_with("issue-usage-")).cloned().collect()
}

/// A deck's label: its path without the deck extension.
///
/// That is `.oku-slides.pdf`, or the extension of a JSON deck.
///
/// The path comes from the file system: [`shown_text`] drops what a terminal would interpret.
pub(crate) fn deck_label(path: &Path) -> String {
    let shown = path.display().to_string();
    let label = match shown.strip_suffix(EXTENSION) {
        Some(stem) => stem.to_owned(),
        None => path.with_extension("").display().to_string(),
    };
    shown_text(&label)
}

/// Every record of `decks` with its deck label.
///
/// A turn already seen in an earlier deck is skipped.
pub(crate) fn once_per_turn(decks: &[(String, UsageLedger)]) -> Vec<(&str, &UsageRecord)> {
    let mut seen = BTreeSet::new();
    decks
        .iter()
        .flat_map(|(label, ledger)| ledger.records().map(move |r| (label.as_str(), r)))
        .filter(|(_, r)| seen.insert(r.turn))
        .collect()
}

/// The report as an aligned table: header, one line per row, then the totals.
///
/// The key column is left-aligned, the others right-aligned; columns are two spaces apart and
/// every line has the same width (counted in characters).
pub(crate) fn format_table(report: &UsageReport, l: &Localizer) -> String {
    let mut lines = vec![usage_headers(report.by, l)];
    lines.extend(report.rows.iter().map(|row| usage_cells(row, l)));
    lines.push(usage_total_cells(&report.totals, l));
    let widths: Vec<usize> =
        (0..USAGE_COLUMNS).map(|c| lines.iter().map(|line| line[c].chars().count()).max().unwrap_or(0)).collect();
    lines.iter().map(|line| aligned(line, &widths)).collect::<Vec<_>>().join("\n")
}

fn aligned(line: &[String; USAGE_COLUMNS], widths: &[usize]) -> String {
    let cell = |(i, (text, width)): (usize, (&String, &usize))| {
        if i == 0 { format!("{text:<width$}") } else { format!("{text:>width$}") }
    };
    line.iter().zip(widths).enumerate().map(cell).collect::<Vec<_>>().join("  ")
}

#[cfg(test)]
#[path = "usage_cmd_tests.rs"]
mod usage_cmd_tests;
