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
pub(crate) struct FocusArgs {
    /// Id of the slide to show.
    pub slide: String,
    /// Id of an element of that slide to show; absent: the slide alone.
    #[serde(default)]
    pub element: Option<String>,
}

impl Tools {
    /// A request that changes the session; `Changed` if it succeeds.
    pub(crate) async fn change(&self, req: Request) -> CallToolResult {
        let done = self.with_session(move |s| s.handle(req).map(|_| ())).await;
        let ok = done.is_ok();
        let result = self.reply(done, |()| {
            self.hub.emit(McpEvent::Changed);
            self.json_result(&json!({ "ok": true }))
        });
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
        let label = label.unwrap_or_else(|| self.localizer.current().tr("agent-undo-apply"));
        self.change(Request::Apply { label, ops, merge: None }).await
    }

    pub(crate) async fn chart_kind(&self, args: Value) -> CallToolResult {
        let ChartKindArgs { slide, element, kind } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let label = self.localizer.current().tr("agent-undo-apply");
        self.change(Request::ChartKind { label, slide, element, kind }).await
    }

    pub(crate) async fn show_as(&self, args: Value) -> CallToolResult {
        let ShowAsArgs { slide, element, view } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let label = self.localizer.current().tr("agent-undo-apply");
        self.change(Request::ShowAs { label, slide, element, view }).await
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
        let session = self.session.clone();
        let done = match tokio::task::spawn_blocking(move || session.with(f).and_then(|r| r)).await {
            Ok(done) => done.map_err(|e| PackChangeSnafu.into_error(e)),
            Err(e) => Err(crate::tool_error::TaskSnafu.into_error(e)),
        };
        let ok = done.is_ok();
        let result = self.reply(done, |()| {
            self.hub.emit(McpEvent::Changed);
            self.json_result(&json!({ "ok": true }))
        });
        if ok {
            self.prune_surfaces().await;
        }
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

    pub(crate) fn focus(&self, args: Value) -> CallToolResult {
        match self.parse::<FocusArgs>(args) {
            Ok(FocusArgs { slide, element }) => {
                self.hub.emit(McpEvent::Focus { slide, element });
                self.json_result(&json!({ "ok": true }))
            }
            Err(r) => r,
        }
    }
}
