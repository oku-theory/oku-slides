//! The query results a deck file caches (`data/<key hex>.json`, see `oku_slides_core::CachedResult`).
//!
//! Open seeds a query `Ready` when the key computed from the opened deck names a cache entry
//! (`Derived::seed`): its view draws with no recompute. Any other entry is ignored and its query
//! computes again. Save writes the ready results of the current keys only (`Derived::cache`).
//! [`DeckResults`] runs the same seed and recompute for the one-shot CLI.

use std::collections::BTreeMap;

use oku_slides_core::{CachedResult, Deck, Issue, ResultKeyHex};
use oku_slides_data::BaseId;
use oku_slides_platform::bases::BaseStore;
use oku_slides_render::Results;
use oku_slides_render_worker::QueryOut;

use super::{Derived, QueryState, ResultKey, keyed};
use crate::Session;
use crate::data_link::DataLink;

/// The cache entry name of `key`.
fn hex_of(key: ResultKey) -> ResultKeyHex {
    hex::encode(key.0)
}

impl Derived {
    /// Sets ready each query of `deck` whose current key names an entry of `cached`, its base
    /// leased in the store; an entry whose base the store lacks is skipped (logged).
    pub(crate) fn seed(&mut self, deck: &Deck, cached: &BTreeMap<ResultKeyHex, CachedResult>) {
        if cached.is_empty() {
            return;
        }
        for (id, keyed) in keyed(deck) {
            let Some(keyed) = keyed else { continue };
            let Some(entry) = cached.get(&hex_of(keyed.key)) else { continue };
            let base = &entry.base.id;
            let leased = self
                .store
                .contains(base)
                .and_then(|held| if held { self.store.lease([base]).map(|()| true) } else { Ok(false) });
            match leased {
                Ok(true) => {
                    let out = QueryOut { base: entry.base.clone(), columns: entry.columns.clone() };
                    self.set(id, QueryState::ready(keyed.key, out));
                }
                Ok(false) => tracing::warn!(data = ?id, "cached query result skipped: its base is not stored"),
                Err(e) => tracing::warn!(data = ?id, error = %e, "cached query result skipped: base not leased"),
            }
        }
    }

    /// The ready results of `deck`'s queries for their current keys, as the deck caches them.
    pub(crate) fn cache(&self, deck: &Deck) -> BTreeMap<ResultKeyHex, CachedResult> {
        keyed(deck)
            .into_iter()
            .filter_map(|(id, keyed)| match (self.states.get(&id)?, keyed?) {
                (QueryState::Ready { key, out, .. }, keyed) if *key == keyed.key => {
                    let hex = hex_of(*key);
                    Some((hex.clone(), CachedResult { key: hex, columns: out.columns.clone(), base: out.base.clone() }))
                }
                _ => None,
            })
            .collect()
    }
}

impl Session {
    /// Seeds the query results the installed document's file caches; the cache is then the
    /// session's to rewrite at the next save.
    pub(crate) fn seed_cached_results(&mut self) {
        let cached = std::mem::take(&mut self.doc.sources.results);
        self.derived.seed(self.doc.sources.deck(), &cached);
    }

    /// Sets the document's cache to the current ready results, before a save writes it.
    pub(crate) fn fill_cached_results(&mut self) {
        self.doc.sources.results = self.derived.cache(self.doc.sources.deck());
    }
}

/// A deck's query results for a one-shot command (the CLI's `check`, `render`, `save`): seeded
/// from the file's cache as an open does, the rest computed blocking in compute order.
///
/// The ready results' bases stay leased until it drops.
#[derive(Debug)]
pub struct DeckResults {
    derived: Derived,
}

impl DeckResults {
    /// The results of `deck`: those `cached` names for their current key, then the others
    /// computed by a query worker of `launch` and `settings`, started only if one is needed.
    ///
    /// # Blocking
    ///
    /// Runs every query left in the worker.
    #[must_use]
    pub fn compute(
        deck: &Deck,
        cached: &BTreeMap<ResultKeyHex, CachedResult>,
        launch: &oku_slides_platform::RenderLaunch,
        settings: &oku_slides_platform::config::RenderSettings,
        store: &BaseStore,
    ) -> Self {
        let mut derived = Derived::new(store.clone());
        derived.seed(deck, cached);
        let link = DataLink::new(launch.clone(), settings.clone(), store.clone());
        loop {
            let jobs = derived.jobs(deck, &|| link.query());
            if jobs.is_empty() {
                return Self { derived };
            }
            for job in jobs {
                let reply = job.run();
                // A failure stays in its state: `issues` reports it.
                derived.land(deck, job, reply);
            }
        }
    }

    /// The results `deck`'s views read, `None` for a query that did not compute.
    #[must_use]
    pub fn results(&self, deck: &Deck) -> Results {
        self.derived.results(deck)
    }

    /// `issue-data-query-failed` for each query of `deck` that failed.
    #[must_use]
    pub fn issues(&self, deck: &Deck) -> Vec<Issue> {
        self.derived.failed_issues(deck)
    }

    /// The bases of the ready results, which the render worker must know.
    pub fn bases(&self) -> impl Iterator<Item = BaseId> + '_ {
        self.derived.bases()
    }

    /// The ready results as the deck caches them, for a save.
    #[must_use]
    pub fn cache(&self, deck: &Deck) -> BTreeMap<ResultKeyHex, CachedResult> {
        self.derived.cache(deck)
    }
}

#[cfg(test)]
#[path = "derived_cache_tests.rs"]
mod derived_cache_tests;
