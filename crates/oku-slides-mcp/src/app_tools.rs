//! Outils de l'app hors du deck : historique, présentation et interface.
//!
//! Le menu principal ne garde que Présenter, Ouvrir et Aide : le reste se demande à l'agent
//! (t-988). Présenter, l'interface et l'ouverture de l'historique vivent dans la page : l'outil
//! envoie un évènement que l'onglet applique par le même chemin que ses propres commandes
//! (saisie en cours validée, enregistrement en attente parti d'abord). Seul l'onglet affiché
//! de sa fenêtre les reçoit : un autre est refusé.

use oku_slides_session::Request;
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use snafu::{OptionExt, ResultExt, ensure};

use crate::history_page::history_page;
use crate::tool_error::{
    EmptyInterfaceSnafu, HistoryIndexSnafu, HistoryNotOpenedSnafu, LayoutMinSnafu, LayoutRangeSnafu,
    NotPresentingSnafu, NotShownSnafu, PresentSlideSnafu, ResponseSnafu, TaskSnafu, ToolError, UnknownLocaleSnafu,
};
use crate::tools::{Tools, text};
use crate::{McpEvent, Nav};

/// Délai laissé à l'onglet pour ouvrir l'historique (saisie validée, enregistrement parti).
const HISTORY_OPEN_WAIT: std::time::Duration = std::time::Duration::from_secs(5);
/// Pas de la relecture de la session pendant cette attente.
const HISTORY_OPEN_POLL: std::time::Duration = std::time::Duration::from_millis(50);

/// Where the presentation shows (contract of `McpEvent::Present`, mirrored in `ui/src/api.ts`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PresentMode {
    /// The whole screen.
    #[default]
    Screen,
    /// The whole window.
    Window,
}

/// Color scheme of the interface (contract of `McpEvent::Interface`).
///
/// The schemes the workspace can force (`light`, `dark`), plus `system` (no choice).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SchemeChoice {
    /// Light.
    Light,
    /// Dark.
    Dark,
    /// The system's scheme.
    System,
}

/// Narrowest pile the page allows, in px (`fixtures/ui/layout-bounds.json`, `ui/src/columns.ts`).
pub(crate) const PILE_MIN: u32 = 180;
/// Widest pile the page allows, in px; a narrow window lowers it.
pub(crate) const PILE_MAX: u32 = 360;
/// Narrowest agent thread the page allows, in px; its widest depends on the window.
pub(crate) const NOTES_MIN: u32 = 300;

/// What the agent changes in the interface; absent: unchanged.
///
/// Contract of `McpEvent::Interface`, mirrored in `ui/src/api.ts`: an absent field goes out as `null`.
/// The layout fields keep the `UiState` names; the page clamps widths to the window's live bounds.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InterfaceChange {
    /// Color scheme.
    #[serde(default)]
    pub scheme: Option<SchemeChoice>,
    /// Language: a shipped code (`en`, `fr`…) or `system`.
    #[serde(default)]
    pub locale: Option<String>,
    /// Guides of the empty fields.
    #[serde(default)]
    pub guides: Option<bool>,
    /// Sections of the pile to fold (`true`) or open (`false`).
    #[serde(default)]
    pub folded: Option<FoldedChange>,
    /// Width of the pile, in px, from 180 to 360.
    #[serde(default)]
    pub pile: Option<u32>,
    /// Width of the agent thread, in px, at least 300.
    #[serde(default)]
    pub notes: Option<u32>,
    /// A pack item's Code tab open (`true`) rather than its Settings tab.
    #[serde(default)]
    pub code: Option<bool>,
}

/// The pile sections to fold or open; absent: unchanged.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FoldedChange {
    /// The slide thumbnails.
    #[serde(default)]
    pub slides: Option<bool>,
    /// The layout thumbnails of a theme tab.
    #[serde(default)]
    pub layouts: Option<bool>,
}

impl InterfaceChange {
    /// No change asked: no field, or a folded object without a section.
    fn is_empty(&self) -> bool {
        let folded = self.folded.is_none_or(|f| f.slides.is_none() && f.layouts.is_none());
        self.scheme.is_none()
            && self.locale.is_none()
            && self.guides.is_none()
            && folded
            && self.pile.is_none()
            && self.notes.is_none()
            && self.code.is_none()
    }

    /// Refuses a width outside the static bounds; the page narrows it further to the window.
    fn check_bounds(&self) -> Result<(), ToolError> {
        if let Some(pile) = self.pile {
            ensure!(
                (PILE_MIN..=PILE_MAX).contains(&pile),
                LayoutRangeSnafu { field: "pile", min: PILE_MIN, max: PILE_MAX }
            );
        }
        if let Some(notes) = self.notes {
            ensure!(notes >= NOTES_MIN, LayoutMinSnafu { field: "notes", min: NOTES_MIN });
        }
        Ok(())
    }
}

/// What the page's presentation does (contract of `McpEvent::Present`, mirrored in `ui/src/present.ts`).
///
/// `Start` presents from the shown slide; the others steer a running presentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PresentCommand {
    /// Start presenting from the shown slide.
    Start {
        /// Screen or window.
        mode: PresentMode,
    },
    /// Show this slide.
    Show {
        /// Id of a slide of the deck.
        slide: String,
    },
    /// Go to the next slide.
    Next,
    /// Go to the previous slide.
    Previous,
    /// Go to the first slide.
    First,
    /// Go to the last slide.
    Last,
    /// Leave the presentation.
    Exit,
}

impl From<Nav> for PresentCommand {
    fn from(nav: Nav) -> Self {
        match nav {
            Nav::Next => Self::Next,
            Nav::Previous => Self::Previous,
            Nav::First => Self::First,
            Nav::Last => Self::Last,
            Nav::Exit => Self::Exit,
        }
    }
}

/// What the `present` tool does.
#[derive(Debug, Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PresentAction {
    /// Starts presenting from the shown slide (default).
    #[default]
    Start,
    /// Shows the slide `slide`.
    Show,
    /// Goes to the next slide.
    Next,
    /// Goes to the previous slide.
    Previous,
    /// Goes to the first slide.
    First,
    /// Goes to the last slide.
    Last,
    /// Leaves the presentation.
    Exit,
}

/// Arguments of `present`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PresentArgs {
    /// `start` (default), or a step of the running presentation.
    #[serde(default)]
    pub action: PresentAction,
    /// For `start`: `screen` (default) or `window`.
    #[serde(default)]
    pub mode: PresentMode,
    /// For `show`: id of the slide to show.
    #[serde(default)]
    pub slide: Option<String>,
}

/// What the `history` tool does.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HistoryAction {
    /// Opens the history in the tab, on the latest version.
    Open,
    /// Lists a page of the versions, oldest to newest, with the history open.
    List,
    /// Shows the version `index`.
    Seek,
    /// Brings the deck back to the shown version, and closes the history.
    Restore,
    /// Closes the history, deck unchanged.
    Close,
}

/// Arguments of `history`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct HistoryArgs {
    /// What the tool does.
    pub action: HistoryAction,
    /// For `seek`: rank of the version (0: the oldest).
    #[serde(default)]
    pub index: Option<usize>,
    /// For `open` and `list`: the page ends before this rank (default: after the latest version).
    #[serde(default)]
    pub before: Option<usize>,
    /// For `open` and `list`: versions in the page (default 20, at most 100).
    #[serde(default)]
    pub count: Option<usize>,
}

impl Tools {
    /// Refuses a tab not shown in its window: it does not apply page events.
    pub(crate) async fn ensure_shown(&self) -> Result<(), ToolError> {
        let tab = self.session.clone();
        let shown = tokio::task::spawn_blocking(move || tab.shown()).await.context(TaskSnafu)?;
        ensure!(shown, NotShownSnafu);
        Ok(())
    }

    /// Ouvre (par l'onglet), liste, parcourt, restaure ou ferme l'historique.
    pub(crate) async fn history(&self, args: Value) -> CallToolResult {
        let HistoryArgs { action, index, before, count } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let req = match action {
            HistoryAction::Open => return self.history_open(before, count).await,
            HistoryAction::List => return self.history_list(before, count).await,
            HistoryAction::Seek => match index.context(HistoryIndexSnafu) {
                Ok(index) => Request::HistorySeek { index },
                Err(e) => return self.fail_with(&e),
            },
            HistoryAction::Restore => Request::HistoryRestore,
            HistoryAction::Close => Request::HistoryClose,
        };
        // Après `seek`, la version montrée (le rang demandé est borné à la dernière).
        let done = self
            .with_session(move |s| {
                s.handle_by_agent(req)?;
                Ok(s.snapshot().history.map(|h| json!({ "index": h.index, "count": h.count })))
            })
            .await;
        self.reply(done, |shown| {
            self.hub.emit(McpEvent::Changed);
            self.json_result(&json!({ "ok": true, "shown": shown }))
        })
    }

    /// Demande à l'onglet d'ouvrir l'historique (sa saisie validée et son enregistrement parti
    /// d'abord, comme par ses propres commandes), attend qu'il soit ouvert et rend les versions.
    /// Refusé là où l'historique ne s'ouvre pas ; pas ouvert à temps : dit à l'agent.
    async fn history_open(&self, before: Option<usize>, count: Option<usize>) -> CallToolResult {
        if let Err(e) = self.ensure_shown().await {
            return self.fail_with(&e);
        }
        if let Err(e) = self.with_session(|s| s.ensure_deck_mode()).await {
            return self.fail_with(&e);
        }
        self.hub.emit(McpEvent::OpenHistory);
        let deadline = tokio::time::Instant::now() + HISTORY_OPEN_WAIT;
        loop {
            match self.with_session(|s| Ok(s.snapshot().history.is_some())).await {
                Ok(true) => return self.history_list(before, count).await,
                Ok(false) if tokio::time::Instant::now() < deadline => tokio::time::sleep(HISTORY_OPEN_POLL).await,
                Ok(false) => return self.fail_with(&HistoryNotOpenedSnafu.build()),
                Err(e) => return self.fail_with(&e),
            }
        }
    }

    /// A page of the versions (`history_page`), with the history open.
    ///
    /// Serialized under the session lock: only the page is copied, not the whole list.
    async fn history_list(&self, before: Option<usize>, count: Option<usize>) -> CallToolResult {
        let page = self
            .with_session(move |s| Ok(serde_json::to_string(&history_page(s.history_versions()?, before, count))))
            .await;
        self.reply(page, |page| match page.context(ResponseSnafu) {
            Ok(page) => text(page),
            Err(e) => self.fail_with(&e),
        })
    }

    /// Starts or steers the presentation in the tab.
    ///
    /// `start` is refused where `Session::can_present` refuses; every other action while the
    /// tab's page does not report a presentation (`NotPresenting`), `show` for a slide the deck
    /// does not have.
    pub(crate) async fn present(&self, args: Value) -> CallToolResult {
        let PresentArgs { action, mode, slide } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        if let Err(e) = self.ensure_shown().await {
            return self.fail_with(&e);
        }
        let command = match self.present_command(action, mode, slide).await {
            Ok(command) => command,
            Err(e) => return self.fail_with(&e),
        };
        self.hub.emit(McpEvent::Present { command });
        self.json_result(&json!({ "ok": true }))
    }

    /// The command `action` sends, checked against the session and the hub's presentation.
    async fn present_command(
        &self,
        action: PresentAction,
        mode: PresentMode,
        slide: Option<String>,
    ) -> Result<PresentCommand, ToolError> {
        match action {
            PresentAction::Start => {
                // The session's rule, as the page's own Present: each refusal says why.
                self.with_session(|s| s.can_present()).await?;
                Ok(PresentCommand::Start { mode })
            }
            PresentAction::Show => {
                self.steer(())?;
                let slide = slide.context(PresentSlideSnafu)?;
                let id = slide.clone();
                let known = self.with_session(move |s| Ok(s.deck().slide(&id).is_some())).await?;
                ensure!(known, PresentSlideSnafu);
                Ok(PresentCommand::Show { slide })
            }
            PresentAction::Next => self.steer(PresentCommand::Next),
            PresentAction::Previous => self.steer(PresentCommand::Previous),
            PresentAction::First => self.steer(PresentCommand::First),
            PresentAction::Last => self.steer(PresentCommand::Last),
            PresentAction::Exit => self.steer(PresentCommand::Exit),
        }
    }

    /// `then` while the tab's page reports a presentation, else `NotPresenting`.
    fn steer<T>(&self, then: T) -> Result<T, ToolError> {
        ensure!(self.hub.presenting().is_some(), NotPresentingSnafu);
        Ok(then)
    }

    /// Changes the interface's scheme, language, guides or layout (folded sections, widths, code).
    pub(crate) async fn set_interface(&self, args: Value) -> CallToolResult {
        let mut change: InterfaceChange = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        if change.is_empty() {
            return self.fail_with(&EmptyInterfaceSnafu.build());
        }
        if let Err(e) = change.check_bounds() {
            return self.fail_with(&e);
        }
        // L'interface ne reçoit qu'un code livré : `FR-ca` part en `fr`.
        if let Some(locale) = change.locale.as_deref() {
            change.locale = match oku_slides_i18n::shipped(locale) {
                _ if locale.eq_ignore_ascii_case("system") => Some("system".to_owned()),
                Some(code) => Some(code.to_owned()),
                None => return self.fail_with(&UnknownLocaleSnafu.build()),
            };
        }
        if let Err(e) = self.ensure_shown().await {
            return self.fail_with(&e);
        }
        self.hub.emit(McpEvent::Interface { change });
        self.json_result(&json!({ "ok": true }))
    }
}

#[cfg(test)]
#[path = "app_tools_tests.rs"]
mod app_tools_tests;
