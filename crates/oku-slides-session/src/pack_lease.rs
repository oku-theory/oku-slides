//! The marks of library packs and themes being edited, per library.
//!
//! A library pack is leased while its pack tab edits it, a library theme
//! while its theme tab is awake: nobody
//! else writes it meanwhile. The registry is
//! process-wide (`PACK_TABS`), keyed by the library's [`LeaseScope`] and the
//! pack id, and stores the holder's label of record for refusals. It is a
//! pure registry: it knows `LeaseScope`, never `PackLibrary`.
//!
//! Lock order: `DRAFTS` (`draft_lock`), then `PACK_TABS`. A lease is taken
//! inside a `DRAFTS` scope (`try_take`, or `acquire` which takes it); dropping
//! one takes `PACK_TABS` alone.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use uuid::Uuid;

use oku_slides_core::PackFiles;
use oku_slides_platform::storage;

use crate::Result;
use crate::agent::bounds::label;
use crate::error::{PackOpenInTabSnafu, ThemeOpenInTabSnafu};
use crate::library::{DraftLock, draft_lock};
use crate::pack_receive::is_library_pack;

/// Per library, per leased pack: the lease's token and the holder's label.
type Leases = BTreeMap<LeaseScope, BTreeMap<Uuid, (u64, String)>>;

/// The leases of this process.
static PACK_TABS: Mutex<Leases> = Mutex::new(BTreeMap::new());

/// The next lease's identity.
static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);

/// The directories that identify one library for leases: of packs or of themes.
///
/// Its `Ord` keeps the registry a `BTreeMap`; the two kinds never share a key,
/// so a pack and a theme with equal ids are leased apart.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum LeaseScope {
    /// A pack library.
    Packs {
        /// The widget library directory, resolved.
        widgets: PathBuf,
        /// The layout library directory, resolved.
        layouts: PathBuf,
    },
    /// A theme library.
    Themes {
        /// The theme library directory, resolved.
        themes: PathBuf,
    },
}

impl LeaseScope {
    /// The lease key of these two pack library directories.
    #[must_use]
    pub(crate) fn new(widgets: &Path, layouts: &Path) -> Self {
        Self::Packs { widgets: storage::resolve(widgets), layouts: storage::resolve(layouts) }
    }

    /// The lease key of this theme library directory.
    #[must_use]
    pub(crate) fn themes(dir: &Path) -> Self {
        Self::Themes { themes: storage::resolve(dir) }
    }

    /// The refusal of a write to `name` while `tab` holds its lease in this scope.
    ///
    /// # Errors
    ///
    /// Always: `PackOpenInTab` for a pack library, `ThemeOpenInTab` for a theme library.
    pub(crate) fn held<T>(&self, name: &str, tab: &str) -> Result<T> {
        match self {
            Self::Packs { .. } => PackOpenInTabSnafu { pack: name, tab }.fail()?,
            Self::Themes { .. } => ThemeOpenInTabSnafu { theme: name, tab }.fail()?,
        }
    }
}

/// A library pack being edited; nobody else writes it meanwhile.
#[derive(Debug)]
#[must_use]
pub(crate) struct PackLease {
    /// The library's lease scope.
    scope: LeaseScope,
    /// The leased pack.
    pack: Uuid,
    /// This lease's identity in `PACK_TABS`, from `NEXT_TOKEN`.
    token: u64,
}

impl PackLease {
    /// Leases `pack` unless another editor holds it, inside a `DRAFTS` scope.
    ///
    /// `name` (the pack's) and `holder` (the editor's tab) are labels already.
    ///
    /// # Errors
    ///
    /// `PackOpenInTab { pack: name, tab: holder of record }` when held in a
    /// pack library, `ThemeOpenInTab { theme: name, tab }` in a theme library.
    pub(crate) fn try_take(
        _drafts: &DraftLock,
        scope: &LeaseScope,
        pack: Uuid,
        name: &str,
        holder: &str,
    ) -> Result<Self> {
        let token = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
        {
            let mut tabs = PACK_TABS.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some((_, tab)) = tabs.get(scope).and_then(|leases| leases.get(&pack)) {
                return scope.held(name, tab);
            }
            let entry = (token, holder.to_owned());
            if let Some(leases) = tabs.get_mut(scope) {
                leases.insert(pack, entry);
            } else {
                tabs.insert(scope.clone(), BTreeMap::from([(pack, entry)]));
            }
        }
        // Built once the guard is gone: a lease dropped under it would deadlock.
        Ok(Self { scope: scope.clone(), pack, token })
    }

    /// Leases `pack` like `try_take`, taking `DRAFTS` itself.
    ///
    /// # Errors
    ///
    /// `PackOpenInTab` or `ThemeOpenInTab` when held, as `try_take`.
    ///
    /// # Blocking
    ///
    /// Waits for `DRAFTS`, which library writes hold.
    pub(crate) fn acquire(scope: &LeaseScope, pack: Uuid, name: &str, holder: &str) -> Result<Self> {
        let lock = draft_lock();
        let lease = Self::try_take(&lock, scope, pack, name, holder);
        drop(lock);
        lease
    }

    /// True if this lease is `pack`'s in `scope`.
    #[must_use]
    pub(crate) fn is(&self, scope: &LeaseScope, pack: Uuid) -> bool {
        self.pack == pack && self.scope == *scope
    }

    /// The leased pack.
    #[must_use]
    pub(crate) fn pack(&self) -> Uuid {
        self.pack
    }
}

impl Drop for PackLease {
    fn drop(&mut self) {
        let mut tabs = PACK_TABS.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(leases) = tabs.get_mut(&self.scope) else {
            tracing::error!(pack = %self.pack, "lease dropped without its entry");
            return;
        };
        match leases.get(&self.pack) {
            Some((token, _)) if *token == self.token => {
                leases.remove(&self.pack);
                if leases.is_empty() {
                    tabs.remove(&self.scope);
                }
            }
            Some(_) => tracing::error!(pack = %self.pack, "lease dropped while another one holds its entry"),
            None => tracing::error!(pack = %self.pack, "lease dropped without its entry"),
        }
    }
}

/// The label of the tab holding `pack`'s lease, if any.
#[must_use]
pub(crate) fn pack_leased(scope: &LeaseScope, pack: Uuid) -> Option<String> {
    let tabs = PACK_TABS.lock().unwrap_or_else(PoisonError::into_inner);
    tabs.get(scope).and_then(|leases| leases.get(&pack)).map(|(_, tab)| tab.clone())
}

/// True when `lease` lets a write to `pack` in `scope` go on.
///
/// A library without a lease scope leases nothing: every write goes on.
#[must_use]
pub(crate) fn covers(lease: Option<&PackLease>, scope: Option<&LeaseScope>, pack: Uuid) -> bool {
    scope.is_none_or(|scope| lease.is_some_and(|l| l.is(scope, pack)))
}

/// Refuses a deck-mode write to a library pack another editor holds.
///
/// `files` are the pack version being written from; a deck-only pack is
/// never leased. `name` is the pack's name, cleaned here.
///
/// # Errors
///
/// `PackOpenInTab` naming the holder.
pub(crate) fn refuse_leased(scope: Option<&LeaseScope>, pack: Uuid, files: &PackFiles, name: &str) -> Result<()> {
    refuse_held(scope.filter(|_| is_library_pack(files)), pack, name)
}

/// Refuses a write to `id` in `scope` while an editor holds its lease.
///
/// `name` is the pack's or theme's name, cleaned here. Without a scope, nothing is leased.
///
/// # Errors
///
/// `PackOpenInTab` or `ThemeOpenInTab` (by scope) naming the holder.
pub(crate) fn refuse_held(scope: Option<&LeaseScope>, id: Uuid, name: &str) -> Result<()> {
    let Some(scope) = scope else { return Ok(()) };
    match pack_leased(scope, id) {
        Some(tab) => scope.held(&label(name), &tab),
        None => Ok(()),
    }
}

#[cfg(test)]
#[path = "pack_lease_tests.rs"]
mod pack_lease_tests;
