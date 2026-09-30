//! Chien de garde du processus de rendu : il s'arrête seul si son hôte ne
//! peut plus le faire.
//!
//! L'hôte tue le travailleur au délai et à la mémoire ; mais s'il meurt
//! pendant une requête, personne ne le fera. Un fil du travailleur termine
//! donc le processus (code [`EXIT_CODE`], raison sur stderr) si :
//! - la requête en cours dure plus de deux fois le délai de l'hôte ;
//! - sa mémoire résidente dépasse la limite de l'hôte depuis plus de
//!   [`MEMORY_GRACE`] (l'hôte, qui relève toutes les 100 ms, l'arrête d'abord
//!   en `OutOfMemory` s'il est là) ;
//! - son parent a disparu (Unix : `getppid` change ou vaut 1 ; ailleurs,
//!   le processus parent n'existe plus).
//!
//! Le fil ne sert qu'en sous-processus (`exit_if_worker`), jamais en
//! `InProcess` : il arrêterait tout le processus hôte.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use sysinfo::System;

use crate::rss::resident_bytes;

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
        match (sample.rss, self.limits.memory) {
            (Some(rss), Some(max)) if rss > max => {
                let since = *self.memory_over_since.get_or_insert(sample.at);
                let over = sample.at.saturating_duration_since(since);
                (over >= MEMORY_GRACE)
                    .then(|| format!("resident memory {rss} bytes, over {max} for {} ms", over.as_millis()))
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

/// Le parent du processus tel qu'il était au lancement.
struct Parent {
    #[cfg(unix)]
    pid: u32,
    #[cfg(not(unix))]
    pid: Option<u32>,
}

impl Parent {
    fn current(_sys: &mut System) -> Self {
        #[cfg(unix)]
        {
            Self { pid: std::os::unix::process::parent_id() }
        }
        #[cfg(not(unix))]
        {
            let me = sysinfo::Pid::from_u32(std::process::id());
            _sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[me]), true);
            Self { pid: _sys.process(me).and_then(sysinfo::Process::parent).map(sysinfo::Pid::as_u32) }
        }
    }

    fn gone(&self, _sys: &mut System) -> bool {
        #[cfg(unix)]
        {
            let now = std::os::unix::process::parent_id();
            now != self.pid || now == 1
        }
        #[cfg(not(unix))]
        {
            self.pid.is_some_and(|pid| resident_bytes(_sys, pid).is_none())
        }
    }
}

/// Lance le fil de garde : il termine le processus quand `watch` rend une raison.
pub(crate) fn spawn(limits: Limits, busy: Arc<Busy>) {
    let started = std::thread::Builder::new().name("oku-render-watchdog".into()).spawn(move || {
        let mut sys = System::new();
        let me = std::process::id();
        let parent = Parent::current(&mut sys);
        let reason = watch(limits, TICK, || {
            let at = Instant::now();
            Sample {
                at,
                busy_for: busy.busy_for(at),
                rss: resident_bytes(&mut sys, me),
                parent_gone: parent.gone(&mut sys),
            }
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
