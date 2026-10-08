//! Outils qui modifient le deck ; tout s'annule (dans le groupe du tour s'il est ouvert).

use oku_slides_chart::SeriesType;
use oku_slides_core::Op;
use oku_slides_session::Request;
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::McpEvent;
use crate::tools::{Tools, fail, json_result, parse};

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ApplyArgs {
    /// Ops du deck, appliquées ensemble (tout ou rien). Voir le schéma de chaque op.
    pub ops: Vec<Op>,
    /// Libellé d'annulation hors d'un tour (dans un tour, celui du tour).
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ShowAsArgs {
    pub slide: String,
    pub element: String,
    /// `table` ou `chart` ; les données sont gardées.
    pub view: oku_slides_session::View,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ChartKindArgs {
    pub slide: String,
    pub element: String,
    /// Nouveau type ; les données sont converties.
    pub kind: SeriesType,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetArgs {
    /// Id d'un pack de modèles (UUID, voir `templates`).
    #[schemars(with = "String")]
    pub set: uuid::Uuid,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct FocusArgs {
    pub slide: String,
    #[serde(default)]
    pub element: Option<String>,
}

impl Tools {
    /// Une requête qui change la session ; `Changed` si elle aboutit.
    async fn change(&self, req: Request) -> CallToolResult {
        match self.with_session(move |s| s.handle(req).map(|_| ()).map_err(|e| e.one_line())).await {
            Ok(()) => {
                self.hub.emit(McpEvent::Changed);
                json_result(&json!({ "ok": true }))
            }
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn apply(&self, args: Value) -> CallToolResult {
        let ApplyArgs { ops, label } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let label = label.unwrap_or_else(|| self.localizer.tr("agent-undo-apply"));
        self.change(Request::Apply { label, ops, merge: None }).await
    }

    pub(crate) async fn chart_kind(&self, args: Value) -> CallToolResult {
        let ChartKindArgs { slide, element, kind } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let label = self.localizer.tr("agent-undo-apply");
        self.change(Request::ChartKind { label, slide, element, kind }).await
    }

    pub(crate) async fn show_as(&self, args: Value) -> CallToolResult {
        let ShowAsArgs { slide, element, view } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let label = self.localizer.tr("agent-undo-apply");
        self.change(Request::ShowAs { label, slide, element, view }).await
    }

    pub(crate) async fn edit_templates(&self, args: Value) -> CallToolResult {
        match parse::<SetArgs>(args) {
            Ok(SetArgs { set }) => self.change(Request::EditTemplates { set }).await,
            Err(r) => r,
        }
    }

    pub(crate) async fn close_templates(&self) -> CallToolResult {
        self.change(Request::CloseTemplates).await
    }

    pub(crate) fn focus(&self, args: Value) -> CallToolResult {
        match parse::<FocusArgs>(args) {
            Ok(FocusArgs { slide, element }) => {
                self.hub.emit(McpEvent::Focus { slide, element });
                json_result(&json!({ "ok": true }))
            }
            Err(r) => r,
        }
    }
}
