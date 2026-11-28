//! Background recompute of a tab's query results, off the workspace and session locks.

use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

use super::Workspace;
use super::slot::Slot;

/// Told the tab whose query results landed, with no lock held.
pub(crate) type ResultsSink = Arc<dyn Fn(&str) + Send + Sync>;

impl Workspace {
    /// Calls `sink` with the tab each time query results land in it off the lock, so the host
    /// pushes the tab's views again (they were rendered with the results meanwhile).
    pub fn on_results(ws: &Mutex<Self>, sink: impl Fn(&str) + Send + Sync + 'static) {
        ws.lock().unwrap_or_else(PoisonError::into_inner).results_sink = Some(Arc::new(sink));
    }

    /// Starts a thread computing the query jobs of `tab`; `None` when no query needs one.
    ///
    /// The jobs are taken under the tab's lock, run with no lock held, and land under the lock
    /// again; the thread then takes the jobs they unblocked (their readers), until none is left.
    /// A tab put to sleep or closed meanwhile drops the jobs' bases. After each batch that
    /// changed a result, the sink of [`Workspace::on_results`] is told the tab, with no lock held.
    pub(crate) fn schedule_recomputes(ws: &Mutex<Self>, tab: &str) -> Option<JoinHandle<()>> {
        let (slot, sink) = {
            let w = ws.lock().unwrap_or_else(PoisonError::into_inner);
            (Arc::clone(&w.tabs.get(tab)?.session), w.results_sink.clone())
        };
        let first = {
            let mut guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
            let Slot::Ready(session) = &mut *guard else { return None };
            session.query_jobs()
        };
        if first.is_empty() {
            return None;
        }
        let tab = tab.to_owned();
        Some(std::thread::spawn(move || {
            let mut jobs = first;
            while !jobs.is_empty() {
                let mut landed = false;
                for job in jobs {
                    let reply = job.run();
                    let mut guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
                    match &mut *guard {
                        Slot::Ready(session) => landed |= session.finish_query(job, reply),
                        Slot::Dormant(_) | Slot::Closed(_) => job.abandon(reply),
                    }
                }
                if let Some(sink) = sink.as_ref().filter(|_| landed) {
                    sink(&tab);
                }
                let mut guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
                jobs = match &mut *guard {
                    Slot::Ready(session) => session.query_jobs(),
                    Slot::Dormant(_) | Slot::Closed(_) => Vec::new(),
                };
            }
        }))
    }
}
