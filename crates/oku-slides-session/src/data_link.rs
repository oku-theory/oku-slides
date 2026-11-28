//! The session's data workers: one for reads, launched at the first read of a data with a base,
//! one for queries, launched at the first query job.

use std::sync::{Arc, OnceLock};

use oku_slides_platform::RenderLaunch;
use oku_slides_platform::bases::BaseStore;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render_worker::DataWorker;

/// What launches the data worker, and the worker once launched.
pub(crate) struct DataLink {
    launch: RenderLaunch,
    settings: RenderSettings,
    store: BaseStore,
    worker: OnceLock<Arc<DataWorker>>,
    /// The worker running the queries, with their own bounds: a query does not wait for a read.
    query: OnceLock<Arc<DataWorker>>,
    /// Times the worker was handed out, for tests proving a path reads no base.
    #[cfg(test)]
    handed: std::sync::atomic::AtomicUsize,
}

impl DataLink {
    /// A link to a worker reading `store`, not launched yet.
    pub(crate) fn new(launch: RenderLaunch, settings: RenderSettings, store: BaseStore) -> Self {
        Self {
            launch,
            settings,
            store,
            worker: OnceLock::new(),
            query: OnceLock::new(),
            #[cfg(test)]
            handed: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// The worker, created at the first call; a compaction calls it with no session lock held.
    pub(crate) fn worker(&self) -> Arc<DataWorker> {
        #[cfg(test)]
        self.handed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let worker = self
            .worker
            .get_or_init(|| Arc::new(DataWorker::new(self.launch.clone(), &self.settings, self.store.clone())));
        Arc::clone(worker)
    }

    /// The query worker, created at the first call; a query job calls it with no session lock held.
    pub(crate) fn query(&self) -> Arc<DataWorker> {
        let worker = self
            .query
            .get_or_init(|| Arc::new(DataWorker::query(self.launch.clone(), &self.settings, self.store.clone())));
        Arc::clone(worker)
    }

    /// Times [`DataLink::worker`] was called: every base read or check goes through it.
    #[cfg(test)]
    pub(crate) fn handed(&self) -> usize {
        self.handed.load(std::sync::atomic::Ordering::Relaxed)
    }
}
