//! A fake app port: a set update state, and the actions it was asked.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use oku_slides_session::agent::{AppPort, AppRefusal, UpdateAction, UpdateState};

use super::lock;

/// An app whose update state the test sets; it counts quits and installs.
#[derive(Debug)]
pub struct FakeApp {
    /// What `update` answers.
    pub state: Mutex<UpdateState>,
    /// The update actions asked, in order.
    pub asked: Mutex<Vec<UpdateAction>>,
    installs: AtomicUsize,
    quits: AtomicUsize,
}

impl Default for FakeApp {
    fn default() -> Self {
        Self {
            state: Mutex::new(UpdateState::UpToDate),
            asked: Mutex::new(Vec::new()),
            installs: AtomicUsize::new(0),
            quits: AtomicUsize::new(0),
        }
    }
}

impl FakeApp {
    /// An installable offer of `version` from now on.
    pub fn offer(&self, version: &str) {
        *lock(&self.state) = UpdateState::Offer { version: version.to_owned(), installable: true };
    }

    /// How many times `install` ran.
    pub fn installs(&self) -> usize {
        self.installs.load(Ordering::SeqCst)
    }

    /// How many times `quit` ran.
    pub fn quits(&self) -> usize {
        self.quits.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl AppPort for FakeApp {
    async fn update(&self, action: UpdateAction) -> Result<UpdateState, AppRefusal> {
        lock(&self.asked).push(action);
        let state = lock(&self.state).clone();
        match (action, &state) {
            (UpdateAction::Later, UpdateState::Offer { .. }) | (UpdateAction::Check, _) => Ok(state),
            (UpdateAction::Later, _) => Err(AppRefusal::no_offer()),
        }
    }

    fn can_quit(&self) -> Result<(), AppRefusal> {
        Ok(())
    }

    fn quit(&self) -> Result<(), AppRefusal> {
        self.quits.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn install(&self) -> Result<(), AppRefusal> {
        self.installs.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
