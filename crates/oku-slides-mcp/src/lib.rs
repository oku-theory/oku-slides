//! Outils MCP d'Oku Slides pour l'agent, servis en HTTP streamable sur
//! `127.0.0.1` avec un jeton par lancement.
//!
//! - [`ToolServer`] : démarre le serveur (port choisi par le système).
//! - [`Tools`] : les outils (lecture, `apply`, surfaces, confirmations).
//! - [`SurfaceHub`] : surfaces ouvertes, réponses attendues, évènements
//!   vers l'interface ([`McpEvent`]).
//! - [`SettingsPort`] : réglages lus et écrits, fournis par l'hôte.
//!
//! Dépend de `oku-slides-session` ; utilisé par `oku-slides-agent`.

mod edit_tools;
mod error;
mod guarded_tools;
mod handler;
mod hub;
mod import_tools;
mod paths;
mod read_tools;
mod server;
mod settings;
mod surface_tools;
mod tool_error;
mod tools;
mod widget_tools;

pub use error::McpError;
pub use hub::{
    Answer, Freeze, HubError, HubErrorKind, McpEvent, STATIC_PER_TURN, SurfaceAction, SurfaceHub, THREAD_BYTES,
};
pub use server::ToolServer;
pub use settings::{Applied, SettingsPort};
pub use tools::{Tools, tool_names};
