//! The interface's commands to a tab's agent, as Tauri and the dev server deserialize them.

use oku_slides_mcp::PresentMode;
use oku_slides_session::agent::AgentUi;

/// Largest draft accepted, in UTF-8 bytes.
pub const MAX_DRAFT_BYTES: usize = 256 * 1024;
/// Largest draft origin (a view id) accepted, in bytes.
pub const MAX_ORIGIN_BYTES: usize = 64;

/// A command from the interface to a tab's agent.
///
/// JSON `{"cmd": …, "tab": …}`, the same for Tauri and HTTP.
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum AgentCommand {
    /// An instruction for the tab's agent.
    ///
    /// With pasted images (`images`), it is never taken for an answer to a question nor for a
    /// shortcut word.
    Prompt {
        /// The target tab.
        tab: String,
        /// The instruction, as spoken or typed.
        text: String,
        /// What the interface shows with the instruction: slide, selected element and field.
        #[serde(default)]
        ui: AgentUi,
        /// The question this instruction answers: the one receiving keys 1–4
        /// (`quickTarget`). Absent, or a question no longer waiting: an ordinary instruction.
        #[serde(default)]
        answer: Option<String>,
        /// The pasted images sent with the instruction (draft ids `sha256:…`).
        #[serde(default)]
        images: Vec<String>,
    },
    /// Fine-tunes the imported tab: a turn in the tab's conversation, starting the agent.
    ///
    /// The thread shows the localized text `agent-tune-prompt`, in the interface's current
    /// language; this text is also the last block of the prompt, after the hidden import
    /// instructions. Reserved to the host ([`Stack::tune`], when the import asks for
    /// fine-tuning: `WorkspaceOutcome::Tune`): never read from the interface channel.
    #[serde(skip_deserializing)]
    Tune {
        /// The imported tab.
        tab: String,
    },
    /// Interrupts the tab's running turn.
    Cancel {
        /// The target tab.
        tab: String,
    },
    /// Starts a new conversation in the tab: the thread starts over.
    NewConversation {
        /// The target tab.
        tab: String,
    },
    /// The user acts on a surface (button, choice, form).
    SurfaceAction {
        /// The tab showing the surface.
        tab: String,
        /// Id of the surface.
        surface: String,
        /// Name of the action.
        name: String,
        /// Form values at the time of the action.
        #[serde(default)]
        context: serde_json::Map<String, serde_json::Value>,
    },
    /// The (re)subscribed interface asks for the tab's kept thread and its open surfaces.
    Resync {
        /// The target tab.
        tab: String,
    },
    /// Another view of the tab (the agent window) subscribes.
    ///
    /// Replays the state, the draft, then the tab's thread; never starts the tab.
    Follow {
        /// The followed tab.
        tab: String,
    },
    /// The tab's draft; `None`: closed. Refused if the tab has no started agent.
    SetDraft {
        /// The target tab.
        tab: String,
        /// The draft's text; `None` closes it.
        text: Option<String>,
        /// The view writing it, returned with the echo.
        origin: Option<String>,
    },
    /// A pasted image (`data`: its bytes in base64), added to the tab's draft.
    ///
    /// Refused without a started agent, beyond `[paste] max_draft_images`, or by the session.
    PasteImage {
        /// The target tab.
        tab: String,
        /// The image bytes, in base64.
        data: String,
    },
    /// Removes image `id` from the draft.
    RemoveImage {
        /// The target tab.
        tab: String,
        /// Id of the image to remove.
        id: String,
    },
    /// The interface opens the deck's packs panel: the surface goes to this tab.
    ShowPacks {
        /// The target tab.
        tab: String,
    },
    /// The interface opens the theme picker on the slide shown: the surface goes to this tab.
    ShowThemes {
        /// The target tab.
        tab: String,
        /// The slide each theme shows; `None`: the first one.
        slide: Option<String>,
    },
    /// The interface opens the dictation and agent settings box.
    ShowSettings {
        /// The target tab.
        tab: String,
    },
    /// The interface asks whether the agent is usable (`availability` event to this tab).
    Availability {
        /// The target tab.
        tab: String,
    },
    /// The tab's page reports what it shows; kept by the stack, never sent to the agent.
    ///
    /// Ordered by `(page, seq)`: an older number is ignored, a new page id starts over.
    /// Accepted only from the window that holds the tab ([`ReportFrom::Holder`]).
    Context {
        /// The target tab.
        tab: String,
        /// The reporting page's id, new at each mount.
        page: String,
        /// What the page shows: the context of the next voice prompt.
        #[serde(default)]
        ui: AgentUi,
        /// The report's number in its page, below `u64::MAX`.
        seq: u64,
        /// The page has the focus (`document.hasFocus()`).
        #[serde(default)]
        has_focus: bool,
    },
    /// The tab's page reports whether it presents; kept by the stack, never sent to the agent.
    ///
    /// Ordered and checked like [`AgentCommand::Context`].
    Presenting {
        /// The target tab.
        tab: String,
        /// The reporting page's id, new at each mount.
        page: String,
        /// Where it presents; absent: it does not present.
        #[serde(default)]
        mode: Option<PresentMode>,
        /// The report's number in its page, below `u64::MAX`.
        seq: u64,
    },
}

impl AgentCommand {
    /// The target tab.
    #[must_use]
    pub fn tab(&self) -> &str {
        match self {
            Self::Prompt { tab, .. }
            | Self::Tune { tab, .. }
            | Self::Cancel { tab }
            | Self::NewConversation { tab }
            | Self::SurfaceAction { tab, .. }
            | Self::Resync { tab }
            | Self::Follow { tab }
            | Self::SetDraft { tab, .. }
            | Self::PasteImage { tab, .. }
            | Self::RemoveImage { tab, .. }
            | Self::ShowPacks { tab }
            | Self::ShowThemes { tab, .. }
            | Self::ShowSettings { tab }
            | Self::Availability { tab }
            | Self::Context { tab, .. }
            | Self::Presenting { tab, .. } => tab,
        }
    }

    /// A page report, which only the window holding its tab may send.
    #[must_use]
    pub fn is_page_report(&self) -> bool {
        matches!(self, Self::Context { .. } | Self::Presenting { .. })
    }
}
