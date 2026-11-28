//! The session's share of the data base store: its leases, and the bridge to the core zip code.

use std::collections::BTreeSet;
use std::io::Read;

use oku_slides_core::{BaseFiles, BaseFilesError};
use oku_slides_data::BaseId;
use oku_slides_platform::bases::BaseStore;

/// The base store as one session uses it: every base its document knows stays leased.
#[derive(Debug)]
pub(crate) struct SessionBases {
    store: BaseStore,
    /// The bases this session holds a lease on.
    leased: BTreeSet<BaseId>,
}

impl SessionBases {
    /// A session's view of `store`, leasing nothing yet.
    pub(crate) fn new(store: BaseStore) -> Self {
        Self { store, leased: BTreeSet::new() }
    }

    /// The shared store.
    pub(crate) fn store(&self) -> &BaseStore {
        &self.store
    }

    /// The store as the core zip code reads and writes it.
    pub(crate) fn files(&self) -> StoreFiles<'_> {
        StoreFiles(&self.store)
    }

    /// Leases the bases of `known` not leased yet; none is released ([`forget`](Self::forget) does).
    pub(crate) fn sync(&mut self, known: &BTreeSet<BaseId>) {
        let new: Vec<BaseId> = known.difference(&self.leased).cloned().collect();
        if new.is_empty() {
            return;
        }
        // The in-process lease is taken before the file is written: the bases are held in this
        // process either way, only another process's collection may miss them.
        if let Err(e) = self.store.lease(&new) {
            tracing::warn!(error = %e, "base lease file not written");
        }
        self.leased.extend(new);
    }

    /// Releases the leases on `ids` this session holds, for bases its document no longer knows.
    pub(crate) fn forget(&mut self, ids: &BTreeSet<BaseId>) {
        let held: Vec<BaseId> = ids.intersection(&self.leased).cloned().collect();
        if held.is_empty() {
            return;
        }
        if let Err(e) = self.store.release(&held) {
            tracing::warn!(error = %e, "base leases not released");
        }
        for id in &held {
            self.leased.remove(id);
        }
    }
}

impl Drop for SessionBases {
    fn drop(&mut self) {
        if let Err(e) = self.store.release(&self.leased) {
            tracing::warn!(error = %e, "base leases not released");
        }
    }
}

impl crate::Session {
    /// Leases the bases the document knows and lets the render worker read them, and those of
    /// the query results (leased by their adoption).
    pub(crate) fn sync_bases(&mut self) {
        self.bases.sync(&self.doc.sources.bases);
        let mut known = self.doc.sources.bases.clone();
        known.extend(self.derived.bases());
        self.worker.set_known_bases(known);
    }

    /// Unknows `ids` (a refused paste's bases) and releases their leases.
    pub(crate) fn forget_bases(&mut self, ids: &BTreeSet<BaseId>) {
        self.doc.sources.bases.retain(|id| !ids.contains(id));
        self.bases.forget(ids);
        self.sync_bases();
    }
}

/// The platform's [`BaseStore`] as the core's [`BaseFiles`].
#[derive(Debug, Clone, Copy)]
pub struct StoreFiles<'a>(&'a BaseStore);

impl<'a> StoreFiles<'a> {
    /// `store` as the core zip code reads and writes it.
    #[must_use]
    pub fn new(store: &'a BaseStore) -> Self {
        Self(store)
    }
}

impl BaseFiles for StoreFiles<'_> {
    fn open(&self, id: &BaseId) -> Result<Box<dyn Read + '_>, BaseFilesError> {
        Ok(Box::new(self.0.open(id)?))
    }

    /// Installs the base, then drops the lease `put_stream` took: the document that knows it
    /// is leased by [`SessionBases::sync`] once installed, well within the GC grace period.
    fn put(&self, r: &mut dyn Read, id: &BaseId, max: u64) -> Result<(), BaseFilesError> {
        self.0.put_stream(r, id, max)?;
        Ok(self.0.release([id])?)
    }
}

#[cfg(test)]
#[path = "bases_tests.rs"]
pub(crate) mod bases_tests;
