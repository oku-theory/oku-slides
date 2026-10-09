//! Ce que la session offre à l'agent : images des slides, tours d'annulation,
//! surfaces (formes, variantes), contexte et la session d'un onglet
//! (`Sessions`, `TabSession`). Les outils MCP (`oku-slides-mcp`) et le
//! client ACP (`oku-slides-agent`) passent par ici.

mod address;
pub mod bounds;
pub mod context;
pub mod conversation;
pub mod diff;
pub mod forms;
pub mod reason;
pub mod receipt;
pub mod render;
pub mod scope;
mod surface_ops;
mod tabs;
pub mod turn_view;
pub mod variants;

pub use bounds::{LABEL_MAX, NOTE_CHARS, PROMPT_MAX, TEXT_MAX, WHY_MAX, check_options};
pub use context::AgentUi;
pub use conversation::{SavedConversation, ScopeMove};
pub use forms::{
    Anchor, AskChoice, AskConfirm, AskRewrite, FieldKind, ParamField, PermissionChoice, PermissionKind, Place,
    ShowParams, ShowVariants, Surface, Variant,
};
pub use oku_slides_a2ui::{ServerMessage, StepStatus, TimelineStep};
pub use reason::AgentFormReason;
pub use receipt::{NextStep, ReceiptSurfaces, Report};
pub use render::Render;
pub use scope::AgentScope;
pub use tabs::{Sessions, TabSession};
pub use turn_view::{TurnState, TurnView};

/// Tours gardés au plus par fil (réducteur des tours, hub, reprise).
pub const KEPT_TURNS: usize = 50;

use snafu::OptionExt;

use crate::error::{AgentFormSnafu, NoSlideSnafu};
use crate::{Result, Session};

/// Pixels par point des images pour l'agent : 960 × 540 pour une slide.
const AGENT_PIXEL_PER_PT: f32 = 0.5;

impl Session {
    /// Image PNG de la slide `id`, pour que l'agent voie le rendu.
    pub fn png(&mut self, id: &str) -> Result<Vec<u8>> {
        let page = self.page_of(id).context(NoSlideSnafu { id })?;
        self.with_compiled(|c| c.png(page - 1, AGENT_PIXEL_PER_PT))
    }
}

/// Refuse un nombre d'entrées hors de `min..=max` (`AgentForm`).
pub fn check_len(what: &str, len: usize, (min, max): (usize, usize)) -> Result<()> {
    snafu::ensure!(
        (min..=max).contains(&len),
        AgentFormSnafu { reason: AgentFormReason::Count { at: what.to_owned(), min, max, count: len } }
    );
    Ok(())
}
