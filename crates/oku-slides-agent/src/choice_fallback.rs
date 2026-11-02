//! The local agent standing in for the selected one: switch, retry backoff, diagnosis.
//!
//! While it stands in, the selected agent's failure kept at the switch makes it unusable for
//! a wait: [`FAILURE_TTL`] at first, doubled by each failed retry up to [`RETRY_MAX`], back to
//! [`FAILURE_TTL`] once the selected agent serves a turn or the user picks an agent.

use std::time::{Duration, Instant};

use oku_slides_i18n::Localize;
use oku_slides_mcp::AgentDiagnosis;

use super::AgentChoice;
use crate::error::{Reason, Unusable};
use crate::link::FAILURE_TTL;

/// The longest wait before the selected agent is retried while the local agent stands in.
pub(crate) const RETRY_MAX: Duration = Duration::from_secs(600);

/// The local agent standing in for the selected one, while the link runs it.
#[derive(Debug, Clone)]
pub(super) struct Fallback {
    /// The fallback preset the link runs (an `[agent.presets]` key).
    pub(super) preset: String,
    /// Why the selected agent could not serve, at the last turn routed.
    pub(super) reason: Reason,
    /// The selected agent's launch failure at the switch (`Link::switch` forgets it), and when.
    pub(super) main_failure: Option<(Unusable, Instant)>,
    /// How long `main_failure` keeps the selected agent unusable.
    pub(super) wait: Duration,
    /// The selected agent's stderr tail at the switch: the link's is the local agent's since.
    pub(super) main_stderr: Vec<String>,
}

/// The wait after a retry of the selected agent waited `retried` failed; `None`: no retry.
pub(crate) fn next_wait(retried: Option<Duration>) -> Duration {
    retried.map_or(FAILURE_TTL, |wait| wait.saturating_mul(2).min(RETRY_MAX))
}

impl AgentChoice {
    /// Why the selected agent could not serve while the local agent stands in for it.
    pub(crate) fn fallback_reason(&self) -> Option<Reason> {
        self.lock_active().fallback.as_ref().map(|f| f.reason.clone())
    }

    /// The link runs the fallback preset `local` for `reason`; `agent.selected` is not touched.
    ///
    /// Already standing in: only the reason is updated. Right after a failed retry of the
    /// selected agent, the wait before the next one doubles. Fast: nothing is written.
    pub(crate) fn fall_back(&self, local: &str, reason: Reason) {
        let mut active = self.lock_active();
        if let Some(fallback) = active.fallback.as_mut() {
            fallback.reason = reason;
            return;
        }
        let Some(preset) = self.platform.settings.agent.preset(local) else {
            tracing::error!(local, "agent local absent des réglages : pas de repli");
            return;
        };
        // Read before the switch, which forgets the failure and begins another stderr tail.
        let (main_failure, main_stderr) = (self.link.failure_at(), self.link.stderr_tail());
        let wait = next_wait(active.retrying.take());
        self.link.switch(local, Self::launch_of(preset, &self.platform), None);
        active.fallback = Some(Fallback { preset: local.to_owned(), reason, main_failure, wait, main_stderr });
        tracing::info!(selected = %active.id, local, wait_s = wait.as_secs(), "repli sur l'agent local");
    }

    /// The link runs the selected agent again; nothing if it already does. Fast: nothing is written.
    ///
    /// A retry: if it fails, the next fallback waits longer before retrying.
    pub(crate) fn back_to_main(&self) {
        let mut active = self.lock_active();
        let Some(fallback) = active.fallback.take() else { return };
        let Some(preset) = self.platform.settings.agent.preset(&active.id) else {
            tracing::error!(selected = %active.id, "agent choisi absent des réglages : repli gardé");
            active.fallback = Some(fallback);
            return;
        };
        let model = preset.builtin.is_none().then(|| self.chosen_model(&active.id)).flatten();
        self.link.switch(&active.id, Self::launch_of(preset, &self.platform), model);
        active.retrying = Some(fallback.wait);
        tracing::info!(selected = %active.id, local = %fallback.preset, "retour à l'agent choisi");
    }

    /// The selected agent served a turn: a later fallback waits [`FAILURE_TTL`] again.
    pub(crate) fn main_served(&self) {
        self.lock_active().retrying = None;
    }

    /// The selected agent for the `diagnose` tool: preset, availability, stderr tail.
    ///
    /// While the local agent stands in, the selected agent's reason and the stderr tail kept at
    /// the switch. May block: reads the disk (see [`AgentChoice::availability`]).
    pub(crate) fn diagnosis(&self) -> AgentDiagnosis {
        let (selected, standing) = {
            let a = self.lock_active();
            (a.id.clone(), a.fallback.as_ref().map(|f| (f.reason.clone(), f.main_stderr.clone())))
        };
        let (unusable, stderr) = match standing {
            Some((reason, stderr)) => (Some(reason.localize(&self.localizer())), stderr),
            None => (self.availability(), self.link.stderr_tail()),
        };
        let preset = self.platform.settings.agent.preset(&selected);
        AgentDiagnosis {
            name: preset.map(|p| p.name.clone()).unwrap_or_default(),
            builtin: preset.is_some_and(|p| p.builtin.is_some()),
            unusable,
            stderr,
            selected,
        }
    }
}
