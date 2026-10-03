//! Agent vocal : Oku Slides lance un agent ACP en sous-processus (stdio),
//! lui injecte ses outils MCP (HTTP local, jeton) et lui relaie les
//! consignes dictées avec le contexte de l'interface.
//!
//! - [`Link`] : la connexion ACP partagée (un sous-processus pour l'app,
//!   lancé au premier prompt, relancé après un arrêt).
//! - [`AgentHandle`] : l'agent d'un onglet (sa session ACP, ses tours).
//! - [`AgentEvent`] : ce que l'interface affiche dans le fil.
//! - [`Stack`] : un agent par onglet (outils MCP, fil gardé) pour l'hôte ;
//!   évènements relayés en enveloppes `{tab, event}`.
//!
//! L'agent n'a ni fichiers ni terminal : il n'agit que par les outils.

mod config;
mod error;
mod events;
mod link;
mod permission;
mod settings_port;
mod stack;
mod tab_agent;
mod tab_entry;
mod turn;
mod turns;

pub use config::{Launch, READY_WITHIN};
pub use error::AgentError;
pub use events::{AgentEvent, AgentState, PlanItem};
pub use link::Link;
pub use settings_port::AppSettings;
pub use stack::{AgentCommand, Stack};
pub use tab_agent::{AgentHandle, ToolsAccess};
