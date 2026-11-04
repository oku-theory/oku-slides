//! A fake app port: a set update state, and the actions it was asked.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use oku_slides_session::agent::{AppPort, AppRefusal, UpdateAction, UpdateState};

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// An app whose update state the test sets; it counts quits and installs.
#[derive(Debug)]
pub struct FakeApp {
    /// What `update` answers.
    pub state: Mutex<UpdateState>,
    /// The update actions asked, in order.
    pub asked: Mutex<Vec<UpdateAction>>,
    installs: AtomicUsize,
    quits: AtomicUsize,
    fail_next: AtomicBool,
}

impl Default for FakeApp {
    fn default() -> Self {
        Self {
            state: Mutex::new(UpdateState::UpToDate),
            asked: Mutex::new(Vec::new()),
            installs: AtomicUsize::new(0),
            quits: AtomicUsize::new(0),
            fail_next: AtomicBool::new(false),
        }
    }
}

impl FakeApp {
    /// An installable offer of `version` from now on.
    pub fn offer(&self, version: &str) {
        *lock(&self.state) = UpdateState::Offer { version: version.to_owned(), installable: true };
    }

    /// The next install is refused as `failed`, uncounted: a visible sentinel in the runner's order.
    pub fn fail_next_install(&self) {
        self.fail_next.store(true, Ordering::SeqCst);
    }

    /// How many times `install` ran (a refused one excluded).
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
        if self.fail_next.swap(false, Ordering::SeqCst) {
            return Err(AppRefusal::failed());
        }
        self.installs.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
