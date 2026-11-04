//! Tools of a pack or theme tab's own document: selection, preview theme, rename, copies.

use oku_slides_core::GuideKind;
use oku_slides_session::Request;
use oku_slides_session::agent::bounds::{LABEL_MAX, check_text, label};
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::IntoError;

use crate::McpEvent;
use crate::tool_error::SessionSnafu;
use crate::tools::Tools;

/// `select_item`: the item to select.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SelectItemArgs {
    /// A widget or layout of this pack, as `widget_sources` lists it, `_commun` included.
    pub name: String,
}

/// `set_preview_theme`: the theme the pack tab previews with.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetPreviewThemeArgs {
    /// Namespace, id or name of a library theme (see `deck_outline`).
    pub theme: String,
}

/// `rename_theme`: the theme tab's new name.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct RenameThemeArgs {
    /// The new name; empty takes back the base theme's name.
    pub name: String,
}

/// `set_guide`: one of the edited pack's or theme's guides for agents.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetGuideArgs {
    /// `edit`: how to edit it; `use`: how a deck uses it.
    pub which: GuideKind,
    /// The guide as plain Markdown; empty clears it.
    pub text: String,
}

impl Tools {
    /// `select_item`: selects an item of the pack tab, as a click in the pile does.
    pub(crate) async fn select_item(&self, args: Value) -> CallToolResult {
        let SelectItemArgs { name } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let done = self.with_session(move |s| s.select_item(&name)).await;
        self.reply(done, |()| {
            self.hub.emit(McpEvent::Changed);
            self.json_result(&json!({ "ok": true }))
        })
    }

    /// `set_preview_theme`: the theme the pack tab previews its items with.
    pub(crate) async fn set_preview_theme(&self, args: Value) -> CallToolResult {
        let SetPreviewThemeArgs { theme } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        self.change(Request::SetPreviewTheme { theme }).await
    }

    /// `rename_theme`: renames the theme this tab edits, in one undo step.
    pub(crate) async fn rename_theme(&self, args: Value) -> CallToolResult {
        let RenameThemeArgs { name } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        if let Err(e) = check_text("name", &name, LABEL_MAX) {
            return self.fail_with(&SessionSnafu.into_error(e));
        }
        self.change(Request::RenameEditedTheme { name }).await
    }

    /// `set_guide`: writes the edited pack's or theme's guide for agents, in one undo step.
    ///
    /// It goes through `set_guide_by_agent`, so the user-write note is not recorded and the
    /// guide is not echoed back to the agent. Answers the stored text's length and the cap.
    /// Within a turn, the write is noted for the turn's receipt.
    pub(crate) async fn set_guide(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let SetGuideArgs { which, text } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        if let Some(refused) = self.continues_elsewhere(turn) {
            return refused;
        }
        let chars = text.chars().count();
        let done = self
            .with_session(move |s| {
                let fact = s.set_guide_by_agent(which, text)?.then(|| s.guide_write_fact(which, chars));
                Ok((s.guide_limit(), fact))
            })
            .await;
        self.reply(done, |(max_chars, fact)| {
            if let (Some(turn), Some(fact)) = (turn, fact) {
                self.hub.note_guide_write(turn, which, fact);
            }
            self.hub.emit(McpEvent::Changed);
            self.json_result(&json!({ "chars": chars, "max_chars": max_chars }))
        })
    }

    /// `save_theme`: saves a copy of the edited theme under a new id, once the user confirms.
    pub(crate) async fn save_theme(&self) -> CallToolResult {
        let name = match self.with_session(|s| s.edited_theme_name()).await {
            Ok(name) => name,
            Err(e) => return self.fail_with(&e),
        };
        let message =
            self.localizer.current().tr_with("agent-confirm-copy-edited-theme", &[("name", label(&name).into())]);
        self.guarded(message, None, Request::SaveTheme { as_new: true }).await
    }
}
