//! Mémoire d'un processus : l'empreinte physique sur macOS (`footprint`), la
//! mémoire résidente ailleurs (sysinfo).

use sysinfo::System;

/// Mémoire de `pid` en octets ; `None` si le processus n'existe pas.
///
/// macOS : `phys_footprint` (mémoire compressée comprise), plutôt que la
/// mémoire résidente, qui baisse quand le noyau compresse.
#[cfg(target_os = "macos")]
pub(crate) fn memory_bytes(_sys: &mut System, pid: u32) -> Option<u64> {
    crate::footprint::phys_footprint(pid)
}

/// Mémoire de `pid` en octets ; `None` si le processus n'existe pas.
///
/// Mémoire résidente (sysinfo).
#[cfg(not(target_os = "macos"))]
pub(crate) fn memory_bytes(sys: &mut System, pid: u32) -> Option<u64> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate};
    let pid = Pid::from_u32(pid);
    sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, ProcessRefreshKind::nothing().with_memory());
    sys.process(pid).map(sysinfo::Process::memory)
}

#[cfg(test)]
#[path = "rss_tests.rs"]
mod rss_tests;
