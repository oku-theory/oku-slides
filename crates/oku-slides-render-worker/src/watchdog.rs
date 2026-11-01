//! Render worker watchdog: the worker stops itself when its host no longer can.
//!
//! The host kills the worker on timeout and memory; but if the host dies
//! during a request, nobody will. A worker thread therefore ends the process
//! (code [`EXIT_CODE`], reason on stderr) when:
//! - the current request runs for more than twice the host's timeout;
//! - during a request, its resident memory stays over the host's limit for
//!   more than [`MEMORY_GRACE`] (the host, sampling every 100 ms, stops it
//!   first as `OutOfMemory` while it lives);
//! - its parent is gone (Unix: `getppid` changes; elsewhere, the parent's pid
//!   is free or held by another process, told apart by its start time:
//!   `ProcessIdentity`).
//!
//! The thread runs only in a child process (`exit_if_worker`), never
//! `InProcess`: it would stop the whole host process.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use sysinfo::System;

use crate::rss::memory_bytes;

/// Code de sortie d'un travailleur arrêté par son chien de garde.
pub(crate) const EXIT_CODE: i32 = 3;

/// Intervalle des relevés du chien de garde.
const TICK: Duration = Duration::from_millis(200);

/// Durée d'un dépassement de mémoire avant l'arrêt : l'hôte vivant a le temps
/// de tuer le travailleur lui-même, et d'en dire la cause.
pub(crate) const MEMORY_GRACE: Duration = Duration::from_secs(1);

/// Début de la requête en cours, s'il y en a une.
#[derive(Debug, Default)]
pub(crate) struct Busy(Mutex<Option<Instant>>);

impl Busy {
    /// Une requête commence à `at`.
    pub(crate) fn start(&self, at: Instant) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(at);
    }

    /// La requête en cours a répondu.
    pub(crate) fn end(&self) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }

    /// Durée de la requête en cours à `now` ; `None` au repos.
    pub(crate) fn busy_for(&self, now: Instant) -> Option<Duration> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).map(|at| now.saturating_duration_since(at))
    }
}

/// Bornes du chien de garde ; `None` : pas de borne.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Limits {
    /// Durée maximale d'une requête (deux fois le délai de l'hôte).
    pub(crate) request: Option<Duration>,
    /// Mémoire résidente maximale, en octets.
    pub(crate) memory: Option<u64>,
}

impl Limits {
    /// Les bornes d'un travailleur dont l'hôte a ce délai et cette mémoire.
    pub(crate) fn from_host(timeout_ms: Option<u64>, memory_bytes: Option<u64>) -> Self {
        Self { request: timeout_ms.map(|ms| Duration::from_millis(ms).saturating_mul(2)), memory: memory_bytes }
    }
}

/// Un relevé du chien de garde.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Sample {
    /// Instant du relevé.
    pub(crate) at: Instant,
    /// Durée de la requête en cours ; `None` au repos.
    pub(crate) busy_for: Option<Duration>,
    /// Mémoire résidente du travailleur, si elle se lit.
    pub(crate) rss: Option<u64>,
    /// Le processus parent a disparu.
    pub(crate) parent_gone: bool,
}

/// Les bornes et ce que les relevés précédents ont vu.
#[derive(Debug)]
pub(crate) struct Guard {
    limits: Limits,
    /// Premier relevé d'un dépassement de mémoire en cours.
    memory_over_since: Option<Instant>,
}

impl Guard {
    pub(crate) fn new(limits: Limits) -> Self {
        Self { limits, memory_over_since: None }
    }

    /// La raison d'arrêter le travailleur d'après ce relevé, s'il y en a une.
    pub(crate) fn check(&mut self, sample: &Sample) -> Option<String> {
        if sample.parent_gone {
            return Some("parent process is gone".to_owned());
        }
        if let (Some(busy), Some(max)) = (sample.busy_for, self.limits.request)
            && busy > max
        {
            return Some(format!("request running for {} ms, over {} ms", busy.as_millis(), max.as_millis()));
        }
        // Au repos, la mémoire ne compte pas : l'allocateur garde les pages
        // d'une grosse requête, et rien ne tourne (t-340).
        match (sample.busy_for, sample.rss, self.limits.memory) {
            (Some(_), Some(rss), Some(max)) if rss > max => {
                let since = *self.memory_over_since.get_or_insert(sample.at);
                let over = sample.at.saturating_duration_since(since);
                (over >= MEMORY_GRACE).then(|| format!("memory {rss} bytes, over {max} for {} ms", over.as_millis()))
            }
            _ => {
                self.memory_over_since = None;
                None
            }
        }
    }
}

/// Relève `sample` toutes les `tick` jusqu'à une raison d'arrêt, qu'il rend.
pub(crate) fn watch(limits: Limits, tick: Duration, mut sample: impl FnMut() -> Sample) -> String {
    let mut guard = Guard::new(limits);
    loop {
        if let Some(reason) = guard.check(&sample()) {
            return reason;
        }
        std::thread::sleep(tick);
    }
}

/// Le parent `now` n'est plus celui du lancement, `at_launch`.
#[cfg(unix)]
pub(crate) fn parent_changed(at_launch: u32, now: u32) -> bool {
    // Rattaché à un autre processus (init ou un sous-moissonneur) : le parent
    // a disparu. Un hôte qui est lui-même PID 1 (conteneur) reste à 1.
    now != at_launch
}

/// Le parent du processus tel qu'il était au lancement.
struct Parent {
    #[cfg(unix)]
    pid: u32,
    /// `None`: the parent was already gone at launch, or could not be opened.
    #[cfg(not(unix))]
    identity: Option<ProcessIdentity>,
}

impl Parent {
    fn current() -> Self {
        #[cfg(unix)]
        {
            Self { pid: std::os::unix::process::parent_id() }
        }
        #[cfg(not(unix))]
        {
            Self { identity: ProcessIdentity::parent_of(std::process::id()) }
        }
    }

    fn gone(&self) -> bool {
        #[cfg(unix)]
        {
            parent_changed(self.pid, std::os::unix::process::parent_id())
        }
        #[cfg(not(unix))]
        {
            self.identity.is_none_or(|parent| !parent.alive())
        }
    }
}

/// A process told apart from a later one reusing its pid, by its start time.
///
/// Without `getppid` (Windows), a parent that died leaves its pid free: a pid
/// that still answers may be another process (t-341, t-1471). Start times
/// are in seconds: a reuse within the parent's own start second goes unseen.
#[cfg(not(unix))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessIdentity {
    pid: u32,
    /// Seconds since the epoch, never 0.
    start_time: u64,
}

#[cfg(not(unix))]
impl ProcessIdentity {
    /// The process holding `pid` now, if any and if it can be opened.
    pub(crate) fn read(pid: u32) -> Option<Self> {
        Self::read_with_parent(pid).map(|(identity, _)| identity)
    }

    /// The parent of `pid`, if it still runs and started no later than `pid`.
    ///
    /// A parent pid held by a younger process was reused: the parent is gone.
    pub(crate) fn parent_of(pid: u32) -> Option<Self> {
        let (child, parent) = Self::read_with_parent(pid)?;
        Self::read(parent?).filter(|parent| parent.start_time <= child.start_time)
    }

    /// This process still runs: its pid is neither free nor reused.
    pub(crate) fn alive(self) -> bool {
        Self::read(self.pid) == Some(self)
    }

    /// The process holding `pid` and its parent's pid.
    fn read_with_parent(pid: u32) -> Option<(Self, Option<u32>)> {
        use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate};
        // A fresh `System` each time: a cached entry would hold the parent's
        // handle open and keep its stale start time.
        let mut sys = System::new();
        let key = Pid::from_u32(pid);
        sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[key]), true, ProcessRefreshKind::nothing());
        let process = sys.process(key)?;
        // sysinfo gives 0 for a process it cannot open: no identity to compare.
        let start_time = Some(process.start_time()).filter(|&t| t != 0)?;
        Some((Self { pid, start_time }, process.parent().map(Pid::as_u32)))
    }
}

/// Lance le fil de garde : il termine le processus quand `watch` rend une raison.
pub(crate) fn spawn(limits: Limits, busy: Arc<Busy>) {
    let started = std::thread::Builder::new().name("oku-render-watchdog".into()).spawn(move || {
        let mut sys = System::new();
        let me = std::process::id();
        let parent = Parent::current();
        let reason = watch(limits, TICK, || {
            let at = Instant::now();
            Sample { at, busy_for: busy.busy_for(at), rss: memory_bytes(&mut sys, me), parent_gone: parent.gone() }
        });
        eprintln!("oku-slides render worker stopped by its watchdog: {reason}");
        std::process::exit(EXIT_CODE);
    });
    if let Err(e) = started {
        tracing::error!(error = %e, "chien de garde du processus de rendu non lancé");
    }
}

#[cfg(test)]
#[path = "watchdog_tests.rs"]
mod watchdog_tests;
