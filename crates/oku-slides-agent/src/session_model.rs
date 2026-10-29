//! The model of ACP sessions: what the agent offers and what each live session uses.
//!
//! An agent tells its models in a session config option of category `model` (in the
//! `session/new`, `session/load` and `session/resume` replies, `session/set_config_option`
//! and `config_option_update`); the app asks for its chosen one with `session/set_config_option`.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use agent_client_protocol::schema::v1::{
    SessionConfigId, SessionConfigKind, SessionConfigOption, SessionConfigOptionCategory, SessionConfigSelectOption,
    SessionConfigSelectOptions, SessionId,
};
use oku_slides_session::{AgentModelView, AgentModelsView};
use tokio::sync::watch;

/// A session's model option: its id, the value in use and the offered values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelOptions {
    /// The option id, sent back with `session/set_config_option`.
    pub config_id: SessionConfigId,
    /// The model the session uses.
    pub current: String,
    /// The offered models, in the agent's order (groups flattened).
    pub values: Vec<ModelValue>,
}

/// A model the agent offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelValue {
    /// Its value id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its description, if any.
    pub description: Option<String>,
}

impl ModelOptions {
    /// The model `id` is offered.
    pub(crate) fn offers(&self, id: &str) -> bool {
        self.values.iter().any(|v| v.id == id)
    }
}

/// The model option among `options`: the first select of category `model`.
///
/// Without any category, a select whose id is `model` counts too.
pub(crate) fn model_options(options: &[SessionConfigOption]) -> Option<ModelOptions> {
    let is_model = |o: &&SessionConfigOption| match &o.category {
        Some(category) => *category == SessionConfigOptionCategory::Model,
        None => o.id.0.as_ref() == "model",
    };
    options.iter().filter(is_model).find_map(|o| {
        let SessionConfigKind::Select(select) = &o.kind else { return None };
        let value = |v: &SessionConfigSelectOption| ModelValue {
            id: v.value.0.to_string(),
            name: v.name.clone(),
            description: v.description.clone(),
        };
        let values = match &select.options {
            SessionConfigSelectOptions::Ungrouped(values) => values.iter().map(value).collect(),
            SessionConfigSelectOptions::Grouped(groups) => {
                groups.iter().flat_map(|g| g.options.iter().map(value)).collect()
            }
            // A future shape of the protocol: no model choice offered.
            _ => return None,
        };
        Some(ModelOptions { config_id: o.id.clone(), current: select.current_value.0.to_string(), values })
    })
}

/// What a launch of the agent told about models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Offered {
    /// No session of this launch epoch has answered yet.
    Unknown,
    /// A session answered without a model option: no choice.
    NoChoice,
    /// The last session that told its models.
    Models(ModelOptions),
}

impl Offered {
    /// What the box shows, with the model the app asks for; `None` without a choice.
    pub(crate) fn view(&self, chosen: Option<String>) -> Option<AgentModelsView> {
        match self {
            Self::Unknown => Some(AgentModelsView::Unknown { chosen }),
            Self::NoChoice => None,
            Self::Models(o) => Some(AgentModelsView::Offered {
                models: o
                    .values
                    .iter()
                    .map(|v| AgentModelView { id: v.id.clone(), name: v.name.clone(), hint: v.description.clone() })
                    .collect(),
                current: o.current.clone(),
                chosen,
            }),
        }
    }
}

/// The model options of live sessions, by launch and session, and the last one told.
#[derive(Debug, Default)]
struct Models {
    sessions: HashMap<(u64, SessionId), ModelOptions>,
    /// The launch epoch of `last`.
    epoch: u64,
    last: Option<Offered>,
}

/// Shared by the connection and the tab actors; its revision advances when `offered` changes.
#[derive(Debug)]
pub(crate) struct ModelBoard {
    state: Mutex<Models>,
    revision: watch::Sender<u64>,
}

impl Default for ModelBoard {
    fn default() -> Self {
        Self { state: Mutex::default(), revision: watch::Sender::new(0) }
    }
}

impl ModelBoard {
    /// Session `id` of launch `generation` (epoch `epoch`) told `options`.
    ///
    /// A late answer from an older epoch (agent switched since) updates only its session.
    pub(crate) fn told(&self, generation: u64, epoch: u64, id: &SessionId, options: Option<ModelOptions>) {
        let changed = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let key = (generation, id.clone());
            let offered = if let Some(o) = options {
                state.sessions.insert(key, o.clone());
                Offered::Models(o)
            } else {
                state.sessions.remove(&key);
                Offered::NoChoice
            };
            if epoch < state.epoch {
                return;
            }
            let changed = state.epoch != epoch || state.last.as_ref() != Some(&offered);
            state.epoch = epoch;
            state.last = Some(offered);
            changed
        };
        if changed {
            self.revision.send_modify(|r| *r = r.wrapping_add(1));
        }
    }

    /// The model option of session `id` of launch `generation`, if known.
    pub(crate) fn session(&self, generation: u64, id: &SessionId) -> Option<ModelOptions> {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.sessions.get(&(generation, id.clone())).cloned()
    }

    /// What launch epoch `epoch` told about models.
    pub(crate) fn offered(&self, epoch: u64) -> Offered {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        match &state.last {
            Some(offered) if state.epoch == epoch => offered.clone(),
            Some(_) | None => Offered::Unknown,
        }
    }

    /// Forgets session `id` of launch `generation`.
    pub(crate) fn forget(&self, generation: u64, id: &SessionId) {
        self.state.lock().unwrap_or_else(PoisonError::into_inner).sessions.remove(&(generation, id.clone()));
    }

    /// Forgets every session of launch `generation` (its connection ended).
    pub(crate) fn forget_generation(&self, generation: u64) {
        self.state.lock().unwrap_or_else(PoisonError::into_inner).sessions.retain(|(g, _), _| *g != generation);
    }

    /// Advances each time what the box shows of models changes.
    pub(crate) fn subscribe(&self) -> watch::Receiver<u64> {
        self.revision.subscribe()
    }
}

#[cfg(test)]
#[path = "session_model_tests.rs"]
mod session_model_tests;
