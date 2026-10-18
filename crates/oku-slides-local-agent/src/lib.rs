//! Agent local de repli : un agent ACP intégré à l'app, sans compte ni réseau.
//!
//! - [`exit_if_local_agent`] : à appeler en tête de `main`, après `exit_if_worker`.
//!
//! Le binaire de l'app, relancé avec `--local-agent`, sert ACP sur stdin/stdout ;
//! il n'agit que par les outils MCP de l'onglet, routés par familles (`families`).

mod acp;
mod compact;
mod engine;
mod entry;
mod error;
#[cfg(debug_assertions)]
mod fake;
mod families;
mod mcp;
mod prompt;
mod sessions;
mod turn;

pub use entry::exit_if_local_agent;
