//! The app's [`AppPort`]: the agent checks, puts off, installs and quits like the windows do.
//!
//! `update` reuses the menu's check and the "Later" button ([`crate::updates`]); quitting and
//! installing run on the agent stack's after-turn thread, once a turn has ended, through
//! `quit::exit` and `updates::install`, so the windows' close guards run first. It holds an
//! `AppHandle` (Tauri has no weak one) and reaches `AppState` through `try_state`: the cycle
//! ends at exit, as `AppState`'s does. A debug build has no updater: `Unavailable`.

use std::sync::atomic::Ordering;

use async_trait::async_trait;
use oku_slides_platform::errors::one_line;
use oku_slides_platform::updates::{InstallMode, UpdateStage, UpdateStatus};
use oku_slides_session::agent::{AppPort, AppRefusal, UpdateAction, UpdateState};
use tauri::{AppHandle, Manager};

use crate::state::AppState;
use crate::updates::{self, Updates};

/// The agent's door to the app's updater and quit path.
pub struct TauriApp {
    app: AppHandle,
}

impl std::fmt::Debug for TauriApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TauriApp").finish_non_exhaustive()
    }
}

impl TauriApp {
    /// The port of the app behind `app`.
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    fn closing(&self) -> bool {
        self.app.try_state::<AppState>().is_some_and(|s| s.closing.load(Ordering::SeqCst))
    }
}

/// An app action the agent asks, as the one gate in front of it sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Asked {
    /// Check or put off: a debug build answers `Unavailable`.
    Update,
    /// Quit: any build may.
    Quit,
    /// Install: a debug build refuses.
    Install,
}

/// What the gate lets through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Gate {
    /// Go ahead.
    Open,
    /// This build has no updater.
    NoUpdater,
    /// The app is closing: nothing more.
    Closing,
}

/// The one gate of every app action: closing first, then a debug build for the updater's.
fn gate(asked: Asked, closing: bool, debug_build: bool) -> Gate {
    if closing {
        Gate::Closing
    } else if debug_build && asked != Asked::Quit {
        Gate::NoUpdater
    } else {
        Gate::Open
    }
}

/// The agent's view of `status`: an offer is installable when the app installs it, none running.
fn state_of(status: &UpdateStatus) -> UpdateState {
    match &status.offer {
        None => UpdateState::UpToDate,
        Some(offer) => UpdateState::Offer {
            version: offer.version.clone(),
            installable: offer.mode == InstallMode::Install && !status.installing,
        },
    }
}

/// An updater failure at `stage`, as the agent's refusal.
fn refusal_of(stage: UpdateStage) -> AppRefusal {
    match stage {
        UpdateStage::Unavailable => AppRefusal::no_offer(),
        // Refused here means closing: no window asked, so no window can lack the right.
        UpdateStage::Refused => AppRefusal::closing(),
        // Check, Download, Open; Busy too: another window's install runs, may still fail, and the
        // agent's ask did not go through. `UpdateStage` is non-exhaustive: a later stage is a failure.
        _ => AppRefusal::failed(),
    }
}

/// Logs `e` and turns it into the agent's refusal.
fn refuse(e: &updates::UpdateError) -> AppRefusal {
    tracing::warn!(error = %one_line(e), "update asked by the agent refused");
    refusal_of(updates::stage(e))
}

#[async_trait]
impl AppPort for TauriApp {
    async fn update(&self, action: UpdateAction) -> Result<UpdateState, AppRefusal> {
        match gate(Asked::Update, self.closing(), cfg!(debug_assertions)) {
            Gate::Closing => return Err(AppRefusal::closing()),
            Gate::NoUpdater => return Ok(UpdateState::Unavailable),
            Gate::Open => {}
        }
        match action {
            UpdateAction::Check => updates::check_asked(&self.app).await.map(|s| state_of(&s)).map_err(|e| refuse(&e)),
            UpdateAction::Later => {
                let Some(updates) = self.app.try_state::<Updates>() else { return Err(AppRefusal::closing()) };
                let Some(offer) = updates.status().offer else { return Err(AppRefusal::no_offer()) };
                Ok(state_of(&updates::put_off(&self.app, &updates, offer.version).await))
            }
        }
    }

    fn can_quit(&self) -> Result<(), AppRefusal> {
        match gate(Asked::Quit, self.closing(), cfg!(debug_assertions)) {
            Gate::Closing => Err(AppRefusal::closing()),
            Gate::NoUpdater => Err(AppRefusal::unavailable()),
            Gate::Open => Ok(()),
        }
    }

    fn quit(&self) -> Result<(), AppRefusal> {
        self.can_quit()?;
        // Returns at once: the close guards and the shutdown run on the async runtime.
        crate::quit::exit(&self.app);
        Ok(())
    }

    fn install(&self) -> Result<(), AppRefusal> {
        match gate(Asked::Install, self.closing(), cfg!(debug_assertions)) {
            Gate::Closing => return Err(AppRefusal::closing()),
            Gate::NoUpdater => return Err(AppRefusal::unavailable()),
            Gate::Open => {}
        }
        // On the after-turn thread, outside the runtime: `block_on` is allowed, as in `state::settle`.
        tauri::async_runtime::block_on(updates::install(&self.app)).map_err(|e| refuse(&e))
    }
}

#[cfg(test)]
#[path = "app_port_tests.rs"]
mod app_port_tests;
