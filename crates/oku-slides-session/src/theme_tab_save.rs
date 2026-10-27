//! Saving a theme tab's draft, freezing its base, and its theme's lease.
//!
//! A theme tab saves by the library's draft rule (`library_draft`): its
//! first write is a new version parented on the version it descends from
//! (`ThemeTab::parent`), the next ones replace it until a deck adopts it,
//! then a child of the frozen version starts and the tab descends from it.
//! A child of a built-in, or a copy, is a new theme: the tab takes its lease
//! under the same `DRAFTS` scope, then edits it. The tab holds its theme's
//! lease while awake; without it, it is read-only and every draft request
//! asks for it again (`ensure_theme_lease`). Opening or resuming a user
//! theme that has not written yet freezes its base (`freeze_theme_base`).

use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::ThemePin;

use crate::agent::bounds::label;
use crate::error::{
    LeaseMissingSnafu, ThemeJsonSnafu, ThemeVersionMissingSnafu, UnknownThemeSnafu, WrongDocumentSnafu,
};
use crate::library::{DraftLock, DraftWrite, draft_lock};
use crate::pack_lease::{PackLease, covers};
use crate::theme::ThemeManifest;
use crate::theme_closure::pin_of;
use crate::theme_edit::ThemeEdit;
use crate::{Mode, Result, Session, SessionErrorKind};

impl Session {
    /// Saves the theme draft by the library's draft rule; `None` when unchanged.
    ///
    /// `as_new`: a copy under a new id, even of an unchanged draft. The tab
    /// then edits the saved version (a new theme for a built-in's child or a
    /// copy), is no longer dirty, and its state is persisted. Saving pins
    /// nothing in any deck.
    ///
    /// # Errors
    ///
    /// `WrongDocument` outside a theme tab; `ThemeOpenInTab` while another
    /// tab holds the theme (or the new theme's id); `UnknownTheme` and the
    /// closure errors when a parent is missing, checked before any write;
    /// `ThemeVersionMissing` without the draft's base; the library's write errors.
    ///
    /// # Blocking
    ///
    /// Reads and writes the theme library under `DRAFTS`.
    pub fn save_theme(&mut self, as_new: bool) -> Result<Option<ThemePin>> {
        self.ensure_theme_tab()?;
        let Some(manifest) = self.theme_to_save(as_new)? else { return Ok(None) };
        let pin = self.write_theme(manifest)?;
        // `handle` persists only requests that change the state: this one says so itself.
        self.persist(true);
        Ok(Some(pin))
    }

    /// The manifest the draft saves, its closure found; `None` when unchanged (unless `as_new`).
    pub(crate) fn theme_to_save(&self, as_new: bool) -> Result<Option<ThemeManifest>> {
        self.ensure_theme_tab()?;
        let Mode::Theme { edit, .. } = &self.mode else { return Ok(None) };
        let base = &edit.draft.base;
        let missing = || ThemeVersionMissingSnafu { id: base.id, version: base.version.clone() };
        let manifest = self.theme_version(base).context(missing())?;
        let saved = self.saved_manifest((&edit.draft, edit.parent.clone()), manifest, self.edited_tokens(), as_new)?;
        let Some(saved) = saved else { return Ok(None) };
        let files = saved.to_files().context(ThemeJsonSnafu { id: saved.id })?;
        // Its parents must be found before any write.
        self.closure_from(&saved.namespace, &(pin_of(&saved, files.version()), files))?;
        Ok(Some(saved))
    }

    /// Writes `manifest`, the draft to save, by the draft rule; the tab then edits it.
    ///
    /// Under one `DRAFTS` scope: the write decided (`draft_write`), a new
    /// theme's lease taken, the lease checked, the files rebuilt on a frozen
    /// draft (`Child`), then `save_draft`.
    pub(crate) fn write_theme(&mut self, mut manifest: ThemeManifest) -> Result<ThemePin> {
        // Outside a theme tab: the session's kind, never a lease fault.
        let Mode::Theme { edit, .. } = &self.mode else {
            return WrongDocumentSnafu { kind: self.document_kind() }.fail()?;
        };
        let (old, session) = (edit.draft.base.clone(), edit.session_version.clone());
        let (id, fresh) = (manifest.id, manifest.id != old.id);
        if !fresh {
            self.ensure_theme_lease()?;
        }
        let mut files = manifest.to_files().context(ThemeJsonSnafu { id })?;
        let lock = draft_lock();
        let write = if fresh { DraftWrite::New } else { self.themes.draft_write(&lock, id, session.as_ref()) };
        let taken = if fresh { self.new_theme_lease(&lock, &manifest)? } else { None };
        let witness = match &self.mode {
            _ if fresh => taken.as_ref(),
            Mode::Theme { lease, .. } => lease.as_ref(),
            _ => None,
        };
        if !covers(witness, self.library.theme_scope(), id) {
            tracing::error!(theme = %id, "theme draft written without its lease: refused");
            return LeaseMissingSnafu.fail()?;
        }
        // The draft is frozen (a deck adopted it): the new version is its child.
        if let DraftWrite::Child(frozen) = &write {
            manifest.parent = Some(frozen.clone());
            files = manifest.to_files().context(ThemeJsonSnafu { id })?;
        }
        let version = self.themes.save_draft(&lock, id, &write, &files)?;
        drop(lock);
        let pin = pin_of(&manifest, version.clone());
        // The saved version is the tab's base from now on: read in memory.
        self.doc.sources.add_theme(files);
        let saved = ThemeEdit::starting(pin.clone(), &manifest);
        if let Mode::Theme { edit, switches, lease } = &mut self.mode {
            // The parents' changes that led to the old base lead to the saved version, each with its parents.
            for s in switches.iter_mut() {
                for e in [&mut s.from, &mut s.to] {
                    if e.base == old {
                        *e = ThemeEdit { extends: std::mem::take(&mut e.extends), ..saved.clone() };
                    }
                }
            }
            edit.parent = match write {
                _ if fresh => None,
                DraftWrite::Child(frozen) => Some(frozen),
                DraftWrite::New | DraftWrite::Replace(_) => edit.parent.take(),
            };
            edit.draft = saved;
            edit.session_version = Some(version);
            // A new theme holds its own lease; the old theme's drops here.
            if fresh {
                *lease = taken;
            }
        }
        self.doc.dirty = false;
        self.theme_list = self.themes.themes();
        Ok(pin)
    }

    /// The lease of the new theme `manifest`, under `lock`; `None` without a theme lease scope.
    fn new_theme_lease(&self, lock: &DraftLock, manifest: &ThemeManifest) -> Result<Option<PackLease>> {
        let Some(scope) = self.library.theme_scope() else { return Ok(None) };
        let name = label(&manifest.name);
        PackLease::try_take(lock, scope, manifest.id, &name, &name).map(Some)
    }

    /// Takes the edited theme's lease before a write, if this tab lacks it.
    ///
    /// Only in a theme tab. A tab that never wrote freezes its base once it holds the lease.
    ///
    /// # Errors
    ///
    /// `ThemeOpenInTab` while another tab holds it, and the `freeze_theme_base` errors.
    ///
    /// # Blocking
    ///
    /// Takes `DRAFTS` through `acquire` and adopts through `freeze_theme_base`.
    pub(crate) fn ensure_theme_lease(&mut self) -> Result<()> {
        let Mode::Theme { edit, lease, .. } = &self.mode else { return Ok(()) };
        let id = edit.draft.base.id;
        let Some(scope) = self.library.theme_scope() else { return Ok(()) };
        if covers(lease.as_ref(), Some(scope), id) {
            return Ok(());
        }
        let (name, unwritten) = (label(&edit.draft.name), edit.session_version.is_none());
        let taken = PackLease::acquire(scope, id, &name, &name)?;
        if unwritten {
            self.freeze_theme_base(&taken)?;
        }
        if let Mode::Theme { lease, .. } = &mut self.mode {
            *lease = Some(taken);
        }
        Ok(())
    }

    /// Freezes the draft's base from the library's bytes, under `lease`.
    ///
    /// A base the library does not list is skipped; a built-in, received or
    /// adopted one is left as is (`adopt` is idempotent).
    ///
    /// # Errors
    ///
    /// `LeaseMissing` unless `lease` is the base's in this library; the library's read and adopt errors.
    ///
    /// # Blocking
    ///
    /// Reads the version and takes `DRAFTS` to adopt it.
    pub(crate) fn freeze_theme_base(&self, lease: &PackLease) -> Result<()> {
        let Mode::Theme { edit, .. } = &self.mode else { return Ok(()) };
        let base = &edit.draft.base;
        ensure!(self.library.theme_scope().is_some_and(|scope| lease.is(scope, base.id)), LeaseMissingSnafu);
        if !self.themes.lists(base.id, &base.version) {
            return Ok(());
        }
        // The bytes are read before `DRAFTS`: no disk read under it.
        let missing = || ThemeVersionMissingSnafu { id: base.id, version: base.version.clone() };
        let files = self.themes.version(base.id, &base.version).context(missing())?;
        let lock = draft_lock();
        let adopted = self.themes.adopt(&lock, base.id, &files);
        drop(lock);
        adopted
    }

    /// The lease of `theme` for a tab opening it; `None` while another tab holds it.
    ///
    /// A theme tab without its lease opens read-only. `None` too without a theme lease scope.
    ///
    /// # Errors
    ///
    /// `UnknownTheme` for a theme the library lacks.
    ///
    /// # Blocking
    ///
    /// Reads the library index and takes `DRAFTS` through `acquire`.
    pub(crate) fn opening_theme_lease(&self, theme: Uuid) -> Result<Option<PackLease>> {
        let current = self.themes.current_version(theme).context(UnknownThemeSnafu { theme: theme.to_string() })?;
        let Some(scope) = self.library.theme_scope() else { return Ok(None) };
        let name = label(&current.manifest.name);
        match PackLease::acquire(scope, theme, &name, &name) {
            Ok(lease) => Ok(Some(lease)),
            Err(e) if e.kind() == SessionErrorKind::ThemeOpenInTab => {
                tracing::info!(theme = %theme, "theme edited in another tab: opened read-only");
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Freezes the opened theme's base when the tab holds its lease (`init_theme_tab`).
    pub(crate) fn freeze_opened_theme(&self) -> Result<()> {
        match &self.mode {
            Mode::Theme { lease: Some(lease), .. } => self.freeze_theme_base(lease),
            _ => Ok(()),
        }
    }

    /// True when this theme tab may write without first taking the lease.
    pub(crate) fn theme_leased(&self) -> bool {
        let Mode::Theme { edit, lease, .. } = &self.mode else { return false };
        covers(lease.as_ref(), self.library.theme_scope(), edit.draft.base.id)
    }
}

#[cfg(test)]
#[path = "theme_tab_save_tests.rs"]
mod theme_tab_save_tests;

#[cfg(test)]
#[path = "theme_tab_lease_tests.rs"]
mod theme_tab_lease_tests;
