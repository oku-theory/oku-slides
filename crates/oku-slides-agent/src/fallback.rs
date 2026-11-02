//! Falling back to the local agent when the selected one is unusable.
//!
//! At the start of each prompt, before `connect`, [`AppVoice::turn_route`] decides where the
//! turn runs ([`decide`], pure): on the selected agent, on the local one (the link switches to
//! the first built-in preset; `agent.selected` is never written), after a model download, or
//! nowhere. The link switches, either way, only while this tab's prompt is the only one running.

use oku_slides_i18n::Localize;
use oku_slides_session::LocalModelView;

use super::AppVoice;
use crate::error::{Reason, Unusable};

/// Where a turn runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TurnRoute {
    /// On the selected agent (also when it is unusable and nothing can stand in for it).
    Main,
    /// On the local agent, standing in for the selected one unusable for `reason`.
    Local { reason: Reason },
    /// On the local agent once its model (`size` bytes) is downloaded.
    NeedsDownload { reason: Reason, size: u64 },
    /// Nowhere: neither the selected agent (`main`) nor the local one (`local`) can run.
    Unusable { main: Reason, local: Unusable },
}

/// What the link does for a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Move {
    /// It keeps the agent it runs.
    Stay,
    /// It switches to the local agent.
    ToLocal,
    /// It switches back to the selected agent.
    ToMain,
}

/// The local agent, as a fallback for the selected one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Standby {
    /// Nothing can stand in for the selected agent.
    Absent,
    /// The local agent can run.
    Ready,
    /// The local agent cannot run, for this reason.
    Blocked(Unusable),
}

impl From<Option<(String, Option<Unusable>)>> for Standby {
    fn from(check: Option<(String, Option<Unusable>)>) -> Self {
        match check {
            None => Self::Absent,
            Some((_, None)) => Self::Ready,
            Some((_, Some(reason))) => Self::Blocked(reason),
        }
    }
}

/// What a route is decided from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Facts {
    /// Why the selected agent is unusable; `None`: it seems usable.
    pub main: Option<Reason>,
    /// Whether the local agent can stand in for the selected one.
    pub local: Standby,
    /// Why the selected agent failed, while the local agent stands in for it.
    pub standing: Option<Reason>,
    /// This tab's prompt is the only one running: the link may switch.
    pub alone: bool,
}

/// The route of a turn and what the link does for it.
pub(crate) fn decide(facts: Facts) -> (TurnRoute, Move) {
    let Facts { main, local, standing, alone } = facts;
    let Some(reason) = main else {
        return match standing {
            None => (TurnRoute::Main, Move::Stay),
            Some(_) if alone => (TurnRoute::Main, Move::ToMain),
            // Another tab prompts on the local agent: switching would stop its turn.
            Some(reason) => (TurnRoute::Local { reason }, Move::Stay),
        };
    };
    match local {
        Standby::Ready if standing.is_some() => (TurnRoute::Local { reason }, Move::Stay),
        Standby::Ready if alone => (TurnRoute::Local { reason }, Move::ToLocal),
        // No fallback, the turn fails on the selected agent as without a local agent; or
        // another tab prompts on the selected agent, and the route stays.
        Standby::Absent | Standby::Ready => (TurnRoute::Main, Move::Stay),
        Standby::Blocked(Unusable::LocalNeedsDownload { size }) => {
            (TurnRoute::NeedsDownload { reason, size }, Move::Stay)
        }
        Standby::Blocked(local) => (TurnRoute::Unusable { main: reason, local }, Move::Stay),
    }
}

/// What a route is read from, off the state lock: the selected agent and both availabilities.
#[derive(Debug, Clone)]
pub(crate) struct RouteRead {
    /// The selected agent when the availabilities were read.
    selected: String,
    /// Why it is unusable.
    main: Option<Reason>,
    /// The fallback preset and why it cannot run, read only if the selected agent is unusable.
    local: Option<(String, Option<Unusable>)>,
}

impl AppVoice {
    /// Where the next turn of `tab` runs; switches the link if needed (see the module).
    ///
    /// The availabilities are read first (disk), then decided and applied under the state lock,
    /// where `tab`'s guard is checked to be the only one. May block: reads the disk.
    pub(crate) fn turn_route(&self, tab: &str) -> TurnRoute {
        let read = self.route_read();
        self.route_apply(tab, read)
    }

    /// The availabilities a route is decided from. May block: reads the disk.
    pub(crate) fn route_read(&self) -> RouteRead {
        let selected = self.choice.active();
        let main = self.choice.main_reason();
        let local = main.as_ref().and_then(|_| self.choice.local_check());
        RouteRead { selected, main, local }
    }

    /// Decides the route of `tab` from `read` and applies it, under the state lock.
    ///
    /// Another agent selected since `read`: what was read is of the agent before, the turn
    /// runs on the one selected now (a launch failure there routes it once more).
    pub(crate) fn route_apply(&self, tab: &str, read: RouteRead) -> TurnRoute {
        let RouteRead { selected, main, local } = read;
        let (route, moved) = {
            let state = self.switch.lock();
            if self.choice.active() != selected {
                return TurnRoute::Main;
            }
            let preset = local.as_ref().map(|(preset, _)| preset.clone());
            let local = Standby::from(local);
            let facts = Facts { main, local, standing: self.choice.fallback_reason(), alone: state.alone(tab) };
            let (route, moved) = decide(facts);
            match (&route, moved, preset) {
                (TurnRoute::Local { reason }, Move::ToLocal | Move::Stay, Some(preset)) => {
                    self.choice.fall_back(&preset, reason.clone());
                }
                (_, Move::ToMain, _) => self.choice.back_to_main(),
                _ => {}
            }
            (route, moved)
        };
        if moved != Move::Stay {
            self.bump();
        }
        route
    }

    /// Why the selected agent could not serve, while the local agent stands in for it.
    pub(crate) fn standing_reason(&self) -> Option<Reason> {
        self.choice.fallback_reason()
    }

    /// The selected agent served a turn: the retry backoff of a later fallback starts over.
    pub(crate) fn main_served(&self) {
        self.choice.main_served();
    }

    /// Why no turn can run, localized: neither the selected agent nor the local one can (a
    /// download included). The selected agent's reason alone when nothing can stand in for it.
    ///
    /// May block: reads the disk.
    pub(crate) fn availability(&self) -> Option<String> {
        let main = self.choice.main_reason()?;
        let l = self.localizer();
        match self.choice.local_check() {
            None => Some(main.localize(&l)),
            Some((_, None | Some(Unusable::LocalNeedsDownload { .. }))) => None,
            Some((_, Some(local))) => Some(both_reasons(&main, &local, &l)),
        }
    }

    /// Starts the local model's download for a fallback turn; false if it is not missing or runs.
    ///
    /// Each percent and its end advance the box's revision. May block: reads the disk.
    pub(crate) fn start_fallback_download(&self) -> bool {
        let me = self.me.clone();
        let started = self.choice.start_download(move || {
            if let Some(voice) = me.upgrade() {
                voice.bump();
            }
        });
        if started {
            self.bump();
        }
        started
    }

    /// The local model's download state; `None` once nothing is missing. May block: reads the disk.
    pub(crate) fn local_model(&self) -> Option<LocalModelView> {
        self.choice.local_model()
    }

    /// Stops the local model's download, if one runs.
    pub(crate) fn cancel_local_download(&self) {
        self.choice.cancel_download();
        self.bump();
    }
}

/// Why no agent can run: the selected agent's reason, then the local agent's.
pub(crate) fn both_reasons(main: &Reason, local: &Unusable, l: &oku_slides_i18n::Localizer) -> String {
    l.tr_with("agent-fallback-unusable", &[("main", main.localize(l).into()), ("local", local.localize(l).into())])
}

#[cfg(test)]
#[path = "fallback_tests.rs"]
mod fallback_tests;
