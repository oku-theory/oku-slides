//! The lease of the edited library pack: taken, checked and moved in pack mode.
//!
//! A pack tab of a library pack holds its lease (`pack_lease`)
//! in its `Mode::Pack`. An editor without it is read-only: every write first
//! asks for it again (`ensure_pack_lease`). The two library writers of pack
//! mode (`pack_library.rs`) take it as a witness. A pack tab freezes its base
//! from the library's bytes once it holds the lease (`freeze_base`).

use std::path::PathBuf;

use snafu::{OptionExt, ensure};
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackVersion};

use crate::agent::bounds::label;
use crate::error::{EditingPackSnafu, LeaseMissingSnafu, PackVersionMissingSnafu, UnknownPackSnafu};
use crate::library::{DraftLock, draft_lock};
use crate::library_kind::StoredVersion;
use crate::pack::PackManifest;
use crate::pack_lease::{PackLease, covers, pack_leased};
use crate::pack_receive::{deck_scope, is_library_pack};
use crate::views::EditingView;
use crate::{Mode, PackLibrary, PackMode, Result, Session};

impl Session {
    /// Checks the lease a pack tab enters `pack` (read as `stored`) with.
    ///
    /// The lease `given` must be `pack`'s in this library, and `pack` a
    /// library pack. A deck-pack tab never comes here: `enter_pack` gives its
    /// link no lease itself.
    ///
    /// # Errors
    ///
    /// `LeaseMissing` without a matching lease, `DeckOnly` for a deck-only pack.
    pub(crate) fn settle_lease(
        &self,
        pack: Uuid,
        stored: &StoredVersion<PackManifest>,
        given: Option<PackLease>,
    ) -> Result<PackLease> {
        let Some(lease) = given else {
            tracing::error!(pack = %pack, "pack tab entered without its lease");
            return LeaseMissingSnafu.fail()?;
        };
        if !covers(Some(&lease), self.library.lease_scope(), pack) {
            tracing::error!(pack = %pack, "lease of another pack or library given to its editor");
            return LeaseMissingSnafu.fail()?;
        }
        ensure!(is_library_pack(&stored.files), crate::error::DeckOnlySnafu { id: pack });
        Ok(lease)
    }

    /// Freezes `base` of the leased pack from the library's bytes when it is a draft.
    ///
    /// A base the library no longer lists is skipped (as `adopt_pins` does);
    /// a frozen or shipped one is left as is (`adopt` is idempotent).
    ///
    /// # Errors
    ///
    /// `LeaseMissing` unless `lease` is live in this library; the library's read and adopt errors.
    ///
    /// # Blocking
    ///
    /// Reads the version and takes `DRAFTS` to adopt it.
    pub(crate) fn freeze_base(&self, lease: &PackLease, base: &PackVersion) -> Result<()> {
        let pack = lease.pack();
        ensure!(self.library.lease_scope().is_some_and(|scope| lease.is(scope, pack)), LeaseMissingSnafu);
        if !self.library.lists(pack, base) {
            return Ok(());
        }
        // The bytes are read before `DRAFTS`: no disk read under it.
        let files =
            self.library.version(pack, base).context(PackVersionMissingSnafu { id: pack, version: base.clone() })?;
        let lock = draft_lock();
        let adopted = self.library.adopt(&lock, pack, &files);
        drop(lock);
        adopted
    }

    /// True when the edited pack is leasable: a library pack with a lease scope.
    ///
    /// Working files of the base that are missing, or of an unreadable scope, count as a library pack.
    #[must_use]
    pub(crate) fn needs_lease(&self) -> bool {
        let Mode::Pack { edit, .. } = &self.mode else { return false };
        self.library.lease_scope().is_some()
            && self.doc.sources.packs.get(&edit.base).is_none_or(|files| deck_scope(files) != Some(true))
    }

    /// The read-only and leased flags of the edited pack, for the snapshot.
    ///
    /// Read from the library's index only (no version load).
    #[must_use]
    pub(crate) fn edit_flags(&self) -> (bool, bool) {
        let Mode::Pack { edit, lease, orphaned, .. } = &self.mode else { return (false, true) };
        let scope = self.library.lease_scope();
        let needs = self.needs_lease();
        let covered = covers(lease.as_ref(), scope, edit.pack);
        let gone = || {
            !self.doc.sources.packs.contains_key(&edit.base)
                || self.library.current(edit.pack).is_none()
                || !self.library.lists(edit.pack, &edit.base)
        };
        let held = || scope.is_some_and(|scope| pack_leased(scope, edit.pack).is_some());
        let read_only = *orphaned || (needs && !covered && (gone() || held()));
        (read_only, !needs || covered)
    }

    /// Pack mode for the snapshot: the edited pack and its lease flags.
    pub(crate) fn editing_view(&self) -> Option<EditingView<'_>> {
        let Mode::Pack { edit, .. } = &self.mode else { return None };
        let (read_only, leased) = self.edit_flags();
        Some(EditingView { mode: edit, read_only, leased, display_name: label(&edit.name), unsent: self.unsent() })
    }

    /// Takes the edited pack's lease before a write, if this editor lacks it.
    ///
    /// Only in pack mode. A pack the library no longer lists marks the editor
    /// orphaned. A pack tab that never saved freezes its base once it holds the lease.
    ///
    /// # Errors
    ///
    /// `PackVersionMissing` without the base's working files, `UnknownPack`
    /// for an orphaned editor, `PackOpenInTab` while another editor holds it,
    /// and the `freeze_base` errors.
    ///
    /// # Blocking
    ///
    /// Reads the library index, takes `DRAFTS` through `acquire` and adopts through `freeze_base`.
    pub(crate) fn ensure_pack_lease(&mut self) -> Result<()> {
        let Mode::Pack { edit, lease, orphaned, .. } = &self.mode else { return Ok(()) };
        let (pack, base) = (edit.pack, edit.base.clone());
        ensure!(self.doc.sources.packs.contains_key(&base), PackVersionMissingSnafu { id: pack, version: base });
        ensure!(!*orphaned, UnknownPackSnafu { id: pack });
        let Some(scope) = self.library.lease_scope().filter(|_| self.needs_lease()) else { return Ok(()) };
        if covers(lease.as_ref(), Some(scope), pack) {
            return Ok(());
        }
        if self.library.current(pack).is_none() || !self.library.lists(pack, &base) {
            if let Mode::Pack { orphaned, .. } = &mut self.mode {
                *orphaned = true;
            }
            tracing::warn!(pack = %pack, "edited pack no longer in the library: read-only");
            return UnknownPackSnafu { id: pack }.fail()?;
        }
        let (name, unsaved) = (label(&edit.name), edit.session_version.is_none());
        let taken = PackLease::acquire(scope, pack, &name, &name)?;
        if unsaved {
            self.freeze_base(&taken, &base)?;
        }
        if let Mode::Pack { lease, .. } = &mut self.mode {
            *lease = Some(taken);
        }
        Ok(())
    }

    /// The lease of a save-as-new copy, and the id check of a plain save.
    ///
    /// Under `lock`, before the library write. Unless `as_new`, `manifest`
    /// must be `edit.pack`'s (a forged base is refused). A library copy under
    /// a new id is leased for its editor; a deck-only copy takes none.
    ///
    /// # Errors
    ///
    /// `LeaseMissing` for a manifest of another pack, or a copy's id already leased.
    pub(crate) fn copy_lease(
        &self,
        lock: &DraftLock,
        edit: &PackMode,
        as_new: bool,
        manifest: &PackManifest,
        files: &PackFiles,
    ) -> Result<Option<PackLease>> {
        if !as_new {
            if manifest.id != edit.pack {
                tracing::error!(pack = %edit.pack, found = %manifest.id, "manifest of another pack written: refused");
                return LeaseMissingSnafu.fail()?;
            }
            return Ok(None);
        }
        let Some(scope) = self.library.lease_scope().filter(|_| manifest.id != edit.pack && is_library_pack(files))
        else {
            return Ok(None);
        };
        let name = label(&manifest.name);
        match PackLease::try_take(lock, scope, manifest.id, &name, &name) {
            Ok(lease) => Ok(Some(lease)),
            Err(e) => {
                tracing::error!(pack = %manifest.id, error = %e.one_line(), "a brand-new pack id is already leased");
                LeaseMissingSnafu.fail()?
            }
        }
    }

    /// The body of `set_pack_dirs`: a new library, refused in pack mode.
    ///
    /// A pack lease's scope is the current pack library's: pack mode is
    /// refused. A theme tab's lease is the theme library's, which stays: it is kept.
    pub(crate) fn move_pack_dirs(&mut self, widgets: PathBuf, layouts: PathBuf) -> Result<()> {
        ensure!(!matches!(self.mode, Mode::Pack { .. }), EditingPackSnafu);
        let themes = self.themes.clone();
        self.library = PackLibrary::new(Some(widgets), Some(layouts), themes, self.storage.clone());
        self.pack_cache.clear();
        self.seed_cache.clear();
        self.refresh_pack_updates();
        Ok(())
    }
}
