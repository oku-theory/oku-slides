//! The agent's tabs and windows: `tabs` lists its window's tabs; `tab` activates, reorders,
//! moves or closes one; `window` opens, closes, detaches or attaches the agent; `help` opens the
//! help tab, searched; `new_conversation` and `quit` run once the turn has ended.
//!
//! Everything goes through the host's [`AgentWorkspace`] in a blocking task, scoped to the
//! requester's window. Activating, reordering and moving a tab is the turn's one tab move: the
//! claim is reserved before any workspace lock and given back when nothing moved. A close is
//! confirmed (`Close`, or `Discard` naming a tab with unsaved work), stored as a
//! [`CloseDecision`] holding each covered tab's dirty revision, then announced to the page,
//! which closes through its own flows; the agent's own tab (a window close included) only once
//! the turn has ended (`AfterTurn::CloseOwn`). Tab titles shown to the agent are cleaned and
//! clipped (`bounds::label`): the user's data, never instructions.

use std::sync::Arc;

use oku_slides_session::DocumentKind;
use oku_slides_session::agent::bounds::{clip, label};
use oku_slides_session::agent::{AgentWorkspace, CloseScope, CloseTabs, HelpOpen, TabOpen, WorkspaceRefusal};
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::{IntoError, ResultExt};

use crate::hub::TabTarget;
use crate::tab_gate::TabVerdict;
use crate::tool_error::{
    AfterTurnTakenSnafu, AppSnafu, LastWindowSnafu, NoTurnSnafu, QuitUnsavedSnafu, TabIndexSnafu, TabNoTurnSnafu,
    TabUnusedSnafu, TaskSnafu, ToolError, WorkspaceSnafu,
};
use crate::tools::Tools;
use crate::{AfterTurn, CloseDecision, CloseTarget, ConfirmAsk, ConfirmVerb, McpEvent};

/// `tab`: what to do with one tab of this tab's window.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TabArgs {
    /// What to do with the tab.
    pub action: TabAction,
    /// The tab's id, `t-<n>`, as `tabs` lists it.
    pub tab: String,
    /// reorder: its new position in the bar, 0 first; move: its position in the window it lands in (absent: last).
    #[serde(default)]
    pub index: Option<usize>,
    /// move: `new` for a new window, else a window label `w-<n>`; absent: a new window.
    #[serde(default)]
    pub window: Option<String>,
}

/// What `tab` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TabAction {
    /// Make it its window's active tab.
    Activate,
    /// Close it, after the user confirms.
    Close,
    /// Move it to `index` in its bar.
    Reorder,
    /// Move it to another window.
    Move,
}

/// `window`: what to do with this tab's window or the agent's.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct WindowArgs {
    /// What to do.
    pub action: WindowAction,
}

/// What `window` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WindowAction {
    /// Open a new window holding a blank tab.
    New,
    /// Close this tab's window, after the user confirms, once this reply ends.
    Close,
    /// Move the agent into its own window.
    DetachAgent,
    /// Bring the agent back into the main window.
    AttachAgent,
}

/// `help`: what to look for in the help.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct HelpArgs {
    /// Words to search the help for; absent: the help opens where it was.
    #[serde(default)]
    pub query: Option<String>,
}

/// The longest help query searched, in characters: longer ones are clipped.
const QUERY_MAX: usize = 200;

/// What a tab move did: whether the turn's one move is spent, and the agent's reply.
type Moved = (bool, Value);

impl Tools {
    /// `tabs`: this tab's window's tabs, in bar order, titles cleaned and clipped.
    pub(crate) async fn tabs(&self) -> CallToolResult {
        let listed = self.on_workspace(|ws, own| ws.tabs(own)).await;
        self.reply(listed, |tabs| {
            let tabs: Vec<Value> = tabs
                .into_iter()
                .map(|t| json!({ "tab": t.tab, "title": label(&t.title), "active": t.active, "dirty": t.dirty, "kind": t.kind }))
                .collect();
            self.json_result(&json!({ "tabs": tabs }))
        })
    }

    /// `tab`: activates, reorders, moves or closes one tab of this tab's window.
    pub(crate) async fn tab(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let args: TabArgs = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let Some(turn) = turn else { return self.fail_with(&TabNoTurnSnafu.build()) };
        if args.action == TabAction::Close {
            if args.index.is_some() || args.window.is_some() {
                return self.fail_with(&TabUnusedSnafu.build());
            }
            return self.close_tab(turn, args.tab).await;
        }
        let Some(number) = tab_number(&args.tab) else {
            return self.fail_with(&WorkspaceSnafu.into_error(WorkspaceRefusal::unknown_tab()));
        };
        let change = match change_of(args.action, args.index, args.window.as_deref()) {
            Ok(change) => change,
            Err(e) => return self.fail_with(&e),
        };
        // The claim comes before any workspace lock: the workspace never asks the hub.
        let claim = TabTarget::Tab(number);
        let mut verdict = TabVerdict::default();
        if !verdict.ask(&self.hub, Some(turn), claim) {
            return self.fail_with(&verdict.refusal());
        }
        let target = args.tab;
        let moved: Result<Moved, ToolError> = self
            .on_workspace(move |ws, own| match change {
                Change::Activate => ws.activate(own, &target, &mut |_| true).map(|opened| activated(own, opened)),
                Change::Reorder { index } => ws.reorder(own, &target, index).map(|()| (true, json!({ "ok": true }))),
                Change::Move { window, index } => {
                    ws.move_tab(own, &target, window.as_deref(), index).map(|()| (true, json!({ "ok": true })))
                }
            })
            .await;
        // A lost task may have moved the tab: its claim stays spent (see `TabVerdict::settle_moved`).
        let spent = matches!(moved, Ok((true, _)) | Err(ToolError::Task { .. }));
        verdict.settle_moved(&self.hub, Some(turn), claim, spent);
        self.reply(moved, |(_, reply)| self.json_result(&reply))
    }

    /// `tab {close}`: confirmed, then announced; the agent's own tab once the turn ended.
    ///
    /// Only from the shown tab: its page alone receives the close event.
    async fn close_tab(&self, turn: u32, target: String) -> CallToolResult {
        if let Err(e) = self.ensure_shown().await {
            return self.fail_with(&e);
        }
        let own = target == self.session.tab();
        if own && self.hub.ends_app_after(turn) {
            return self.fail_with(&AfterTurnTakenSnafu.build());
        }
        let scope_tab = target.clone();
        let covered = match self.on_workspace(move |ws, own| ws.dirty_revisions(own, CloseScope::Tab(&scope_tab))).await
        {
            Ok(c) => c,
            Err(e) => return self.fail_with(&e),
        };
        let Some(tab) = covered.tabs.first() else {
            return self.fail_with(&WorkspaceSnafu.into_error(WorkspaceRefusal::unknown_tab()));
        };
        let (key, verb) = if tab.dirty {
            ("agent-confirm-discard-tab", ConfirmVerb::Discard)
        } else {
            ("agent-confirm-close-tab", ConfirmVerb::Close)
        };
        let message = self.localizer.current().tr_with(key, &[("tab", label(&tab.title).into())]);
        if !self.hub.confirm(message, ConfirmAsk::Danger(verb)).await {
            return self.denied();
        }
        let target = CloseTarget::Tab { tab: target };
        self.announce_close(turn, target, &covered, own)
    }

    /// `window`: a new window, the agent's window, or this tab's window closed.
    pub(crate) async fn window(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let WindowArgs { action } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let Some(turn) = turn else { return self.fail_with(&TabNoTurnSnafu.build()) };
        let done = match action {
            WindowAction::Close => return self.close_window(turn).await,
            WindowAction::New => self.on_workspace(|ws, own| ws.new_window(own)).await,
            WindowAction::DetachAgent => self.on_workspace(|ws, _| ws.detach_agent(false)).await,
            WindowAction::AttachAgent => self.on_workspace(|ws, _| ws.detach_agent(true)).await,
        };
        self.reply(done, |()| self.json_result(&json!({ "ok": true })))
    }

    /// `window {close}`: this tab's window, never the last one, confirmed, once the turn ended.
    async fn close_window(&self, turn: u32) -> CallToolResult {
        if let Err(e) = self.ensure_shown().await {
            return self.fail_with(&e);
        }
        if self.hub.ends_app_after(turn) {
            return self.fail_with(&AfterTurnTakenSnafu.build());
        }
        let covered = match self.on_workspace(|ws, own| ws.dirty_revisions(own, CloseScope::Window)).await {
            Ok(c) => c,
            Err(e) => return self.fail_with(&e),
        };
        if covered.last_window {
            return self.fail_with(&LastWindowSnafu.build());
        }
        let dirty: Vec<String> = covered.tabs.iter().filter(|t| t.dirty).map(|t| label(&t.title)).collect();
        let l = self.localizer.current();
        let (message, verb) = if dirty.is_empty() {
            (l.tr("agent-confirm-close-window"), ConfirmVerb::Close)
        } else {
            (l.tr_with("agent-confirm-discard-window", &[("tabs", dirty.join(", ").into())]), ConfirmVerb::Discard)
        };
        if !self.hub.confirm(message, ConfirmAsk::Danger(verb)).await {
            return self.denied();
        }
        let target = CloseTarget::Window { window: covered.window.clone() };
        self.announce_close(turn, target, &covered, true)
    }

    /// Stores the confirmed close of `covered`, then announces it now, or once the turn ended (`own`).
    fn announce_close(&self, turn: u32, target: CloseTarget, covered: &CloseTabs, own: bool) -> CallToolResult {
        let decision = CloseDecision {
            discard: covered.tabs.iter().any(|t| t.dirty),
            revisions: covered.tabs.iter().map(|t| (t.tab.clone(), t.revision)).collect(),
        };
        let id = self.hub.store_close(target, &covered.window, decision);
        if !own {
            self.hub.send_close(id);
            return self.json_result(&json!({ "closing": "now" }));
        }
        if let Err(e) = self.record_after_turn(turn, AfterTurn::CloseOwn { decision: id }) {
            self.hub.drop_close(id);
            return self.fail_with(&e);
        }
        self.json_result(&json!({ "closing": "after_turn" }))
    }

    /// `help`: the help tab opened or shown, at its first hit for `query`; the turn's one tab move.
    ///
    /// Called from the help tab itself, it moves nothing and claims no move: its page shows the
    /// first hit. A reused help tab's pages are sent that hit too, live in another window or not.
    pub(crate) async fn help(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let HelpArgs { query } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let Some(turn) = turn else { return self.fail_with(&TabNoTurnSnafu.build()) };
        let query = query.map(|q| clip(q.trim(), QUERY_MAX)).filter(|q| !q.is_empty());
        // The help tab's own call pages it: no tab moves, so no move is claimed.
        let moves = self.kind != DocumentKind::Help;
        let mut verdict = TabVerdict::default();
        if moves && !verdict.ask(&self.hub, Some(turn), TabTarget::Help) {
            return self.fail_with(&verdict.refusal());
        }
        let opened = self.on_workspace(move |ws, own| ws.open_help(own, query.as_deref(), &mut |_| true)).await;
        // A lost task may have opened the tab: its claim stays spent (see `TabVerdict::settle_moved`).
        let spent = matches!(&opened, Ok(o) if matches!(o.open, TabOpen::Opened { .. } | TabOpen::Focused { .. }))
            || matches!(opened, Err(ToolError::Task { .. }));
        if moves {
            verdict.settle_moved(&self.hub, Some(turn), TabTarget::Help, spent);
        }
        let HelpOpen { open, hits } = match opened {
            Ok(o) => o,
            Err(e) => return self.fail_with(&e),
        };
        let first = hits.first().map(|h| h.slide.clone());
        let hits: Vec<Value> =
            hits.into_iter().map(|h| json!({ "slide": h.slide, "title": h.title.as_deref().map(label) })).collect();
        let reply = match open {
            TabOpen::Opened { tab, shown } => {
                self.hub.emit(McpEvent::Changed);
                json!({ "ok": true, "tab": tab, "shown": shown, "hits": hits })
            }
            TabOpen::Focused { tab, shown } => {
                self.hub.emit(McpEvent::Changed);
                // A page mounted on the help tab keeps its slide, and saves it back: send it the hit.
                let Some(slide) = first else {
                    return self.json_result(&json!({ "ok": true, "tab": tab, "shown": shown, "hits": hits }));
                };
                let (tabs, target) = (self.tabs.clone(), tab.clone());
                let paged = tokio::task::spawn_blocking(move || tabs.page(&target, slide)).await;
                let paged = paged.unwrap_or_else(|e| {
                    tracing::warn!(error = %e, "help: paging the help tab failed");
                    false
                });
                // Not paged: no page shows the tab, which keeps its saved slide.
                json!({ "ok": true, "tab": tab, "shown": shown, "hits": hits, "paged": paged })
            }
            TabOpen::Here => {
                if first.is_some() {
                    self.hub.emit(McpEvent::Focus { slide: first });
                }
                json!({ "ok": true, "tab": self.session.tab(), "shown": true, "hits": hits })
            }
            TabOpen::Refused => json!({ "ok": false }),
        };
        self.json_result(&reply)
    }

    /// `new_conversation`: the conversation starts over once this turn has ended.
    pub(crate) fn new_conversation(&self, turn: Option<u32>) -> CallToolResult {
        let Some(turn) = turn else { return self.fail_with(&NoTurnSnafu { tool: "new_conversation" }.build()) };
        match self.record_after_turn(turn, AfterTurn::NewConversation) {
            Ok(()) => self.json_result(&json!({ "new_conversation": "after_turn" })),
            Err(e) => self.fail_with(&e),
        }
    }

    /// `quit`: confirmed, then the app quits once the turn has ended (checked again then).
    ///
    /// Refused first, without asking: where the host cannot quit, once the turn ends the app
    /// another way, and while tabs hold unsaved work, told by their count, never their titles.
    pub(crate) async fn quit(&self, turn: Option<u32>) -> CallToolResult {
        let Some(turn) = turn else { return self.fail_with(&NoTurnSnafu { tool: "quit" }.build()) };
        if let Err(e) = self.app.can_quit() {
            return self.fail_with(&AppSnafu.into_error(e));
        }
        if self.hub.ends_app_after(turn) {
            return self.fail_with(&AfterTurnTakenSnafu.build());
        }
        let workspace = Arc::clone(&self.workspace);
        let unsaved = match tokio::task::spawn_blocking(move || workspace.unsaved_tabs()).await {
            Ok(n) => n,
            Err(e) => return self.fail_with(&TaskSnafu.into_error(e)),
        };
        if unsaved > 0 {
            return self.fail_with(&QuitUnsavedSnafu { count: unsaved }.build());
        }
        let message = self.localizer.current().tr("agent-confirm-quit");
        if !self.hub.confirm(message, ConfirmAsk::Danger(ConfirmVerb::Quit)).await {
            return self.denied();
        }
        match self.record_after_turn(turn, AfterTurn::Quit) {
            Ok(()) => self.json_result(&json!({ "quit": "after_turn" })),
            Err(e) => self.fail_with(&e),
        }
    }

    /// Runs `f` on the host's workspace with this tab's id, in a blocking task.
    async fn on_workspace<T: Send + 'static>(
        &self,
        f: impl FnOnce(&dyn AgentWorkspace, &str) -> Result<T, WorkspaceRefusal> + Send + 'static,
    ) -> Result<T, ToolError> {
        let (workspace, own) = (Arc::clone(&self.workspace), self.session.tab().to_owned());
        tokio::task::spawn_blocking(move || f(workspace.as_ref(), &own))
            .await
            .context(TaskSnafu)?
            .context(WorkspaceSnafu)
    }
}

/// A tab change once its arguments are checked: each action with only what it uses.
#[derive(Debug)]
enum Change {
    /// Show the tab.
    Activate,
    /// Put it at `index` in its bar.
    Reorder { index: usize },
    /// Send it to `window` (`None`: a new one), at `index` there.
    Move { window: Option<String>, index: Option<usize> },
}

/// The change `action` names; an argument it does not use is refused, as a missing index.
fn change_of(action: TabAction, index: Option<usize>, window: Option<&str>) -> Result<Change, ToolError> {
    match (action, index, window) {
        (TabAction::Activate, None, None) => Ok(Change::Activate),
        (TabAction::Reorder, Some(index), None) => Ok(Change::Reorder { index }),
        (TabAction::Reorder, None, None) => TabIndexSnafu.fail(),
        (TabAction::Move, index, None | Some("new")) => Ok(Change::Move { window: None, index }),
        (TabAction::Move, index, Some(label)) if oku_slides_session::is_window_label(label) => {
            Ok(Change::Move { window: Some(label.to_owned()), index })
        }
        (TabAction::Move, _, Some(_)) => Err(WorkspaceSnafu.into_error(WorkspaceRefusal::unknown_window())),
        _ => TabUnusedSnafu.fail(),
    }
}

/// The number of a tab id `t-<n>`; `None` for anything else.
fn tab_number(tab: &str) -> Option<u64> {
    let digits = tab.strip_prefix("t-")?;
    if digits.is_empty() || digits.len() > 18 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// The reply to an activation: a move unless `own` itself was named (`Here`).
fn activated(own: &str, opened: TabOpen) -> Moved {
    match opened {
        TabOpen::Focused { tab, shown } | TabOpen::Opened { tab, shown } => {
            (true, json!({ "ok": true, "tab": tab, "shown": shown }))
        }
        TabOpen::Here => (false, json!({ "ok": true, "tab": own, "shown": true })),
        TabOpen::Refused => (false, json!({ "ok": false })),
    }
}

#[cfg(test)]
#[path = "workspace_tools_tests.rs"]
mod workspace_tools_tests;
