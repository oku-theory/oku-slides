//! What the agent may ask of the app itself: updates, and quitting once a turn ends.
//!
//! The app answers from its updater and its quit path; the dev server, which is no app,
//! answers [`UpdateState::Unavailable`] and refuses the rest ([`UnavailableApp`]).

use async_trait::async_trait;
use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, Snafu};

/// What an `update` call asks the host, besides installing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateAction {
    /// Check for a newer version now, forgetting a version put off.
    Check,
    /// Put the current offer off, as the "Later" button does.
    Later,
}

/// The app's update state, as the host tells it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateState {
    /// No newer version.
    UpToDate,
    /// A newer version; `installable`: the app can install it itself and restart.
    Offer {
        /// The offered version.
        version: String,
        /// The app installs it (false: the user downloads it from the releases page).
        installable: bool,
    },
    /// This host has no updater: the dev server, or a debug build.
    Unavailable,
}

/// Implement this trait to let agent tools ask the host for app-level actions.
///
/// `quit` and `install` run on the stack's after-turn thread, never inside a tool, so
/// they may block and may stop the stack itself.
#[async_trait]
pub trait AppPort: Send + Sync + std::fmt::Debug {
    /// Checks for an update, or puts the current offer off; the state that follows.
    ///
    /// # Errors
    ///
    /// Refused while closing or without an offer to put off; a failed check is `failed`.
    async fn update(&self, action: UpdateAction) -> Result<UpdateState, AppRefusal>;

    /// Whether this host can quit now: asked before the user confirms an agent's quit.
    ///
    /// # Errors
    ///
    /// Refused when this host cannot quit (the dev server), or is closing already.
    fn can_quit(&self) -> Result<(), AppRefusal>;

    /// Quits once the turn has ended; never called inside a tool.
    ///
    /// # Errors
    ///
    /// Refused when this host cannot quit, or is closing already.
    fn quit(&self) -> Result<(), AppRefusal>;

    /// Installs and restarts once the turn has ended; never called inside a tool.
    ///
    /// # Errors
    ///
    /// Refused without an installable offer, when this host cannot install, or while closing.
    fn install(&self) -> Result<(), AppRefusal>;
}

/// Why the host refused (opaque Snafu with a Fluent key: unavailable, closing, no offer, failed).
#[derive(Debug, Snafu)]
pub struct AppRefusal(AppRefusalKind);

/// The refusals of an [`AppPort`], each with its Fluent key.
#[derive(Debug, Snafu)]
enum AppRefusalKind {
    #[snafu(display("not available in this host"))]
    Unavailable { backtrace: Backtrace },
    #[snafu(display("the app is closing"))]
    Closing { backtrace: Backtrace },
    #[snafu(display("no update offer to put off or install"))]
    NoOffer { backtrace: Backtrace },
    #[snafu(display("the updater failed"))]
    Failed { backtrace: Backtrace },
}

impl AppRefusal {
    /// This host cannot do it: the dev server, or a debug build for updates.
    #[must_use]
    pub fn unavailable() -> Self {
        Self(UnavailableSnafu.build())
    }

    /// The app is closing already.
    #[must_use]
    pub fn closing() -> Self {
        Self(ClosingSnafu.build())
    }

    /// No current offer to put off, or none the app can install.
    #[must_use]
    pub fn no_offer() -> Self {
        Self(NoOfferSnafu.build())
    }

    /// The updater failed: no connection, a bad download; the host logged why.
    #[must_use]
    pub fn failed() -> Self {
        Self(FailedSnafu.build())
    }

    /// The Fluent key of the refusal's message.
    #[must_use]
    pub fn key(&self) -> &'static str {
        match self.0 {
            AppRefusalKind::Unavailable { .. } => "agent-error-app-unavailable",
            AppRefusalKind::Closing { .. } => "agent-error-app-closing",
            AppRefusalKind::NoOffer { .. } => "agent-error-app-no-offer",
            AppRefusalKind::Failed { .. } => "agent-error-app-failed",
        }
    }
}

impl Localize for AppRefusal {
    fn localize(&self, l: &Localizer) -> String {
        l.tr(self.key())
    }
}

/// The dev server's port: no updater, no quit.
#[derive(Debug, Default)]
pub struct UnavailableApp;

#[async_trait]
impl AppPort for UnavailableApp {
    async fn update(&self, _action: UpdateAction) -> Result<UpdateState, AppRefusal> {
        Ok(UpdateState::Unavailable)
    }

    fn can_quit(&self) -> Result<(), AppRefusal> {
        Err(AppRefusal::unavailable())
    }

    fn quit(&self) -> Result<(), AppRefusal> {
        Err(AppRefusal::unavailable())
    }

    fn install(&self) -> Result<(), AppRefusal> {
        Err(AppRefusal::unavailable())
    }
}

#[cfg(test)]
#[path = "app_port_tests.rs"]
mod app_port_tests;
