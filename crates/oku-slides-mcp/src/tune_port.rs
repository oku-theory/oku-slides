//! The fine-tuning turn an agent's import asks for, started by the host in the imported tab.
//!
//! A Tune turn starts only from the host (agent rule t-692): the `import` tool asks it through
//! this port once the tab is open, and never sends the import instructions itself.

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, Snafu};

/// Implement this trait to let an agent's import start the imported tab's fine-tuning turn.
///
/// The host starts that tab's agent on its Tune turn; an unusable agent is told in that tab.
pub trait TunePort: Send + Sync + std::fmt::Debug {
    /// Starts the fine-tuning turn of the imported tab `tab`.
    ///
    /// Blocking (it may start the tab's entry and read the disk): call it off the async threads.
    ///
    /// # Errors
    ///
    /// `Unavailable` once the host's agent stack is gone; `NotStarted` when no turn started.
    fn tune(&self, tab: &str) -> Result<(), TuneRefusal>;
}

/// Why the host started no fine-tuning turn (opaque Snafu with a Fluent key).
#[derive(Debug, Snafu)]
pub struct TuneRefusal(TuneRefusalKind);

/// The refusals of a [`TunePort`].
#[derive(Debug, Snafu)]
enum TuneRefusalKind {
    #[snafu(display("no agent stack to start the fine-tuning turn"))]
    Unavailable { backtrace: Backtrace },
    #[snafu(display("the fine-tuning turn did not start"))]
    NotStarted { backtrace: Backtrace },
}

impl TuneRefusal {
    /// The host's agent stack is gone (the app is closing) or was never wired.
    #[must_use]
    pub fn unavailable() -> Self {
        Self(UnavailableSnafu.build())
    }

    /// The tab's agent is unusable for now, or refused the turn; the tab itself is told why.
    #[must_use]
    pub fn not_started() -> Self {
        Self(NotStartedSnafu.build())
    }

    /// The Fluent key of the refusal's message.
    #[must_use]
    pub fn key(&self) -> &'static str {
        match self.0 {
            TuneRefusalKind::Unavailable { .. } => "agent-error-tune-unavailable",
            TuneRefusalKind::NotStarted { .. } => "agent-error-tune-not-started",
        }
    }
}

impl Localize for TuneRefusal {
    fn localize(&self, l: &Localizer) -> String {
        l.tr(self.key())
    }
}

/// A port that starts no fine-tuning, for tests that never ask one.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Default)]
pub struct NoTune;

#[cfg(any(test, feature = "test-support"))]
impl TunePort for NoTune {
    fn tune(&self, _: &str) -> Result<(), TuneRefusal> {
        Err(TuneRefusal::unavailable())
    }
}

#[cfg(test)]
#[path = "tune_port_tests.rs"]
mod tune_port_tests;
