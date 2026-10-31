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
//!   dictation and agent settings box (`voice_port`).
//!
//! The agent has no files and no terminal: it acts only through the tools.

mod agent_switch;
mod choice;
mod command_error;
mod config;
mod drafts_port;
mod error;
mod events;
mod instructions;
mod keeper;
mod link;
mod model_download;
mod paste;
mod permission;
mod prompt_images;
mod session_model;
mod settings_port;
mod shortcut;
mod stack;
mod stt_watch;
mod tab_agent;
mod tab_entry;
mod table;
mod turn;
mod turns;
mod usage_board;
mod voice_port;
mod voice_refresh;
mod wire;

pub use command_error::CommandError;
pub use config::{Launch, READY_WITHIN};
pub use error::AgentError;
pub use events::{AgentEvent, AgentState, PlanItem};
pub use keeper::{Keep, Keeper};
pub use link::{Link, agent_fingerprint};
pub use settings_port::AppSettings;
pub use stack::{AgentCommand, MAX_DRAFT_BYTES, MAX_ORIGIN_BYTES, Stack};
pub use tab_agent::{AgentHandle, ToolsAccess};
