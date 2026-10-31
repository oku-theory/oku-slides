//! One page of a deck's version list for the agent's `history` tool.
//!
//! A long-lived deck holds thousands of versions: the tool returns the latest
//! `count` before `before`, each with its `index` (the rank `seek` takes), and the
//! session-start and merge milestones older than the page, so the agent can jump
//! to them or page further back with `next_before`.

use oku_slides_core::Version;
use serde::Serialize;

/// Versions in a page when the agent names no `count`.
pub(crate) const DEFAULT_COUNT: usize = 20;
/// Versions in a page at most.
pub(crate) const MAX_COUNT: usize = 100;
/// Milestones older than the page listed at most, the most recent ones.
pub(crate) const MAX_MILESTONES: usize = 20;

/// A version and its rank in the whole list.
#[derive(Debug, Serialize)]
pub(crate) struct Ranked<'a> {
    /// Rank of the version (0: the oldest), as `seek` takes it.
    pub index: usize,
    /// The version.
    #[serde(flatten)]
    pub version: &'a Version,
}

/// A page of the version list, oldest to newest.
#[derive(Debug, Serialize)]
pub(crate) struct HistoryPage<'a> {
    /// Number of versions in the whole list.
    pub total: usize,
    /// The page's versions, oldest to newest.
    pub versions: Vec<Ranked<'a>>,
    /// Session starts and merges older than the page, oldest to newest (the most recent ones).
    pub milestones: Vec<Ranked<'a>>,
    /// The `before` of the previous (older) page; absent when this page starts at the oldest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_before: Option<usize>,
}

/// The page of `versions` ending before rank `before` (default: the end), `count` long at most.
///
/// `count` defaults to [`DEFAULT_COUNT`] and is clamped to `1..=MAX_COUNT`; a `before` past the
/// end means the end.
pub(crate) fn history_page(versions: &[Version], before: Option<usize>, count: Option<usize>) -> HistoryPage<'_> {
    let total = versions.len();
    let end = before.unwrap_or(total).min(total);
    let count = count.unwrap_or(DEFAULT_COUNT).clamp(1, MAX_COUNT);
    let start = end.saturating_sub(count);
    let ranked = |(index, version)| Ranked { index, version };
    let page = versions.iter().enumerate().take(end).skip(start).map(ranked).collect();
    let older: Vec<Ranked<'_>> =
        versions.iter().enumerate().take(start).filter(|(_, v)| v.session_start || v.merge).map(ranked).collect();
    let skip = older.len().saturating_sub(MAX_MILESTONES);
    HistoryPage {
        total,
        versions: page,
        milestones: older.into_iter().skip(skip).collect(),
        next_before: (start > 0).then_some(start),
    }
}

#[cfg(test)]
#[path = "history_page_tests.rs"]
mod history_page_tests;
