//! Voice agent: Oku Slides runs an ACP agent as a subprocess (stdio).
//!
//! It injects its MCP tools into the agent (local HTTP, token) and relays the
//! dictated instructions to it with the interface's context.
//!
//! - [`Link`]: the shared ACP connection (one subprocess for the app,
//!   started on the first prompt, restarted after it stops).
//! - [`AgentHandle`]: a tab's agent (one ACP session per tab, its turns).
//! - [`Keeper`]: a tab's keeper, the only way to its kept conversation.
//! - [`AgentEvent`]: what the interface shows in the thread.
//! - [`Stack`]: one agent per tab (MCP tools, kept thread) for the host;
//!   events relayed in `{tab, event}` envelopes, the agent choice (`choice`) and the
//!   dictation and agent settings box (`voice_port`); the host voice router (`voice_router`)
//!   routes each hands-free utterance before any agent launch.
//!
//! The agent has no files and no terminal: it acts only through the tools.

mod after_turn;
mod agent_command;
mod agent_switch;
mod choice;
mod command_error;
mod config;
mod drafts_port;
mod error;
mod events;
#[cfg(test)]
mod hand_voice;
mod instructions;
mod keeper;
mod link;
mod model_download;
mod page_reports;
mod paste;
mod permission;
mod prompt_images;
mod reader;
mod session_model;
mod settings_port;
mod shortcut;
mod stack;
mod stack_reports;
mod stderr_tail;
mod stt_watch;
mod tab_agent;
mod tab_entry;
mod table;
mod tune_port;
mod turn;
mod turns;
mod usage_board;
mod voice_port;
mod voice_refresh;
mod voice_router;
mod wire;

pub use agent_command::{AgentCommand, MAX_DRAFT_BYTES, MAX_ORIGIN_BYTES};
pub use command_error::CommandError;
pub use config::{Launch, READY_WITHIN};
pub use error::AgentError;
pub use events::{AgentEvent, AgentState, PlanItem};
pub use keeper::{Keep, Keeper};
pub use link::{Link, agent_fingerprint};
/// The agent's close decision a page redeems (`Stack::take_close_decision`).
pub use oku_slides_mcp::CloseDecision;
pub use page_reports::ReportFrom;
pub use settings_port::AppSettings;
pub use stack::{Stack, Voices};
pub use tab_agent::{AgentHandle, ToolsAccess};
pub use voice_router::RouterEnd;
