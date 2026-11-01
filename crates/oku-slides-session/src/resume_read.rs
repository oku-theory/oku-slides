//! Bounded reads of a tab's resume blobs: one budget per tab, a cost per entry (t-1382).
//!
//! Every file read from `assets/`, `fonts/`, `packs/` and `themes/` is charged against one
//! [`ReadBudget`] (`[sources] max_sources_mib`), shared by every document of the tab (a pack
//! tab reads two). Each listed entry costs at least [`ENTRY_BYTES`], so the same setting also
//! caps how many files a resume may hold.

use std::collections::BTreeMap;
use std::path::Path;

use oku_slides_core::{SourceLimits, WidgetFile};
use snafu::{OptionExt, ResultExt};

use super::{AccessSnafu, BudgetExhaustedSnafu, Result, Resume};

/// Least bytes a listed entry costs, however small: 100 000 empty files are not free.
pub(crate) const ENTRY_BYTES: u64 = 4096;

/// What is left to read of one tab's resume, shared by its documents.
#[derive(Debug)]
pub(crate) struct ReadBudget {
    /// The source limits: `total_bytes` started the budget, `replica_bytes` caps each replica.
    pub(crate) limits: SourceLimits,
    /// Bytes still allowed.
    left: u64,
}

impl ReadBudget {
    /// A full budget of `limits.total_bytes`.
    pub(crate) fn new(limits: SourceLimits) -> Self {
        Self { limits, left: u64::try_from(limits.total_bytes).unwrap_or(u64::MAX) }
    }
}

/// A pack's files in a resume folder: its `<hex>.json`, if there, and its `<hex>.<file>.typ`.
pub(super) type PackEntry<'a> = (Option<&'a [u8]>, Vec<(WidgetFile, &'a [u8])>);

/// The files of `packs/` grouped by version hex, in one pass over the names.
pub(super) fn group_packs(raw: &BTreeMap<String, Vec<u8>>) -> BTreeMap<&str, PackEntry<'_>> {
    let mut packs: BTreeMap<&str, PackEntry<'_>> = BTreeMap::new();
    for (name, bytes) in raw {
        let Some((hex, rest)) = name.split_once('.') else { continue };
        if rest == "json" {
            packs.entry(hex).or_default().0 = Some(bytes);
        } else if let Some(widget) = WidgetFile::from_file_name(rest) {
            packs.entry(hex).or_default().1.push((widget, bytes));
        }
    }
    packs
}

impl Resume {
    /// The files of `dir`, each charged against `budget`: its length, [`ENTRY_BYTES`] at least.
    ///
    /// Hidden entries are skipped but charged too.
    ///
    /// # Errors
    ///
    /// `BudgetExhausted` when the budget cannot pay an entry; `Access` when a file is unreadable
    /// or larger than what is left.
    pub(super) fn read_files(&self, dir: &Path, budget: &mut ReadBudget) -> Result<BTreeMap<String, Vec<u8>>> {
        let mut files = BTreeMap::new();
        for path in self.storage.list_files(dir).context(AccessSnafu { path: dir })? {
            let room = budget.left.checked_sub(ENTRY_BYTES).context(BudgetExhaustedSnafu { path: dir })?;
            budget.left = room;
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if name.is_empty() || name.starts_with('.') {
                continue;
            }
            // The entry's first `ENTRY_BYTES` are paid already.
            let bytes = self.storage.read_limited(&path, room + ENTRY_BYTES).context(AccessSnafu { path: &path })?;
            let len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
            budget.left = room.saturating_sub(len.saturating_sub(ENTRY_BYTES));
            files.insert(name, bytes);
        }
        Ok(files)
    }
}

#[cfg(test)]
#[path = "resume_read_tests.rs"]
mod resume_read_tests;
