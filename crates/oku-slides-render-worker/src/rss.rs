//! Mémoire résidente d'un processus (sysinfo, mêmes appels sur les trois OS).

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// Mémoire résidente de `pid` en octets ; `None` si le processus n'existe pas.
pub(crate) fn resident_bytes(sys: &mut System, pid: u32) -> Option<u64> {
    let pid = Pid::from_u32(pid);
    sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, ProcessRefreshKind::nothing().with_memory());
    sys.process(pid).map(sysinfo::Process::memory)
}

#[cfg(test)]
#[path = "rss_tests.rs"]
mod rss_tests;
