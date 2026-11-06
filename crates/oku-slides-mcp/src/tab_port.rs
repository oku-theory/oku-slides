//! The host actions one tab's tools take on another tab: prefill, page, fine-tune.
//!
//! The request passed by `open_pack` or `open_theme` only waits in the opened tab's composer,
//! for the user to review and send. A Tune turn starts only from the host (agent rule t-692):
//! the `import` tool asks it through this port once the tab is open, and never sends the
//! import instructions itself.

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, Snafu};

/// Implement this trait to let a tab's agent act on another tab through the host.
///
/// The host never starts a tab's agent nor sends text on a draft's behalf, but for the Tune turn.
pub trait TabPort: Send + Sync + std::fmt::Debug {
    /// Sets `tab`'s draft to `text` unless it already holds something.
    ///
    /// Blocking (it may start the tab's entry): call it off the async threads.
    fn prefill(&self, tab: &str, text: String) -> Prefill;

    /// Shows slide `slide` on tab `tab`'s pages, as that tab's own `focus` does; true if sent.
    ///
    /// Blocking, as [`TabPort::prefill`]. False when no page shows the tab.
    fn page(&self, tab: &str, slide: String) -> bool;

    /// Starts the fine-tuning turn of the imported tab `tab`.
    ///
    /// The host starts that tab's agent on its Tune turn; an unusable agent is told in that tab.
    /// Blocking (it may start the tab's entry and read the disk): call it off the async threads.
    ///
    /// # Errors
    ///
    /// `Unavailable` once the host's agent stack is gone; `NotStarted` when no turn started.
    fn tune(&self, tab: &str) -> Result<(), TuneRefusal>;
}

/// What a [`TabPort::prefill`] did to the target tab's draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prefill {
    /// The draft was empty and now holds the request.
    Set,
    /// The draft already held text or images: it is kept as is.
    Kept,
    /// No such tab, or no port wired: nothing was set.
    Unavailable,
}

/// Why the host started no fine-tuning turn (opaque Snafu with a Fluent key).
#[derive(Debug, Snafu)]
pub struct TuneRefusal(TuneRefusalKind);

/// The refusals of [`TabPort::tune`].
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

/// A port without other tabs, for tests that never prefill, page or tune one.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Default)]
pub struct NoTabs;

#[cfg(any(test, feature = "test-support"))]
impl TabPort for NoTabs {
    fn prefill(&self, _: &str, _: String) -> Prefill {
        Prefill::Unavailable
    }

    fn page(&self, _: &str, _: String) -> bool {
        false
    }

    fn tune(&self, _: &str) -> Result<(), TuneRefusal> {
        Err(TuneRefusal::unavailable())
    }
}

#[cfg(test)]
#[path = "tab_port_tests.rs"]
mod tab_port_tests;
