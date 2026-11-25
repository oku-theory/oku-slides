//! Tools that change the deck; everything undoes (in the turn's group when one is open).

use oku_slides_chart::SeriesType;
use oku_slides_core::{Namespace, Op, PackKind};
use oku_slides_session::Request;
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::{IntoError, ResultExt};

use crate::McpEvent;
use crate::tool_error::{PackChangeSnafu, PackIdSnafu, ToolError};
use crate::tools::Tools;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ApplyArgs {
    /// Deck ops, applied together (all or nothing); see each op's schema.
    pub ops: Vec<Op>,
    /// Undo label outside a turn; within a turn, the turn's label is used.
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ShowAsArgs {
    /// Id of the slide that holds the element.
    pub slide: String,
    /// Id of the chart or table element.
    pub element: String,
    /// `table` or `chart`; the data is kept.
    pub view: oku_slides_session::View,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ChartKindArgs {
    /// Id of the slide that holds the chart.
    pub slide: String,
    /// Id of the chart element.
    pub element: String,
    /// New series type; the data is converted.
    pub kind: SeriesType,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PackIdArgs {
    /// Id of a pack (UUID, see `packs`).
    pub pack: String,
}

impl PackIdArgs {
    /// The pack's id; a text that is not a UUID is refused by a message
    /// that points the agent to `packs`.
    pub(crate) fn id(&self) -> Result<uuid::Uuid, ToolError> {
        uuid::Uuid::parse_str(&self.pack).context(PackIdSnafu)
    }
}

/// `add_pack` and `remove_pack`: the targeted pack.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PackRefArgs {
    /// `widgets` (elements) or `layouts` (slides).
    pub kind: PackKind,
    /// Namespace of the pack (see `packs` and `deck_outline`).
    pub namespace: Namespace,
}

/// `set_theme_extends`: the parents of the edited theme.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ThemeExtendsArgs {
    /// Namespaces of library themes, in order; the later ones win.
    pub extends: Vec<Namespace>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct FocusArgs {
    /// Id of the slide to show; absent: deselect, the shown slide stays. The agent never selects.
    #[serde(default)]
    pub slide: Option<String>,
}

/// The notice of a session snapshot, its other fields left unread.
#[derive(Deserialize)]
struct SnapshotNotice {
    notice: Option<String>,
}

impl Tools {
    /// A request of the agent that changes the session (traced in its turn); `Changed` if it succeeds.
    ///
    /// The snapshot's notice (inline data lifted with a warning, say) joins the reply: the
    /// snapshot sent it once, to the agent, and the user sees it in the agent's answer. It may
    /// quote the user's data, so the reply says `untrusted`.
    pub(crate) async fn change(&self, req: Request) -> CallToolResult {
        self.change_told(req, false).await
    }

    /// [`Tools::change`]; an error goes in the untrusted wrapper when `untrusted_error`.
    async fn change_told(&self, req: Request, untrusted_error: bool) -> CallToolResult {
        let done = self.with_folding(move |s| s.handle_by_agent(req.clone())).await;
        let ok = done.is_ok();
        let result = match done {
            Ok(snapshot) => {
                self.hub.emit(McpEvent::Changed);
                match serde_json::from_str::<SnapshotNotice>(&snapshot) {
                    Ok(SnapshotNotice { notice: Some(notice) }) => {
                        self.json_result(&json!({ "ok": true, "untrusted": true, "notice": notice }))
                    }
                    Ok(SnapshotNotice { notice: None }) => self.json_result(&json!({ "ok": true })),
                    // The change is done; only its notice is lost, and the log says so.
                    Err(error) => {
                        tracing::error!(%error, "snapshot unreadable: notice not sent to the agent");
                        self.json_result(&json!({ "ok": true }))
                    }
                }
            }
            Err(e) if untrusted_error => self.fail_untrusted(&e),
            Err(e) => self.fail_with(&e),
        };
        if ok {
            self.prune_surfaces().await;
        }
        result
    }

    /// Prunes and refreshes the hub's surfaces after a change of the deck.
    ///
    /// `SurfaceHub::prune` locks the session: it runs off the async thread.
    pub(crate) async fn prune_surfaces(&self) {
        let hub = self.hub.clone();
        if let Err(e) = tokio::task::spawn_blocking(move || hub.prune()).await {
            tracing::warn!(error = %e, "surfaces not pruned");
        }
    }

    pub(crate) async fn apply(&self, args: Value) -> CallToolResult {
        let ApplyArgs { ops, label } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let label = label.unwrap_or_else(|| {
            let l = self.localizer.current();
            oku_slides_session::data_undo_label(&l, &ops).unwrap_or_else(|| l.tr("agent-undo-apply"))
        });
        // Any op may name a data or a view's data, whose refusal quotes the user's data
        // (a data's or a column's name, a view's id).
        self.change_told(Request::Apply { label, ops, merge: None }, true).await
    }

    pub(crate) async fn chart_kind(&self, args: Value) -> CallToolResult {
        let ChartKindArgs { slide, element, kind } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let label = self.localizer.current().tr("agent-undo-apply");
        // Its refusal may name the view's data or columns.
        self.change_told(Request::ChartKind { label, slide, element, kind }, true).await
    }

    pub(crate) async fn show_as(&self, args: Value) -> CallToolResult {
        let ShowAsArgs { slide, element, view } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let label = self.localizer.current().tr("agent-undo-apply");
        // Its refusal may name the view's data or columns.
        self.change_told(Request::ShowAs { label, slide, element, view }, true).await
    }

    /// Adds a library pack's current version to the deck (deck mode).
    pub(crate) async fn add_pack(&self, args: Value) -> CallToolResult {
        let PackRefArgs { kind, namespace } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        self.change_packs(move |s| s.add_library_pack(kind, &namespace)).await
    }

    /// Removes a pack from the deck (deck mode); a refusal says why.
    pub(crate) async fn remove_pack(&self, args: Value) -> CallToolResult {
        let PackRefArgs { kind, namespace } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        self.change_packs(move |s| s.remove_deck_pack(kind, &namespace)).await
    }

    /// A change of the deck's packs; `Changed` and a prune if it succeeds.
    ///
    /// A core refusal is told by the refused op itself (`PackChange`), without the transaction's rank.
    async fn change_packs(
        &self,
        f: impl FnOnce(&mut oku_slides_session::Session) -> oku_slides_session::Result<()> + Send + 'static,
    ) -> CallToolResult {
        match self.on_packs(f).await {
            Ok(()) => self.packs_changed().await,
            Err(e) => self.fail_with(&e),
        }
    }

    /// Runs `f` on the session off the async threads; a session refusal is a `PackChange`.
    pub(crate) async fn on_packs<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut oku_slides_session::Session) -> oku_slides_session::Result<T> + Send + 'static,
    ) -> Result<T, crate::tool_error::ToolError> {
        let session = self.session.clone();
        match tokio::task::spawn_blocking(move || session.with(f).and_then(|r| r)).await {
            Ok(done) => done.map_err(|e| PackChangeSnafu.into_error(e)),
            Err(e) => Err(crate::tool_error::TaskSnafu.into_error(e)),
        }
    }

    /// After a change of the deck's packs: `Changed`, then a prune (t-1133); answers `{ok: true}`.
    pub(crate) async fn packs_changed(&self) -> CallToolResult {
        self.hub.emit(McpEvent::Changed);
        let result = self.json_result(&json!({ "ok": true }));
        self.prune_surfaces().await;
        result
    }

    /// The edited theme's parents (theme tab), in one undo step.
    pub(crate) async fn set_theme_extends(&self, args: Value) -> CallToolResult {
        let ThemeExtendsArgs { extends } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        self.change(Request::SetEditedThemeExtends { extends }).await
    }

    /// Shows a slide, never selecting (the user's selection stays); without one, deselects on the shown slide.
    pub(crate) fn focus(&self, args: Value) -> CallToolResult {
        match self.parse::<FocusArgs>(args) {
            Ok(FocusArgs { slide }) => {
                self.hub.emit(McpEvent::Focus { slide });
                self.json_result(&json!({ "ok": true }))
            }
            Err(r) => r,
        }
    }
}
