//! What the session offers the agent: slide images, undo turns, surfaces
//! (forms, variants), context and a tab's session (`Sessions`, `TabSession`,
//! `TabGate`, `TabOpen`). The MCP tools (`oku-slides-mcp`) and the ACP
//! client (`oku-slides-agent`) go through here.
//! Pasted images: `attachments` (`AttachmentRef`).

mod address;
pub mod attachments;
pub mod bounds;
pub mod context;
pub mod conversation;
pub mod diff;
pub mod forms;
pub mod reason;
pub mod receipt;
pub mod render;
mod surface_ops;
mod tabs;
pub(crate) mod turn_undo;
pub mod turn_view;
mod usage_panel;
pub(crate) mod usage_table;
pub mod variants;

pub use attachments::{AttachmentRef, PasteReason};
pub use bounds::{DETAIL_CHARS, LABEL_MAX, NOTE_CHARS, PROMPT_MAX, TEXT_MAX, WHY_MAX, check_options, clip};
pub use context::AgentUi;
pub use conversation::SavedConversation;
pub use forms::{
    Anchor, AskChoice, AskConfirm, AskRewrite, FieldKind, ParamField, PermissionChoice, PermissionKind, Place,
    ShowParams, ShowVariants, Surface, Variant,
};
pub use oku_slides_a2ui::{PromptImage, ServerMessage, StepStatus, TimelineStep, ToolCallDetail};
pub use reason::AgentFormReason;
pub use receipt::{NextStep, ReceiptSurfaces, Report};
pub use render::Render;
pub use tabs::{Sessions, TabGate, TabOpen, TabSession};
pub use turn_view::{TurnState, TurnView};
pub use usage_panel::usage_panel;

/// Tours gardés au plus par fil (réducteur des tours, hub, reprise).
pub const KEPT_TURNS: usize = 50;

use snafu::OptionExt;

use crate::error::{AgentFormSnafu, NoSlideSnafu};
use crate::{Result, Session};

/// Pixels par point des images pour l'agent : 960 × 540 pour une slide.
const AGENT_PIXEL_PER_PT: f32 = 0.5;

impl Session {
    /// PNG image of slide `id`, so that the agent sees the rendering.
    ///
    /// # Errors
    ///
    /// Fails (`NoSlide`) for an unknown slide, or if rendering the page fails.
    pub fn png(&mut self, id: &str) -> Result<Vec<u8>> {
        let page = self.page_of(id).context(NoSlideSnafu { id })?;
        self.with_compiled(|c| c.png(page - 1, AGENT_PIXEL_PER_PT))
    }
}

/// Rejects a number of entries outside `min..=max` (`AgentForm`).
///
/// # Errors
///
/// Fails (`AgentForm`, `Count`) if `len` falls outside `min..=max`.
pub fn check_len(what: &str, len: usize, (min, max): (usize, usize)) -> Result<()> {
    snafu::ensure!(
        (min..=max).contains(&len),
        AgentFormSnafu { reason: AgentFormReason::Count { at: what.to_owned(), min, max, count: len } }
    );
    Ok(())
}
