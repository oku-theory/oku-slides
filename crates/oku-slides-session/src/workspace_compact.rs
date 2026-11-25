//! Background compaction of a tab's data, off the workspace and session locks.

use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Instant;

use super::Workspace;
use super::slot::Slot;

impl Workspace {
    /// Starts a compaction thread for each data of `tab` due now; their handles.
    ///
    /// Each job snapshots under the tab's lock, runs the worker with no lock held, then commits
    /// under the lock again. A tab put to sleep or closed meanwhile drops the job's base.
    pub(super) fn schedule_compactions(ws: &Mutex<Self>, tab: &str) -> Vec<JoinHandle<()>> {
        let slot = {
            let w = ws.lock().unwrap_or_else(PoisonError::into_inner);
            match w.tabs.get(tab) {
                Some(t) => Arc::clone(&t.session),
                None => return Vec::new(),
            }
        };
        let jobs = {
            let mut guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
            let Slot::Ready(session) = &mut *guard else { return Vec::new() };
            let now = Instant::now();
            let due = session.compactions_due(now);
            due.iter().filter_map(|id| session.compaction_job(id, now, false)).collect::<Vec<_>>()
        };
        jobs.into_iter()
            .map(|job| {
                let slot = Arc::clone(&slot);
                std::thread::spawn(move || {
                    let reply = job.run();
                    let mut guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
                    match &mut *guard {
                        Slot::Ready(session) => {
                            session.finish_compaction(job, reply, Instant::now());
                        }
                        Slot::Dormant(_) | Slot::Closed(_) => job.abandon(reply),
                    }
                })
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "workspace_compact_tests.rs"]
mod workspace_compact_tests;
