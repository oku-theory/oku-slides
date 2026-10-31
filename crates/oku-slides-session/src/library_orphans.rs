//! Empty drafts a crash left behind, collected at workspace resume (t-1251).
//!
//! `NewPack` and `NewTheme` create an empty draft, marked `created-empty`,
//! then open its tab, which freezes it (adopted): a crash in between leaves a
//! marked draft that no tab cites, which `Trash` never sees. At resume, such
//! a user pack or theme is an orphan when it holds that one marked version,
//! not frozen, current, its only folder on disk, created over [`ORPHAN_AGE`]
//! ago, and no tab shows it; an empty
//! pack made any other way (an import, a CLI write) is never one. Each goes
//! through `discard_untouched`, which refuses one another tab of this process
//! holds. Another process (the CLI's `serve` beside the app) is not seen: the
//! age keeps its fresh drafts, the rest of that race is accepted.

use time::{Duration, OffsetDateTime, PrimitiveDateTime, macros::format_description};
use uuid::Uuid;

use oku_slides_core::{PackKind, PackVersion};

use crate::library_packs::{DraftKind, PackLibrary};
use crate::theme::ThemeLibrary;

/// How old an empty draft must be before resume collects it.
pub(crate) const ORPHAN_AGE: Duration = Duration::days(1);

/// When `created` (`now_created`'s form, UTC) was; `None` when unreadable.
fn created_at(created: &str) -> Option<OffsetDateTime> {
    let format = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z");
    PrimitiveDateTime::parse(created, format).ok().map(PrimitiveDateTime::assume_utc)
}

/// True when `created` is at least `ORPHAN_AGE` before `now`; an unreadable date is never old.
fn old_enough(created: &str, now: OffsetDateTime) -> bool {
    created_at(created).is_some_and(|at| now - at >= ORPHAN_AGE)
}

impl PackLibrary {
    /// Deletes the orphan empty drafts `shown` does not name (see the module): their ids.
    ///
    /// Each is checked again under `DRAFTS` before its deletion. A failed deletion is
    /// logged and the draft kept.
    ///
    /// # Blocking
    ///
    /// Reads the libraries and may delete in them, taking `DRAFTS` per draft.
    pub(crate) fn collect_orphan_drafts(&self, shown: &[Uuid], now: OffsetDateTime) -> Vec<Uuid> {
        // Each found draft with its pack kind (`None`: a theme), rechecked under `DRAFTS`.
        let mut found: Vec<(Option<PackKind>, Uuid, PackVersion)> = Vec::new();
        let unshown_old = |id: Uuid, created: &str| !shown.contains(&id) && old_enough(created, now);
        for kind in PackKind::ALL {
            for s in self.entries(kind) {
                let id = s.manifest.id;
                if !Self::is_builtin(id)
                    && unshown_old(id, &s.manifest.created)
                    && self.is_orphan_draft(kind, id, &s.version)
                {
                    found.push((Some(kind), id, s.version));
                }
            }
        }
        let themes = self.themes();
        for s in themes.entries() {
            let id = s.manifest.id;
            if !ThemeLibrary::is_builtin(id) && unshown_old(id, &s.manifest.created) && themes.is_orphan(id, &s.version)
            {
                found.push((None, id, s.version));
            }
        }
        found
            .into_iter()
            .filter(|(kind, id, v)| {
                let (draft, id) = (kind.map_or(DraftKind::Theme, |_| DraftKind::Pack), *id);
                let still =
                    |l: &Self| kind.map_or_else(|| l.themes().is_orphan(id, v), |k| l.is_orphan_draft(k, id, v));
                match self.discard_if(draft, id, still) {
                    Ok(deleted) => deleted,
                    Err(e) => {
                        tracing::warn!(draft = %id, error = %e.one_line(), "orphan empty draft not deleted");
                        false
                    }
                }
            })
            .map(|(_, id, _)| id)
            .collect()
    }
}

#[cfg(test)]
#[path = "library_orphans_tests.rs"]
mod library_orphans_tests;
