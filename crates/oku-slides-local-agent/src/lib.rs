//! Agent local de repli : un agent ACP intégré à l'app, sans compte ni réseau.
//!
//! - [`exit_if_local_agent`] : à appeler en tête de `main`, après `exit_if_worker`.
//!
//! Le binaire de l'app, relancé avec `--local-agent`, sert ACP sur stdin/stdout ;
//! il n'agit que par les outils MCP de l'onglet, routés par familles (`families`).

mod acp;
#[cfg_attr(
    all(not(test), not(all(target_os = "macos", target_arch = "aarch64"))),
    expect(dead_code, reason = "lu par le moteur Gguf, sur Apple silicon seulement")
)]
mod chatml;
mod compact;
mod engine;
mod entry;
mod error;
#[cfg(debug_assertions)]
mod fake;
mod families;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod gguf;
mod mcp;
mod prompt;
#[cfg_attr(
    all(not(test), not(all(target_os = "macos", target_arch = "aarch64"))),
    expect(dead_code, reason = "lu par le moteur Gguf, sur Apple silicon seulement")
)]
mod reply;
mod sessions;
mod tool_error;
mod turn;

pub use entry::exit_if_local_agent;
