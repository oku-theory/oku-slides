//! Agent vocal : Oku Slides lance un agent ACP en sous-processus (stdio),
//! lui injecte ses outils MCP (HTTP local, jeton) et lui relaie les
//! consignes dictées avec le contexte de l'interface.
//!
//! - [`Link`] : la connexion ACP partagée (un sous-processus pour l'app,
//!   lancé au premier prompt, relancé après un arrêt).
//! - [`AgentHandle`] : l'agent d'un onglet (une session ACP par portée, ses tours).
//! - [`Keeper`] : le gardien d'un onglet, seul passage vers ses conversations gardées.
//! - [`AgentEvent`] : ce que l'interface affiche dans le fil.
//! - [`Stack`] : un agent par onglet (outils MCP, fil gardé) pour l'hôte ;
//!   évènements relayés en enveloppes `{tab, event}`, et le panneau des agents
//!   (`picker`, `select_agent`).
//!
//! L'agent n'a ni fichiers ni terminal : il n'agit que par les outils.

mod command_error;
mod config;
mod error;
mod events;
mod instructions;
mod keeper;
mod link;
mod permission;
mod picker;
mod settings_port;
mod stack;
mod tab_agent;
mod tab_entry;
mod turn;
mod turns;

pub use command_error::CommandError;
pub use config::{Launch, READY_WITHIN};
pub use error::AgentError;
pub use events::{AgentEvent, AgentState, PlanItem};
pub use keeper::{Keep, Keeper};
pub use link::{Link, agent_fingerprint};
pub use settings_port::AppSettings;
pub use stack::{AgentCommand, MAX_DRAFT_BYTES, MAX_ORIGIN_BYTES, Stack};
pub use tab_agent::{AgentHandle, ToolsAccess};
