//! Outils des widgets du pack : statut, source, écriture, suppression et
//! aperçu. Disponibles en mode deck comme en mode modèles.

use oku_slides_session::{PreviewFormat, Request};
use rust_mcp_sdk::schema::{CallToolResult, TextContent};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::McpEvent;
use crate::tools::{Tools, fail, json_result, parse, png, text};

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct WidgetName {
    /// Nom du fichier : `[a-z][a-z0-9-]*` (le nom du widget), ou `_commun` pour le code partagé.
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetWidgetArgs {
    /// Nom du fichier : `[a-z][a-z0-9-]*` (le nom du widget), ou `_commun`.
    pub name: String,
    /// Source Typst complète du fichier ; vide : le fichier est supprimé.
    pub source: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PreviewWidgetArgs {
    /// Nom du widget à montrer.
    pub name: String,
    /// Valeurs de paramètres ; les autres prennent leur défaut ou un exemple.
    #[serde(default)]
    pub params: Map<String, Value>,
}

impl Tools {
    /// Une requête de widgets, sa réponse JSON telle quelle ; `Changed` si `changes`.
    async fn widget_request(&self, req: Request, changes: bool) -> CallToolResult {
        match self.with_session(move |s| s.handle(req).map_err(|e| e.one_line())).await {
            Ok(json) => {
                if changes {
                    self.hub.emit(McpEvent::Changed);
                }
                text(json)
            }
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn widget_sources(&self) -> CallToolResult {
        self.widget_request(Request::WidgetStatus, false).await
    }

    pub(crate) async fn widget_source(&self, args: Value) -> CallToolResult {
        match parse::<WidgetName>(args) {
            Ok(WidgetName { name }) => {
                match self.with_session(move |s| s.widget_source(&name).map_err(|e| e.one_line())).await {
                    Ok(source) => text(source),
                    Err(e) => fail(e),
                }
            }
            Err(r) => r,
        }
    }

    pub(crate) async fn set_widget(&self, args: Value) -> CallToolResult {
        match parse::<SetWidgetArgs>(args) {
            Ok(SetWidgetArgs { name, source }) => self.widget_request(Request::SetWidget { name, source }, true).await,
            Err(r) => r,
        }
    }

    pub(crate) async fn remove_widget(&self, args: Value) -> CallToolResult {
        match parse::<WidgetName>(args) {
            Ok(WidgetName { name }) => self.widget_request(Request::RemoveWidget { name }, true).await,
            Err(r) => r,
        }
    }

    /// L'image PNG du widget, suivie de ses diagnostics s'il en a.
    pub(crate) async fn preview_widget(&self, args: Value) -> CallToolResult {
        let PreviewWidgetArgs { name, params } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let preview = self
            .with_session(move |s| s.preview_widget(&name, &params, PreviewFormat::Png).map_err(|e| e.one_line()))
            .await;
        let preview = match preview {
            Ok(p) => p,
            Err(e) => return fail(e),
        };
        let Some(bytes) = &preview.png else { return json_result(&preview.diagnostics) };
        let mut result = png(bytes);
        if !preview.diagnostics.is_empty() {
            match serde_json::to_string(&preview.diagnostics) {
                Ok(json) => result.content.push(TextContent::new(json, None, None).into()),
                Err(e) => return fail(format!("response not serializable: {e}")),
            }
        }
        result
    }
}
