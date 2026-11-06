//! The sources budget an in-app addition must keep (`[sources] max_sources_mib`).
//!
//! Opening a deck refuses sources past `SourceLimits::total_bytes`: a deck
//! the app let grow past it would not reopen. Every addition a user deck takes
//! (pasted or carried assets, library packs, theme closures, an import, a
//! merge, a deck-pack save) asks `ensure_sources_room` first, so a refusal
//! changes nothing. The replica is exported to weigh it, only when something
//! is added.

use snafu::ResultExt;

use oku_slides_core::{PackFiles, PackVersion, SourceBundle, SourceLimits, SourcesError};

use crate::deck_closure::Pinned;
use crate::error::{SourcesOverBudgetSnafu, SourcesSnafu};
use crate::{Result, Session};

/// One mebibyte, the unit of `[sources] max_sources_mib`.
const MIB: u64 = 1024 * 1024;

/// A byte count as `u64`; a `usize` never exceeds it on supported targets.
fn bytes(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// Bytes a pack or theme version adds to the sources, as the source zip holds them.
pub(crate) fn files_bytes(files: &PackFiles) -> u64 {
    let manifest = files.pack_json().map_or(u64::MAX, |json| bytes(json.len()));
    files.widgets().values().map(|w| bytes(w.len())).fold(manifest, u64::saturating_add)
}

/// Bytes the bundle weighs against `SourceLimits::total_bytes`.
///
/// # Errors
///
/// `Sources` when the replica does not export: a real fault, never a refusal.
pub(crate) fn source_bytes(sources: &SourceBundle) -> Result<u64> {
    let replica = sources.replica().snapshot().map_err(SourcesError::from).context(SourcesSnafu)?.len();
    let blobs = sources.assets.values().chain(sources.fonts.values()).map(|b| bytes(b.len()));
    let packs = sources.packs.values().chain(sources.themes.values()).map(files_bytes);
    Ok(blobs.chain(packs).fold(bytes(replica), u64::saturating_add))
}

/// Bytes of the `versions` a bundle holding `held` does not have yet.
pub(crate) fn new_files_bytes<'a>(
    held: &std::collections::BTreeMap<PackVersion, PackFiles>,
    versions: impl IntoIterator<Item = &'a PackFiles>,
) -> u64 {
    let mut seen = std::collections::BTreeSet::new();
    versions
        .into_iter()
        .filter(|f| {
            let version = f.version();
            !held.contains_key(&version) && seen.insert(version)
        })
        .map(files_bytes)
        .fold(0, u64::saturating_add)
}

/// Bytes of the pinned `packs` a bundle holding `held` does not have yet.
pub(crate) fn new_packs_bytes(held: &std::collections::BTreeMap<PackVersion, PackFiles>, packs: &[Pinned]) -> u64 {
    new_files_bytes(held, packs.iter().map(|(_, _, files)| files))
}

/// Bytes a merge of `theirs` into `ours` may add.
///
/// Their assets, fonts, packs and themes ours lacks, and their whole replica:
/// the merged history holds at most their changes on top of ours.
///
/// # Errors
///
/// `Sources` when their replica does not export.
pub(crate) fn merge_bytes(ours: &SourceBundle, theirs: &SourceBundle) -> Result<u64> {
    let replica = theirs.replica().snapshot().map_err(SourcesError::from).context(SourcesSnafu)?.len();
    let assets = theirs.assets.iter().filter(|(id, _)| !ours.assets.contains_key(*id)).map(|(_, b)| b);
    let fonts = theirs.fonts.iter().filter(|(name, _)| !ours.fonts.contains_key(*name)).map(|(_, b)| b);
    let blobs = assets.chain(fonts).map(|b| bytes(b.len())).fold(bytes(replica), u64::saturating_add);
    let packs = new_files_bytes(&ours.packs, theirs.packs.values());
    let themes = new_files_bytes(&ours.themes, theirs.themes.values());
    Ok(blobs.saturating_add(packs).saturating_add(themes))
}

/// Refuses `sources` grown by `added` bytes past `limits.total_bytes`.
///
/// # Errors
///
/// `SourcesOverBudget` past the budget; `Sources` when the replica does not export.
pub(crate) fn ensure_room(sources: &SourceBundle, limits: SourceLimits, added: u64) -> Result<()> {
    let total = source_bytes(sources)?.saturating_add(added);
    let max = bytes(limits.total_bytes);
    if total > max {
        tracing::warn!(total, max, "addition refused: over the sources budget");
        return SourcesOverBudgetSnafu { limit_mib: max / MIB }.fail()?;
    }
    Ok(())
}

impl Session {
    /// Refuses an addition of `added` bytes that would pass the sources budget.
    ///
    /// Nothing added costs nothing: the replica is only exported for an addition.
    ///
    /// # Errors
    ///
    /// `SourcesOverBudget` past the budget; `Sources` when the replica does not export.
    pub(crate) fn ensure_sources_room(&self, added: u64) -> Result<()> {
        if added == 0 {
            return Ok(());
        }
        ensure_room(&self.doc.sources, self.source_limits, added)
    }
}

#[cfg(test)]
#[path = "sources_budget_tests.rs"]
mod sources_budget_tests;
